package cmsintsetup

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
)

func TestHandler_AuthScope(t *testing.T) {
	e := echo.New()
	g := e.Group("")
	assert.NoError(t, Handler(Config{
		Token:            "token",
		CMSURL:           "https://example.com",
		CMSToken:         "cmstoken",
		CMSSystemProject: "system",
	}, g))
	g.POST("/other", func(c echo.Context) error {
		return c.NoContent(http.StatusOK)
	})

	req := httptest.NewRequest(http.MethodPost, "/setup", nil)
	rec := httptest.NewRecorder()
	e.ServeHTTP(rec, req)
	assert.Equal(t, http.StatusUnauthorized, rec.Code)

	req = httptest.NewRequest(http.MethodPost, "/other", nil)
	rec = httptest.NewRecorder()
	e.ServeHTTP(rec, req)
	assert.Equal(t, http.StatusOK, rec.Code)
}

func TestHandler_EmptyToken(t *testing.T) {
	e := echo.New()
	assert.NoError(t, Handler(Config{
		CMSURL:           "https://example.com",
		CMSToken:         "cmstoken",
		CMSSystemProject: "system",
	}, e.Group("")))

	req := httptest.NewRequest(http.MethodPost, "/setup", nil)
	req.Header.Set("Authorization", "Bearer ")
	rec := httptest.NewRecorder()
	e.ServeHTTP(rec, req)
	assert.Equal(t, http.StatusUnauthorized, rec.Code)
}
