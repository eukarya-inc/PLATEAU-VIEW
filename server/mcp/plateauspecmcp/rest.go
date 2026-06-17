package plateauspecmcp

import (
	"net/http"
	"strconv"
	"strings"

	"github.com/eukarya-inc/plateau-spec/plateaudocsearch"
	"github.com/labstack/echo/v4"
)

// RegisterEcho registers the REST endpoints for the PLATEAU specification
// documents on the given group (mounted at /spec). It exposes the same
// functionality as the plateau_spec_* MCP tools so the specification can be
// queried without an MCP client, in a resource-oriented layout:
//
//	GET /spec                       -> list of documents
//	GET /spec/search?q=...          -> full-text search across documents
//	GET /spec/{docType}             -> outline (table of contents)
//	GET /spec/{docType}/{path}      -> section content
//
// /spec/search is a static route, so it takes precedence over /spec/:docType.
func RegisterEcho(g *echo.Group) {
	g.GET("", handleListHTTP)
	g.GET("/search", handleSearchHTTP)
	g.GET("/:docType", handleOutlineHTTP)
	g.GET("/:docType/:path", handleReadHTTP)
}

// SpecDocument describes one specification document.
type SpecDocument struct {
	DocType string `json:"document_type"`
	Title   string `json:"title"`
	Path    string `json:"path"`
}

// specDocuments is the catalog returned by the list endpoint.
var specDocuments = []SpecDocument{
	{DocType: "standard", Title: "3D都市モデル標準製品仕様書", Path: "/spec/standard"},
	{DocType: "procedure", Title: "3D都市モデル標準作業手順書", Path: "/spec/procedure"},
}

// SpecSearchResult is a single search hit.
type SpecSearchResult struct {
	Title    string   `json:"title"`
	Path     string   `json:"path"`
	DocType  string   `json:"document_type"`
	Score    float64  `json:"score"`
	Snippets []string `json:"snippets,omitempty"`
}

// SpecSearchResponse is the response body of the search endpoint.
type SpecSearchResponse struct {
	Query   string             `json:"query"`
	Results []SpecSearchResult `json:"results"`
}

// SpecReadResponse is the JSON response body of the read endpoint.
type SpecReadResponse struct {
	Path    string `json:"path"`
	DocType string `json:"document_type"`
	Content string `json:"content"`
}

// handleListHTTP handles GET /spec: the catalog of available documents.
func handleListHTTP(c echo.Context) error {
	return c.JSON(http.StatusOK, specDocuments)
}

// handleSearchHTTP handles GET /spec/search.
func handleSearchHTTP(c echo.Context) error {
	ctx := c.Request().Context()

	query := c.QueryParam("q")
	if query == "" {
		query = c.QueryParam("query")
	}
	if query == "" {
		return echo.NewHTTPError(http.StatusBadRequest, "q is required")
	}

	docType, ok := parseSearchDocType(c.QueryParam("document_type"))
	if !ok {
		return echo.NewHTTPError(http.StatusBadRequest, "invalid document_type: must be 'standard', 'procedure', or 'all'")
	}

	limit := parseIntDefault(c.QueryParam("limit"), 10)
	if limit < 1 {
		limit = 10
	}
	if limit > 50 {
		limit = 50
	}

	client, err := searchClientFactory(ctx)
	if err != nil {
		return echo.NewHTTPError(http.StatusInternalServerError, "failed to initialize search index")
	}

	results, err := client.Search(ctx, docType, query, plateaudocsearch.WithLimit(limit))
	if err != nil {
		return echo.NewHTTPError(http.StatusInternalServerError, "search failed")
	}

	res := SpecSearchResponse{
		Query:   query,
		Results: make([]SpecSearchResult, 0, len(results)),
	}
	for _, r := range results {
		res.Results = append(res.Results, SpecSearchResult{
			Title: r.Title,
			// Normalize to an extension-less path so it can be used directly as
			// the {path} segment of GET /spec/{docType}/{path}.
			Path:     strings.TrimSuffix(r.Path, ".md"),
			DocType:  string(r.DocType),
			Score:    r.Score,
			Snippets: r.Snippets,
		})
	}

	return c.JSON(http.StatusOK, res)
}

// handleOutlineHTTP handles GET /spec/{docType}: the table of contents.
func handleOutlineHTTP(c echo.Context) error {
	ctx := c.Request().Context()

	docType := c.Param("docType")
	if !validDocType(docType) {
		return echo.NewHTTPError(http.StatusNotFound, "unknown document type")
	}

	chapter := c.QueryParam("chapter")
	format := c.QueryParam("format")

	depth := parseIntDefault(c.QueryParam("depth"), 2)
	if depth < 1 {
		depth = 1
	}
	if depth > 4 {
		depth = 4
	}

	client := clientFactory()

	var outline []OutlineItem
	var err error
	if chapter != "" {
		outline, err = client.GetChapterOutline(ctx, docType, chapter)
	} else {
		outline, err = client.GetOutline(ctx, docType)
	}
	if err != nil {
		return echo.NewHTTPError(http.StatusInternalServerError, "failed to get outline")
	}

	outline = limitDepth(outline, depth)

	if format == "markdown" {
		md := formatOutlineAsMarkdown(outline, docType, "各節の本文は `/spec/"+docType+"/<path>` で取得できます。")
		return c.Blob(http.StatusOK, "text/markdown; charset=utf-8", []byte(md))
	}
	return c.JSON(http.StatusOK, outline)
}

// handleReadHTTP handles GET /spec/{docType}/{path}: a section's content.
func handleReadHTTP(c echo.Context) error {
	ctx := c.Request().Context()

	docType := c.Param("docType")
	if !validDocType(docType) {
		return echo.NewHTTPError(http.StatusNotFound, "unknown document type")
	}

	// Accept both extension-less paths (from the outline) and .md paths.
	path := strings.TrimSuffix(c.Param("path"), ".md")
	if path == "" {
		return echo.NewHTTPError(http.StatusBadRequest, "path is required")
	}

	singlePage, _ := strconv.ParseBool(c.QueryParam("single_page"))
	format := c.QueryParam("format")

	client := clientFactory()

	var content string
	var err error
	if singlePage {
		content, err = client.GetMarkdown(ctx, docType, path)
	} else {
		content, err = client.GetMarkdownWithChildren(ctx, docType, path)
	}
	if err != nil {
		return echo.NewHTTPError(http.StatusNotFound, "failed to read content")
	}

	if format == "json" {
		return c.JSON(http.StatusOK, SpecReadResponse{
			Path:    path,
			DocType: docType,
			Content: content,
		})
	}
	return c.Blob(http.StatusOK, "text/markdown; charset=utf-8", []byte(content))
}

// parseSearchDocType maps the document_type query param for search. The second
// return value is false when the value is not a recognized document type.
func parseSearchDocType(s string) (plateaudocsearch.DocType, bool) {
	switch s {
	case "standard":
		return plateaudocsearch.DocTypeStandard, true
	case "procedure":
		return plateaudocsearch.DocTypeProcedure, true
	case "all", "":
		return plateaudocsearch.DocTypeAll, true
	default:
		return "", false
	}
}

// validDocType reports whether s is a readable document type (standard or procedure).
func validDocType(s string) bool {
	return s == "standard" || s == "procedure"
}

func parseIntDefault(s string, def int) int {
	if s == "" {
		return def
	}
	if v, err := strconv.Atoi(strings.TrimSpace(s)); err == nil {
		return v
	}
	return def
}
