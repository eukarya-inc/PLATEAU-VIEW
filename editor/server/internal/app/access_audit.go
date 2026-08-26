package app

import (
	"encoding/json"
	"strings"
	"time"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearth/server/internal/adapter"
	"github.com/reearth/reearthx/log"
)

// accessAuditLogPrefix is a stable, greppable prefix on every audit line.
// Downstream (Cloud Logging → BigQuery) filters on this to pick audit events
// out of the general application log stream.
const accessAuditLogPrefix = "editor_access_audit "

// accessAuditRecord is the JSON payload emitted for each authenticated
// Editor API request. Kept small on purpose: no headers, no bodies, no
// Authorization values. Downstream schema in BigQuery is:
//
//	SAFE.PARSE_JSON(REGEXP_EXTRACT(jsonPayload.message,
//	  r'^editor_access_audit (\{.*\})$'))
type accessAuditRecord struct {
	Ts         string `json:"ts"`
	Sub        string `json:"sub,omitempty"`
	UserID     string `json:"user_id,omitempty"`
	Email      string `json:"email,omitempty"`
	Name       string `json:"name,omitempty"`
	Method     string `json:"method"`
	Path       string `json:"path"`
	Status     int    `json:"status"`
	LatencyMS  int64  `json:"latency_ms"`
	RemoteIP   string `json:"remote_ip,omitempty"`
	UserAgent  string `json:"ua,omitempty"`
	RequestID  string `json:"request_id,omitempty"`
	Referer    string `json:"referer,omitempty"`
	AuthMethod string `json:"auth,omitempty"` // "jwt" | "mock" | "debug"
}

// accessAuditSkipPrefixes is the list of URL path prefixes that are excluded
// from audit logging. These are either unauthenticated (published data), noisy
// (health checks, static assets), or already covered by other logs.
var accessAuditSkipPrefixes = []string{
	"/api/ping",
	"/api/published/",
	"/api/published_data/",
	"/p/",
	"/assets/",
	"/static/",
	"/favicon",
	"/debug/pprof",
	"/health",
	"/robots.txt",
}

// accessAuditMiddleware records one structured log line per authenticated
// Editor API request. It runs AFTER attachOpMiddleware so that
// adapter.User(ctx) is already resolved from the JWT sub.
//
// Design notes:
//   - Unauthenticated requests (no user resolved) are skipped: aggregating
//     them adds no value for the "who used the Editor, when" question and
//     would only bloat logs.
//   - We never log the Authorization header or the raw JWT.
//   - Email fallback order: DB user → JWT AuthInfo → empty.
//   - Latency is measured around the downstream handler only.
func accessAuditMiddleware(enabled bool) echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			if !enabled {
				return next(c)
			}

			path := c.Request().URL.Path
			for _, p := range accessAuditSkipPrefixes {
				if strings.HasPrefix(path, p) {
					return next(c)
				}
			}

			start := time.Now()
			err := next(c)
			latency := time.Since(start)

			ctx := c.Request().Context()

			// Only log authenticated requests. If neither the JWT nor the
			// resolved user is present, there is nothing to attribute.
			u := adapter.User(ctx)
			au := adapter.GetAuthInfo(ctx)
			if u == nil && au == nil {
				return err
			}

			rec := accessAuditRecord{
				Ts:        start.UTC().Format(time.RFC3339Nano),
				Method:    c.Request().Method,
				Path:      path,
				Status:    c.Response().Status,
				LatencyMS: latency.Milliseconds(),
				RemoteIP:  c.RealIP(),
				UserAgent: c.Request().UserAgent(),
				RequestID: c.Response().Header().Get(echo.HeaderXRequestID),
				Referer:   c.Request().Referer(),
			}
			if rec.RequestID == "" {
				rec.RequestID = c.Request().Header.Get(echo.HeaderXRequestID)
			}

			if au != nil {
				rec.Sub = au.Sub
				if au.Email != "" {
					rec.Email = au.Email
				}
				if au.Name != "" {
					rec.Name = au.Name
				}
				rec.AuthMethod = "jwt"
			}
			if u != nil {
				rec.UserID = u.ID().String()
				if e := u.Email(); e != "" {
					rec.Email = e
				}
				if n := u.Name(); n != "" {
					rec.Name = n
				}
			}
			if adapter.IsMockAuth(ctx) {
				rec.AuthMethod = "mock"
			}

			buf, jerr := json.Marshal(rec)
			if jerr != nil {
				// Never let audit logging break a request. Just drop the line.
				return err
			}
			log.Infofc(ctx, "%s%s", accessAuditLogPrefix, string(buf))

			return err
		}
	}
}
