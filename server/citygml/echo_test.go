package citygml

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"testing"

	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
)

func TestValidateCityGMLURL(t *testing.T) {
	tests := []struct {
		name       string
		url        string
		domain     string
		wantCode   int
		wantErrMsg string
	}{
		{
			name:   "valid",
			url:    "https://example.com/udx/bldg/foo.gml",
			domain: "example.com",
		},
		{
			name:   "valid http",
			url:    "http://example.com/udx/bldg/foo.gml",
			domain: "example.com",
		},
		{
			name:       "domain not configured",
			url:        "https://example.com/udx/bldg/foo.gml",
			domain:     "",
			wantCode:   http.StatusServiceUnavailable,
			wantErrMsg: "citygml domain is not configured",
		},
		{
			name:       "domain not configured with internal url",
			url:        "http://169.254.169.254/computeMetadata/v1/",
			domain:     "",
			wantCode:   http.StatusServiceUnavailable,
			wantErrMsg: "citygml domain is not configured",
		},
		{
			name:       "other domain",
			url:        "https://evil.example.org/foo.gml",
			domain:     "example.com",
			wantCode:   http.StatusBadRequest,
			wantErrMsg: "invalid domain",
		},
		{
			name:       "invalid scheme",
			url:        "file:///etc/passwd",
			domain:     "example.com",
			wantCode:   http.StatusBadRequest,
			wantErrMsg: "invalid url scheme",
		},
		{
			name:       "invalid url",
			url:        "http://example.com/%%",
			domain:     "example.com",
			wantCode:   http.StatusBadRequest,
			wantErrMsg: "invalid url",
		},
		{
			name:       "empty url",
			url:        "",
			domain:     "example.com",
			wantCode:   http.StatusBadRequest,
			wantErrMsg: "invalid url scheme",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			code, msg := validateCityGMLURL(tt.url, tt.domain)
			assert.Equal(t, tt.wantCode, code)
			assert.Equal(t, tt.wantErrMsg, msg)
		})
	}
}

func TestHandlersRejectWhenDomainIsNotConfigured(t *testing.T) {
	for name, h := range map[string]func(string) echo.HandlerFunc{
		"attributes": attributeHandler,
		"features":   featureHandler,
	} {
		t.Run(name, func(t *testing.T) {
			u := &url.URL{Path: "/" + name}
			q := url.Values{}
			q.Set("url", "http://169.254.169.254/computeMetadata/v1/")
			q.Set("id", "foo")
			q.Set("sid", "/18/0/0/0")
			u.RawQuery = q.Encode()

			req := httptest.NewRequest(http.MethodGet, u.String(), nil)
			rec := httptest.NewRecorder()
			c := echo.New().NewContext(req, rec)

			assert.NoError(t, h("")(c))
			assert.Equal(t, http.StatusServiceUnavailable, rec.Code)
			assert.JSONEq(t, `{"url":"http://169.254.169.254/computeMetadata/v1/","error":"citygml domain is not configured"}`, rec.Body.String())
		})
	}
}
