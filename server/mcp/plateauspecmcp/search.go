package plateauspecmcp

import (
	"context"
	"fmt"
	"strings"
	"sync"

	"github.com/eukarya-inc/plateau-spec/plateaudocsearch"
	"github.com/mark3labs/mcp-go/mcp"
)

var (
	searchClient     *plateaudocsearch.Client
	searchClientOnce sync.Once
	searchClientErr  error
)

// getSearchClient returns the singleton search client, initializing it if necessary.
// The search client downloads the search index on first use.
func getSearchClient(ctx context.Context) (*plateaudocsearch.Client, error) {
	searchClientOnce.Do(func() {
		searchClient = plateaudocsearch.New()
		_, searchClientErr = searchClient.Init(ctx)
		if searchClientErr != nil {
			searchClient = nil
		}
	})

	if searchClientErr != nil {
		return nil, searchClientErr
	}
	return searchClient, nil
}

// searchClientFactory allows overriding getSearchClient for testing
var searchClientFactory = getSearchClient

var specSearchTool = mcp.NewTool("plateau_spec_search",
	mcp.WithDescription("Full-text search in PLATEAU 3D City Model specification documents. Returns matching sections with relevance scores and snippets. Use this to find specific topics, definitions, or requirements in the specification."),
	mcp.WithReadOnlyHintAnnotation(true),
	mcp.WithString("query",
		mcp.Required(),
		mcp.Description("Search query. Supports Japanese text search. Example: 'LOD', 'CityGML', '属性', 'メタデータ'"),
	),
	mcp.WithString("document_type",
		mcp.Description("Document type to search: 'standard' (3D City Model Standard Product Specification), 'procedure' (Standard Work Procedures), or 'all' (both). Default: 'all'"),
	),
	mcp.WithNumber("limit",
		mcp.Description("Maximum number of results to return. Default: 10, Max: 50"),
	),
)

// HandleSpecSearch handles the plateau_spec_search tool
func HandleSpecSearch(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	query := request.GetString("query", "")
	docType := request.GetString("document_type", "all")
	limit := request.GetInt("limit", 10)

	if query == "" {
		return nil, fmt.Errorf("query is required")
	}

	// Validate and normalize document type
	var searchDocType plateaudocsearch.DocType
	switch docType {
	case "standard":
		searchDocType = plateaudocsearch.DocTypeStandard
	case "procedure":
		searchDocType = plateaudocsearch.DocTypeProcedure
	case "all", "":
		searchDocType = plateaudocsearch.DocTypeAll
	default:
		return nil, fmt.Errorf("invalid document_type: %s (must be 'standard', 'procedure', or 'all')", docType)
	}

	// Validate limit
	if limit < 1 {
		limit = 10
	}
	if limit > 50 {
		limit = 50
	}

	// Get or initialize the search client
	client, err := searchClientFactory(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize search client: %w", err)
	}

	// Perform search
	results, err := client.Search(ctx, searchDocType, query, plateaudocsearch.WithLimit(limit))
	if err != nil {
		return nil, fmt.Errorf("search failed: %w", err)
	}

	// Format results
	content := formatSearchResults(query, results)
	return mcp.NewToolResultText(content), nil
}

// formatSearchResults formats search results as markdown
func formatSearchResults(query string, results []plateaudocsearch.SearchResult) string {
	var sb strings.Builder

	sb.WriteString(fmt.Sprintf("# 検索結果: \"%s\"\n\n", query))

	if len(results) == 0 {
		sb.WriteString("該当する結果が見つかりませんでした。\n\n")
		sb.WriteString("検索のヒント:\n")
		sb.WriteString("- 別のキーワードを試してください\n")
		sb.WriteString("- より一般的な用語を使用してください\n")
		sb.WriteString("- 日本語と英語の両方で検索してみてください\n")
		return sb.String()
	}

	sb.WriteString(fmt.Sprintf("%d 件の結果が見つかりました。\n\n", len(results)))

	for i, result := range results {
		docTypeName := "標準製品仕様書"
		if result.DocType == plateaudocsearch.DocTypeProcedure {
			docTypeName = "標準作業手順書"
		}

		sb.WriteString(fmt.Sprintf("## %d. %s\n\n", i+1, result.Title))
		sb.WriteString(fmt.Sprintf("- **ドキュメント**: %s\n", docTypeName))
		sb.WriteString(fmt.Sprintf("- **パス**: `%s`\n", result.Path))
		sb.WriteString(fmt.Sprintf("- **関連度スコア**: %.2f\n", result.Score))

		if len(result.Snippets) > 0 {
			sb.WriteString("\n**マッチ箇所**:\n")
			for _, snippet := range result.Snippets {
				// Clean up snippet and add quote formatting
				snippet = strings.TrimSpace(snippet)
				if snippet != "" {
					sb.WriteString(fmt.Sprintf("> %s\n\n", snippet))
				}
			}
		}

		sb.WriteString("\n---\n\n")
	}

	sb.WriteString("詳細を読むには `plateau_spec_read` ツールでパスを指定してください。\n")

	return sb.String()
}
