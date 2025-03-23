package tiles

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/jarcoal/httpmock"
	"github.com/labstack/echo/v4"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestHanlder_GetTile(t *testing.T) {
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	httpmock.RegisterResponder("GET", "https://example.com/1/2/3.png", httpmock.NewStringResponder(200, "image"))

	h := &Handler{
		tiles: Tiles{
			"test": []lo.Entry[Range, string]{
				{
					Key:   Range{ZMin: -1, ZMax: -1, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
					Value: "https://example.com",
				},
			},
		},
		http: http.DefaultClient,
	}

	// 200
	req := httptest.NewRequest("GET", "/test/1/2/3.png", nil)
	w := httptest.NewRecorder()
	c := echo.New().NewContext(req, w)
	c.SetParamNames("id", "z", "x", "y")
	c.SetParamValues("test", "1", "2", "3.png")

	assert.NoError(t, h.GetTile(c))
	assert.Equal(t, 200, w.Code)
	assert.Equal(t, "image", w.Body.String())

	// 404
	req = httptest.NewRequest("GET", "/test2/1/2/3.png", nil)
	w = httptest.NewRecorder()
	c = echo.New().NewContext(req, w)
	c.SetParamNames("id", "z", "x", "y")
	c.SetParamValues("test2", "1", "2", "3.png")

	assert.NoError(t, h.GetTile(c))
	assert.Equal(t, 404, w.Code)
	assert.Equal(t, `{"error":"not found"}`+"\n", w.Body.String())
}
