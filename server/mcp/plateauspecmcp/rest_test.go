package plateauspecmcp

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/eukarya-inc/plateau-spec/plateaudoc"
	"github.com/labstack/echo/v4"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func newSpecTestContext(target string) (echo.Context, *httptest.ResponseRecorder) {
	e := echo.New()
	req := httptest.NewRequest(http.MethodGet, target, nil)
	rec := httptest.NewRecorder()
	return e.NewContext(req, rec), rec
}

func TestHandleSearchHTTP_Validation(t *testing.T) {
	t.Run("query required", func(t *testing.T) {
		c, _ := newSpecTestContext("/search")
		err := handleSearchHTTP(c)
		require.Error(t, err)
		he, ok := err.(*echo.HTTPError)
		require.True(t, ok)
		assert.Equal(t, http.StatusBadRequest, he.Code)
	})

	t.Run("invalid document_type", func(t *testing.T) {
		c, _ := newSpecTestContext("/search?q=LOD&document_type=bogus")
		err := handleSearchHTTP(c)
		require.Error(t, err)
		he, ok := err.(*echo.HTTPError)
		require.True(t, ok)
		assert.Equal(t, http.StatusBadRequest, he.Code)
	})
}

func TestHandleOutlineHTTP(t *testing.T) {
	origFactory := clientFactory
	clientFactory = func() *Client {
		return &Client{
			client: &mockPlateaudocsClient{
				index: &plateaudoc.Index{
					DocumentType: "standard",
					Title:        "3D都市モデル標準製品仕様書",
					Chapters: []plateaudoc.Chapter{
						{Path: "toc1", Title: "第1章 概要"},
						{Path: "toc2", Title: "第2章 データ構造"},
					},
				},
			},
		}
	}
	defer func() { clientFactory = origFactory }()

	t.Run("json", func(t *testing.T) {
		c, rec := newSpecTestContext("/outline?depth=1")
		require.NoError(t, handleOutlineHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)

		var parsed []OutlineItem
		require.NoError(t, json.Unmarshal(rec.Body.Bytes(), &parsed))
		require.Len(t, parsed, 2)
		assert.Equal(t, "第1章 概要", parsed[0].Title)
		assert.Equal(t, "toc1", parsed[0].Path)
	})

	t.Run("markdown", func(t *testing.T) {
		c, rec := newSpecTestContext("/outline?depth=1&format=markdown")
		require.NoError(t, handleOutlineHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)
		assert.Contains(t, rec.Body.String(), "第1章 概要")
		assert.Contains(t, rec.Body.String(), "`toc1`")
	})
}

func TestHandleReadHTTP(t *testing.T) {
	origFactory := clientFactory
	clientFactory = func() *Client {
		return &Client{
			client: &mockPlateaudocsClient{
				index: &plateaudoc.Index{
					DocumentType: "standard",
					Chapters:     []plateaudoc.Chapter{{Path: "toc1", Title: "第1章 概要"}},
				},
				markdownContent: map[string]string{
					"standard/toc1": "# 第1章 概要\n\nPLATEAUの概要。",
				},
			},
		}
	}
	defer func() { clientFactory = origFactory }()

	t.Run("path required", func(t *testing.T) {
		c, _ := newSpecTestContext("/read")
		err := handleReadHTTP(c)
		require.Error(t, err)
		he, ok := err.(*echo.HTTPError)
		require.True(t, ok)
		assert.Equal(t, http.StatusBadRequest, he.Code)
	})

	t.Run("markdown body", func(t *testing.T) {
		c, rec := newSpecTestContext("/read?path=toc1&single_page=true")
		require.NoError(t, handleReadHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)
		assert.Contains(t, rec.Header().Get(echo.HeaderContentType), "text/markdown")
		assert.Contains(t, rec.Body.String(), "PLATEAUの概要")
	})

	t.Run("json body", func(t *testing.T) {
		c, rec := newSpecTestContext("/read?path=toc1&single_page=true&format=json")
		require.NoError(t, handleReadHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)

		var parsed SpecReadResponse
		require.NoError(t, json.Unmarshal(rec.Body.Bytes(), &parsed))
		assert.Equal(t, "toc1", parsed.Path)
		assert.Equal(t, "standard", parsed.DocType)
		assert.Contains(t, parsed.Content, "PLATEAUの概要")
	})
}

func TestParseSearchDocType(t *testing.T) {
	for _, in := range []string{"standard", "procedure", "all", ""} {
		_, err := parseSearchDocType(in)
		assert.NoError(t, err, in)
	}
	_, err := parseSearchDocType("bogus")
	assert.Error(t, err)
}

func TestParseIntDefault(t *testing.T) {
	assert.Equal(t, 10, parseIntDefault("", 10))
	assert.Equal(t, 5, parseIntDefault("5", 10))
	assert.Equal(t, 10, parseIntDefault("abc", 10))
}

func TestDocTypeOrDefault(t *testing.T) {
	assert.Equal(t, "standard", docTypeOrDefault(""))
	assert.Equal(t, "standard", docTypeOrDefault("standard"))
	assert.Equal(t, "procedure", docTypeOrDefault("procedure"))
	assert.Equal(t, "standard", docTypeOrDefault("bogus"))
}
