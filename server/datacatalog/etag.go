package datacatalog

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"strings"

	"github.com/labstack/echo/v4"
)

// stableURLCacheControl is sent on every stable composite URL response so
// clients always revalidate. Source data can be replaced at any time
// (re-publication of the same year, switching of -latest), so we can't safely
// allow caches to serve responses without a conditional GET.
const stableURLCacheControl = "no-cache, must-revalidate"

// setRevisionETag attaches a weak ETag derived from the repo revision, request
// host, and URL path, plus the standard stable-URL Cache-Control header. The
// returned bool reports whether the client's If-None-Match already matches —
// when true the caller should return 304 without producing a body, skipping
// any expensive downstream computation.
//
// Using the repo revision instead of hashing the response body lets the
// 304 fast path fire before we serialize multi-megabyte responses, and keeps
// ETags stable across replicas that share the same underlying data.
func setRevisionETag(c echo.Context, revision string) (etag string, matched bool) {
	req := c.Request()
	h := sha256.New()
	_, _ = fmt.Fprintf(h, "%s|%s|%s", revision, req.Host, req.URL.Path)
	etag = `W/"` + hex.EncodeToString(h.Sum(nil)[:16]) + `"`

	resp := c.Response().Header()
	resp.Set("ETag", etag)
	resp.Set(echo.HeaderCacheControl, stableURLCacheControl)

	return etag, matchesIfNoneMatch(req.Header.Get("If-None-Match"), etag)
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
