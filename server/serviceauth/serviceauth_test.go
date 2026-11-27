package serviceauth

import (
	"context"
	"errors"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
)

func TestMiddleware(t *testing.T) {
	const aud = "https://api.example.com"
	validate := func(ctx context.Context, token, audience string) (map[string]any, error) {
		if audience != aud {
			return nil, errors.New("wrong audience")
		}
		switch token {
		case "job":
			return map[string]any{"iss": "https://accounts.google.com", "email": "job@p.iam.gserviceaccount.com", "email_verified": true}, nil
		case "other":
			return map[string]any{"iss": "https://accounts.google.com", "email": "other@p.iam.gserviceaccount.com", "email_verified": true}, nil
		case "foreign":
			return map[string]any{"iss": "https://example.com", "email": "job@p.iam.gserviceaccount.com", "email_verified": true}, nil
		case "unverified":
			return map[string]any{"iss": "https://accounts.google.com", "email": "job@p.iam.gserviceaccount.com", "email_verified": false}, nil
		}
		return nil, errors.New("invalid")
	}

	run := func(audience string, accounts []string, header string) int {
		e := echo.New()
		req := httptest.NewRequest(http.MethodPost, "/", nil)
		if header != "" {
			req.Header.Set("Authorization", header)
		}
		rec := httptest.NewRecorder()
		h := Middleware(audience, accounts, validate)(func(c echo.Context) error {
			return c.NoContent(http.StatusOK)
		})
		_ = h(e.NewContext(req, rec))
		return rec.Code
	}

	accounts := []string{"job@p.iam.gserviceaccount.com"}
	assert.Equal(t, http.StatusOK, run(aud, accounts, "Bearer job"))
	assert.Equal(t, http.StatusForbidden, run(aud, accounts, "Bearer other"))
	assert.Equal(t, http.StatusForbidden, run(aud, accounts, "Bearer unverified"))
	assert.Equal(t, http.StatusUnauthorized, run(aud, accounts, "Bearer foreign"))
	assert.Equal(t, http.StatusUnauthorized, run(aud, accounts, "Bearer broken"))
	// spaces around the configured emails and a trailing slash of the audience are ignored
	assert.Equal(t, http.StatusOK, run(aud+"/", []string{" other@x", " job@p.iam.gserviceaccount.com "}, "Bearer job"))
	assert.Equal(t, http.StatusUnauthorized, run(aud, accounts, ""))
	assert.Equal(t, http.StatusUnauthorized, run(aud, accounts, "job"))
	assert.Equal(t, http.StatusUnauthorized, run(aud, nil, "Bearer job"))
	assert.Equal(t, http.StatusUnauthorized, run("", accounts, "Bearer job"))
}
