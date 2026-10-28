package proxy

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"testing"

	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
)

func TestEcho(t *testing.T) {
	var gotPath, gotKey string
	upstream := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.Path
		gotKey = r.URL.Query().Get("acl:consumerKey")
		_, _ = w.Write([]byte("ok"))
	}))
	defer upstream.Close()

	target, _ := url.Parse(upstream.URL)
	e := echo.New()
	echoWithTarget(e.Group("/odpt-proxy"), Config{ODPTConsumerKey: "KEY"}, target)

	tests := []struct {
		name string
		path string
		code int
	}{
		{name: "allowed", path: "/odpt-proxy/api/v4/gtfs/realtime/ToeiBus", code: http.StatusOK},
		{name: "other api", path: "/odpt-proxy/api/v4/odpt:Train", code: http.StatusNotFound},
		{name: "other feed", path: "/odpt-proxy/api/v4/gtfs/realtime/Other", code: http.StatusNotFound},
		{name: "encoded traversal", path: "/odpt-proxy/api/v4/gtfs/realtime/ToeiBus%2F..%2F..%2Fx", code: http.StatusNotFound},
		{name: "root", path: "/odpt-proxy/", code: http.StatusNotFound},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gotPath, gotKey = "", ""
			req := httptest.NewRequest(http.MethodGet, tt.path, nil)
			rec := httptest.NewRecorder()
			e.ServeHTTP(rec, req)
			assert.Equal(t, tt.code, rec.Code)
			if tt.code == http.StatusOK {
				assert.Equal(t, "/api/v4/gtfs/realtime/ToeiBus", gotPath)
				assert.Equal(t, "KEY", gotKey)
			} else {
				assert.Empty(t, gotPath)
			}
		})
	}
}
