package app

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearth/server/internal/adapter"
	"github.com/reearth/reearthx/account/accountdomain/user"
	"github.com/reearth/reearthx/appx"
	"github.com/reearth/reearthx/log"
	"github.com/stretchr/testify/assert"
)

// captureLogs redirects the reearthx global logger to an in-memory buffer so
// we can inspect emitted audit lines. Restored on t.Cleanup.
func captureLogs(t *testing.T) *bytes.Buffer {
	t.Helper()
	// disable color codes so the plain substring match works reliably
	_ = os.Setenv("NO_COLOR", "1")
	buf := &bytes.Buffer{}
	log.SetOutput(buf)
	t.Cleanup(func() {
		log.SetOutput(os.Stdout)
	})
	return buf
}

func newAuditRequest(t *testing.T, method, path string, u *user.User, au *appx.AuthInfo) echo.Context {
	t.Helper()
	e := echo.New()
	req := httptest.NewRequest(method, path, nil)
	req.Header.Set(echo.HeaderXRequestID, "req-1")
	req.Header.Set("User-Agent", "test-agent")
	rec := httptest.NewRecorder()
	c := e.NewContext(req, rec)

	ctx := req.Context()
	if u != nil {
		ctx = adapter.AttachUser(ctx, u)
	}
	if au != nil {
		ctx = context.WithValue(ctx, adapter.ContextAuthInfo, *au)
	}
	c.SetRequest(req.WithContext(ctx))
	return c
}

// extractAuditRecord finds the last audit line in the captured log output and
// parses its JSON payload. Returns false when no audit line was emitted.
func extractAuditRecord(t *testing.T, out string) (accessAuditRecord, bool) {
	t.Helper()
	idx := strings.LastIndex(out, accessAuditLogPrefix)
	if idx < 0 {
		return accessAuditRecord{}, false
	}
	rest := out[idx+len(accessAuditLogPrefix):]
	// The payload ends at the first newline.
	if nl := strings.IndexByte(rest, '\n'); nl >= 0 {
		rest = rest[:nl]
	}
	// The console encoder may color-wrap; strip trailing ANSI reset if any.
	rest = strings.TrimSpace(rest)
	// Trim to the last '}' to survive any trailing suffix the encoder adds.
	if end := strings.LastIndexByte(rest, '}'); end >= 0 {
		rest = rest[:end+1]
	}
	var rec accessAuditRecord
	if err := json.Unmarshal([]byte(rest), &rec); err != nil {
		t.Fatalf("failed to parse audit payload %q: %v (raw=%q)", rest, err, out)
	}
	return rec, true
}

func TestAccessAuditMiddleware_Disabled(t *testing.T) {
	buf := captureLogs(t)
	mw := accessAuditMiddleware(false)

	u := user.New().NewID().Name("Alice").Email("alice@city.osaka.lg.jp").MustBuild()
	c := newAuditRequest(t, http.MethodPost, "/api/graphql", u, &appx.AuthInfo{Sub: "auth0|abc"})

	err := mw(func(c echo.Context) error {
		return c.NoContent(http.StatusOK)
	})(c)
	assert.NoError(t, err)

	assert.NotContains(t, buf.String(), accessAuditLogPrefix, "should emit no audit line when disabled")
}

func TestAccessAuditMiddleware_SkipsUnauthenticated(t *testing.T) {
	buf := captureLogs(t)
	mw := accessAuditMiddleware(true)

	c := newAuditRequest(t, http.MethodGet, "/api/graphql", nil, nil)
	err := mw(func(c echo.Context) error {
		return c.NoContent(http.StatusOK)
	})(c)
	assert.NoError(t, err)

	assert.NotContains(t, buf.String(), accessAuditLogPrefix, "should not audit unauthenticated requests")
}

func TestAccessAuditMiddleware_SkipsExcludedPaths(t *testing.T) {
	buf := captureLogs(t)
	mw := accessAuditMiddleware(true)

	u := user.New().NewID().Name("Alice").Email("alice@city.osaka.lg.jp").MustBuild()
	au := &appx.AuthInfo{Sub: "auth0|abc"}
	for _, p := range []string{
		"/api/ping",
		"/api/published/foo",
		"/api/published_data/foo",
		"/p/foo/data.json",
		"/assets/x.png",
		"/favicon.ico",
	} {
		c := newAuditRequest(t, http.MethodGet, p, u, au)
		err := mw(func(c echo.Context) error {
			return c.NoContent(http.StatusOK)
		})(c)
		assert.NoError(t, err)
	}

	assert.NotContains(t, buf.String(), accessAuditLogPrefix, "excluded paths should not be audited")
}

func TestAccessAuditMiddleware_EmitsForAuthenticated(t *testing.T) {
	buf := captureLogs(t)
	mw := accessAuditMiddleware(true)

	u := user.New().NewID().Name("Alice").Email("alice@city.osaka.lg.jp").MustBuild()
	au := &appx.AuthInfo{Sub: "auth0|abc", Email: "alice-jwt@city.osaka.lg.jp", Name: "Alice JWT"}
	c := newAuditRequest(t, http.MethodPost, "/api/graphql", u, au)

	err := mw(func(c echo.Context) error {
		return c.NoContent(http.StatusNoContent)
	})(c)
	assert.NoError(t, err)

	rec, ok := extractAuditRecord(t, buf.String())
	if !ok {
		t.Fatalf("expected audit record, got none. logs=%q", buf.String())
	}
	assert.Equal(t, "auth0|abc", rec.Sub)
	assert.Equal(t, u.ID().String(), rec.UserID)
	// DB email wins over JWT email.
	assert.Equal(t, "alice@city.osaka.lg.jp", rec.Email)
	assert.Equal(t, "Alice", rec.Name)
	assert.Equal(t, http.MethodPost, rec.Method)
	assert.Equal(t, "/api/graphql", rec.Path)
	assert.Equal(t, http.StatusNoContent, rec.Status)
	assert.Equal(t, "jwt", rec.AuthMethod)
	assert.NotEmpty(t, rec.Ts)
	assert.Equal(t, "req-1", rec.RequestID)
	assert.Equal(t, "test-agent", rec.UserAgent)
}

func TestAccessAuditMiddleware_FallsBackToJWTEmail(t *testing.T) {
	buf := captureLogs(t)
	mw := accessAuditMiddleware(true)

	au := &appx.AuthInfo{Sub: "auth0|xyz", Email: "jwt@example.com", Name: "JWT Only"}
	c := newAuditRequest(t, http.MethodGet, "/api/graphql", nil, au)

	err := mw(func(c echo.Context) error {
		return c.NoContent(http.StatusOK)
	})(c)
	assert.NoError(t, err)

	rec, ok := extractAuditRecord(t, buf.String())
	if !ok {
		t.Fatalf("expected audit record, got none. logs=%q", buf.String())
	}
	assert.Equal(t, "auth0|xyz", rec.Sub)
	assert.Equal(t, "jwt@example.com", rec.Email)
	assert.Equal(t, "JWT Only", rec.Name)
	assert.Empty(t, rec.UserID)
}
