package estat

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"testing"

	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
)

func TestEcho(t *testing.T) {
	e := echo.New()
	Echo(e.Group("/estat"), NewFinder(&fakeStore{docs: testDocs}))

	get := func(target string) *httptest.ResponseRecorder {
		rec := httptest.NewRecorder()
		e.ServeHTTP(rec, httptest.NewRequest(http.MethodGet, target, nil))
		return rec
	}

	rec := get("/estat/areas?q=" + url.QueryEscape("渋谷区") + "&q=" + url.QueryEscape("渋谷"))
	assert.Equal(t, http.StatusOK, rec.Code)
	assert.JSONEq(t, `[{"id":"2","prefectureCode":"13","municipalityCode":"13113","name":"渋谷","address":"東京都渋谷区渋谷","addressComponents":["東京都","渋谷区","渋谷"],"bbox":[139,35,140,36]}]`, rec.Body.String())

	rec = get("/estat/areas")
	assert.Equal(t, http.StatusOK, rec.Code)
	assert.JSONEq(t, `[]`, rec.Body.String())

	assert.Equal(t, http.StatusBadRequest, get("/estat/areas?q=a&limit=x").Code)
	assert.Equal(t, http.StatusNotFound, get("/estat/areas/none/geometry").Code)
}
