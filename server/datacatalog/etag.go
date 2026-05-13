package datacatalog

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"strings"

	"github.com/labstack/echo/v4"
)

// stableURLCacheControl is sent on every stable composite URL response so
// clients always revalidate. Source data can be replaced at any time
// (re-publication of the same year, switching of -latest), so we can't safely
// allow caches to serve responses without a conditional GET.
const stableURLCacheControl = "no-cache, must-revalidate"

// writeJSONWithETag serializes v to JSON, attaches a weak ETag derived from
// the payload, sets Cache-Control for stable composite URLs, and either
// returns 304 when If-None-Match matches or writes the JSON body.
func writeJSONWithETag(c echo.Context, v any) error {
	body, err := json.Marshal(v)
	if err != nil {
		return err
	}
	return writeBytesWithETag(c, echo.MIMEApplicationJSON, body)
}

// writeBytesWithETag is the bytes-oriented variant of writeJSONWithETag.
func writeBytesWithETag(c echo.Context, contentType string, body []byte) error {
	etag := weakETag(body)
	h := c.Response().Header()
	h.Set("ETag", etag)
	h.Set(echo.HeaderCacheControl, stableURLCacheControl)

	if matchesIfNoneMatch(c.Request().Header.Get("If-None-Match"), etag) {
		return c.NoContent(http.StatusNotModified)
	}
	return c.Blob(http.StatusOK, contentType, body)
}

// redirectWithETag emits a 302 to target while attaching a weak ETag derived
// from the target URL. The redirect target itself can change (e.g.
// -latest rolling to a newer year, or an asset being re-uploaded), so we
// require revalidation and short-circuit to 304 when the client already
// holds the same target.
func redirectWithETag(c echo.Context, target string) error {
	etag := weakETag([]byte(target))
	h := c.Response().Header()
	h.Set("ETag", etag)
	h.Set(echo.HeaderCacheControl, stableURLCacheControl)

	if matchesIfNoneMatch(c.Request().Header.Get("If-None-Match"), etag) {
		return c.NoContent(http.StatusNotModified)
	}
	return c.Redirect(http.StatusFound, target)
}

func weakETag(body []byte) string {
	sum := sha256.Sum256(body)
	return `W/"` + hex.EncodeToString(sum[:16]) + `"`
}

// matchesIfNoneMatch reports whether the If-None-Match header value selects
// the given ETag. Per RFC 7232 a value of "*" matches any current
// representation, and otherwise the header is a comma-separated list of
// entity-tags compared with the weak comparison function (which, for our
// always-weak tags, reduces to opaque-tag equality).
func matchesIfNoneMatch(header, etag string) bool {
	if header == "" {
		return false
	}
	if strings.TrimSpace(header) == "*" {
		return true
	}
	want := strings.TrimPrefix(etag, "W/")
	for part := range strings.SplitSeq(header, ",") {
		got := strings.TrimPrefix(strings.TrimSpace(part), "W/")
		if got == want {
			return true
		}
	}
	return false
}
