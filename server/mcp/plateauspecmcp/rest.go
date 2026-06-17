package plateauspecmcp

import (
	"errors"
	"net/http"
	"strconv"
	"strings"

	"github.com/eukarya-inc/plateau-spec/plateaudocsearch"
	"github.com/labstack/echo/v4"
)

// RegisterEcho registers the REST endpoints for PLATEAU specification document
// search/outline/read on the given group. These expose the same functionality
// as the plateau_spec_* MCP tools so the specification can be queried without
// an MCP client.
func RegisterEcho(g *echo.Group) {
	g.GET("/search", handleSearchHTTP)
	g.GET("/outline", handleOutlineHTTP)
	g.GET("/read", handleReadHTTP)
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

// handleSearchHTTP handles GET /search.
func handleSearchHTTP(c echo.Context) error {
	ctx := c.Request().Context()

	query := c.QueryParam("q")
	if query == "" {
		query = c.QueryParam("query")
	}
	if query == "" {
		return echo.NewHTTPError(http.StatusBadRequest, "q is required")
	}

	docType, err := parseSearchDocType(c.QueryParam("document_type"))
	if err != nil {
		return echo.NewHTTPError(http.StatusBadRequest, err.Error())
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
			Title:    r.Title,
			Path:     r.Path,
			DocType:  string(r.DocType),
			Score:    r.Score,
			Snippets: r.Snippets,
		})
	}

	return c.JSON(http.StatusOK, res)
}

// handleOutlineHTTP handles GET /outline.
func handleOutlineHTTP(c echo.Context) error {
	ctx := c.Request().Context()

	docType := docTypeOrDefault(c.QueryParam("document_type"))
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
		return c.String(http.StatusOK, formatOutlineAsMarkdown(outline, docType))
	}
	return c.JSON(http.StatusOK, outline)
}

// handleReadHTTP handles GET /read.
func handleReadHTTP(c echo.Context) error {
	ctx := c.Request().Context()

	path := c.QueryParam("path")
	if path == "" {
		return echo.NewHTTPError(http.StatusBadRequest, "path is required")
	}

	docType := docTypeOrDefault(c.QueryParam("document_type"))
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

// parseSearchDocType validates and maps the document_type query param for search.
func parseSearchDocType(s string) (plateaudocsearch.DocType, error) {
	switch s {
	case "standard":
		return plateaudocsearch.DocTypeStandard, nil
	case "procedure":
		return plateaudocsearch.DocTypeProcedure, nil
	case "all", "":
		return plateaudocsearch.DocTypeAll, nil
	default:
		return "", errors.New("invalid document_type: must be 'standard', 'procedure', or 'all'")
	}
}

// docTypeOrDefault returns the document type for outline/read, defaulting to standard.
func docTypeOrDefault(s string) string {
	if s == "procedure" {
		return "procedure"
	}
	return "standard"
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
