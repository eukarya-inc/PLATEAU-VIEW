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

// newSpecTestContext builds an echo context for the given target URL and path
// params (name/value pairs).
func newSpecTestContext(target string, params ...string) (echo.Context, *httptest.ResponseRecorder) {
	e := echo.New()
	req := httptest.NewRequest(http.MethodGet, target, nil)
	rec := httptest.NewRecorder()
	c := e.NewContext(req, rec)
	if len(params) > 0 {
		names := make([]string, 0, len(params)/2)
		values := make([]string, 0, len(params)/2)
		for i := 0; i+1 < len(params); i += 2 {
			names = append(names, params[i])
			values = append(values, params[i+1])
		}
		c.SetParamNames(names...)
		c.SetParamValues(values...)
	}
	return c, rec
}

func TestHandleListHTTP(t *testing.T) {
	c, rec := newSpecTestContext("/spec")
	require.NoError(t, handleListHTTP(c))
	assert.Equal(t, http.StatusOK, rec.Code)

	var docs []SpecDocument
	require.NoError(t, json.Unmarshal(rec.Body.Bytes(), &docs))
	require.Len(t, docs, 2)
	assert.Equal(t, "standard", docs[0].DocType)
	assert.Equal(t, "/spec/standard", docs[0].Path)
	assert.Equal(t, "procedure", docs[1].DocType)
}

func TestHandleSearchHTTP_Validation(t *testing.T) {
	t.Run("query required", func(t *testing.T) {
		c, _ := newSpecTestContext("/spec/search")
		err := handleSearchHTTP(c)
		require.Error(t, err)
		he, ok := err.(*echo.HTTPError)
		require.True(t, ok)
		assert.Equal(t, http.StatusBadRequest, he.Code)
	})

	t.Run("invalid document_type", func(t *testing.T) {
		c, _ := newSpecTestContext("/spec/search?q=LOD&document_type=bogus")
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
		c, rec := newSpecTestContext("/spec/standard?depth=1", "docType", "standard")
		require.NoError(t, handleOutlineHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)

		var parsed []OutlineItem
		require.NoError(t, json.Unmarshal(rec.Body.Bytes(), &parsed))
		require.Len(t, parsed, 2)
		assert.Equal(t, "第1章 概要", parsed[0].Title)
		assert.Equal(t, "toc1", parsed[0].Path)
	})

	t.Run("markdown", func(t *testing.T) {
		c, rec := newSpecTestContext("/spec/standard?depth=1&format=markdown", "docType", "standard")
		require.NoError(t, handleOutlineHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)
		assert.Contains(t, rec.Header().Get(echo.HeaderContentType), "text/markdown")
		assert.Contains(t, rec.Body.String(), "第1章 概要")
		assert.Contains(t, rec.Body.String(), "`toc1`")
		// REST output points to the REST resource, not the MCP tool.
		assert.Contains(t, rec.Body.String(), "/spec/standard/")
		assert.NotContains(t, rec.Body.String(), "plateau_spec_read")
	})

	t.Run("unknown document type", func(t *testing.T) {
		c, _ := newSpecTestContext("/spec/bogus", "docType", "bogus")
		err := handleOutlineHTTP(c)
		require.Error(t, err)
		he, ok := err.(*echo.HTTPError)
		require.True(t, ok)
		assert.Equal(t, http.StatusNotFound, he.Code)
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

	t.Run("unknown document type", func(t *testing.T) {
		c, _ := newSpecTestContext("/spec/bogus/toc1", "docType", "bogus", "path", "toc1")
		err := handleReadHTTP(c)
		require.Error(t, err)
		he, ok := err.(*echo.HTTPError)
		require.True(t, ok)
		assert.Equal(t, http.StatusNotFound, he.Code)
	})

	t.Run("markdown body", func(t *testing.T) {
		c, rec := newSpecTestContext("/spec/standard/toc1?single_page=true", "docType", "standard", "path", "toc1")
		require.NoError(t, handleReadHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)
		assert.Contains(t, rec.Header().Get(echo.HeaderContentType), "text/markdown")
		assert.Contains(t, rec.Body.String(), "PLATEAUの概要")
	})

	t.Run(".json extension returns json", func(t *testing.T) {
		c, rec := newSpecTestContext("/spec/standard/toc1.json?single_page=true", "docType", "standard", "path", "toc1.json")
		require.NoError(t, handleReadHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)
		assert.Contains(t, rec.Header().Get(echo.HeaderContentType), "application/json")

		var parsed SpecReadResponse
		require.NoError(t, json.Unmarshal(rec.Body.Bytes(), &parsed))
		assert.Equal(t, "toc1", parsed.Path)
		assert.Equal(t, "standard", parsed.DocType)
		assert.Contains(t, parsed.Content, "PLATEAUの概要")
	})

	t.Run(".md extension returns markdown (overrides ?format=json)", func(t *testing.T) {
		c, rec := newSpecTestContext("/spec/standard/toc1.md?single_page=true&format=json", "docType", "standard", "path", "toc1.md")
		require.NoError(t, handleReadHTTP(c))
		assert.Equal(t, http.StatusOK, rec.Code)
		assert.Contains(t, rec.Header().Get(echo.HeaderContentType), "text/markdown")
		assert.Contains(t, rec.Body.String(), "PLATEAUの概要")
	})
}

// TestRegisterEcho_Routing verifies the route layout through a real router,
// in particular that the static /spec/search wins over /spec/:docType.
func TestRegisterEcho_Routing(t *testing.T) {
	origFactory := clientFactory
	clientFactory = func() *Client {
		return &Client{
			client: &mockPlateaudocsClient{
				index: &plateaudoc.Index{
					DocumentType: "standard",
					Chapters:     []plateaudoc.Chapter{{Path: "toc1", Title: "第1章 概要"}},
				},
				markdownContent: map[string]string{"standard/toc1": "# 第1章"},
			},
		}
	}
	defer func() { clientFactory = origFactory }()

	e := echo.New()
	RegisterEcho(e.Group("/spec"))

	do := func(target string) int {
		req := httptest.NewRequest(http.MethodGet, target, nil)
		rec := httptest.NewRecorder()
		e.ServeHTTP(rec, req)
		return rec.Code
	}

	assert.Equal(t, http.StatusOK, do("/spec"), "list")
	// /spec/search must route to the search handler (400 for missing q), not
	// the outline handler (which would 404 on document type "search").
	assert.Equal(t, http.StatusBadRequest, do("/spec/search"), "search wins over :docType")
	assert.Equal(t, http.StatusOK, do("/spec/standard?depth=1"), "outline")
	assert.Equal(t, http.StatusOK, do("/spec/standard/toc1?single_page=true"), "read")
	assert.Equal(t, http.StatusNotFound, do("/spec/bogus"), "unknown doc type")
}

func TestParseSearchDocType(t *testing.T) {
	for _, in := range []string{"standard", "procedure", "all", ""} {
		_, ok := parseSearchDocType(in)
		assert.True(t, ok, in)
	}
	_, ok := parseSearchDocType("bogus")
	assert.False(t, ok)
}

func TestValidDocType(t *testing.T) {
	assert.True(t, validDocType("standard"))
	assert.True(t, validDocType("procedure"))
	assert.False(t, validDocType(""))
	assert.False(t, validDocType("bogus"))
}

func TestParseIntDefault(t *testing.T) {
	assert.Equal(t, 10, parseIntDefault("", 10))
	assert.Equal(t, 5, parseIntDefault("5", 10))
	assert.Equal(t, 10, parseIntDefault("abc", 10))
}
