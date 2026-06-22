package plateauspecmcp

import (
	"context"
	"fmt"
	"strings"
	"sync"
	"time"

	"github.com/eukarya-inc/plateau-spec/plateaudocsearch"
	"github.com/mark3labs/mcp-go/mcp"
)

// searchInitTimeout bounds a single attempt to download and open the search
// index. It is deliberately independent of any caller's request context so a
// client disconnect cannot abort — and permanently break — initialization.
const searchInitTimeout = 6 * time.Minute

var (
	searchMu     sync.Mutex
	searchClient *plateaudocsearch.Client // non-nil only after a successful Init
)

// getSearchClient returns the singleton search client, initializing it on first
// use. The search client downloads the search index on first use, which can take
// several seconds.
//
// Initialization is retryable: unlike a sync.Once, a failed attempt (transient
// network error, a 5xx from the index host, a cancelled download, etc.) is NOT
// cached, so the next call tries again instead of failing forever. The download
// also runs on a context detached from the caller, so a single client that
// disconnects mid-download cannot abort initialization for everyone else.
func getSearchClient(ctx context.Context) (*plateaudocsearch.Client, error) {
	searchMu.Lock()
	defer searchMu.Unlock()

	if searchClient != nil {
		return searchClient, nil
	}

	c := plateaudocsearch.New()

	// Detach from the caller's context: downloading the index is a shared,
	// one-time cost, so a single request's cancellation (client disconnect or a
	// per-request timeout) must not cancel it for every other caller. Bound it
	// with our own timeout instead.
	initCtx, cancel := context.WithTimeout(context.WithoutCancel(ctx), searchInitTimeout)
	defer cancel()

	if _, err := c.Init(initCtx); err != nil {
		// Do not cache the failure: the next call retries from scratch.
		return nil, err
	}

	searchClient = c
	return searchClient, nil
}

// Prewarm eagerly initializes the search client so the first real request does
// not pay the index-download cost and a transient startup failure does not
// surface to that first user. It is safe to call from a background goroutine and
// to call more than once; initialization is serialized and retried by
// getSearchClient.
func Prewarm(ctx context.Context) error {
	_, err := getSearchClient(ctx)
	return err
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
