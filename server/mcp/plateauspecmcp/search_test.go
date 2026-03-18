package plateauspecmcp

import (
	"context"
	"testing"

	"github.com/eukarya-inc/plateau-spec/plateaudocsearch"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// mockSearchClient is a mock implementation of plateaudocsearch.Client
type mockSearchClient struct {
	results []plateaudocsearch.SearchResult
	err     error
}

func (m *mockSearchClient) Search(_ context.Context, _ plateaudocsearch.DocType, _ string, _ ...plateaudocsearch.SearchOption) ([]plateaudocsearch.SearchResult, error) {
	if m.err != nil {
		return nil, m.err
	}
	return m.results, nil
}

func TestHandleSpecSearch(t *testing.T) {
	// Create mock search client
	mockClient := &mockSearchClient{
		results: []plateaudocsearch.SearchResult{
			{
				ID:       "toc4_01",
				DocType:  plateaudocsearch.DocTypeStandard,
				Path:     "toc4_01",
				Title:    "4.1 データ構造",
				Score:    1.5,
				Snippets: []string{"CityGMLのデータ構造について説明する"},
			},
			{
				ID:       "toc4_02",
				DocType:  plateaudocsearch.DocTypeProcedure,
				Path:     "toc4_02",
				Title:    "4.2 属性定義",
				Score:    1.2,
				Snippets: []string{"属性の定義方法"},
			},
		},
	}

	// Override search client factory
	origFactory := searchClientFactory
	searchClientFactory = func(_ context.Context) (*plateaudocsearch.Client, error) {
		// Return a wrapper that satisfies the interface
		// Since we can't easily mock plateaudocsearch.Client directly,
		// we test with the mock results
		return nil, nil
	}
	defer func() { searchClientFactory = origFactory }()

	// For testing, we need to test with the mock results directly
	// by testing formatSearchResults
	t.Run("format search results", func(t *testing.T) {
		content := formatSearchResults("CityGML", mockClient.results)
		assert.Contains(t, content, "検索結果: \"CityGML\"")
		assert.Contains(t, content, "2 件の結果")
		assert.Contains(t, content, "4.1 データ構造")
		assert.Contains(t, content, "4.2 属性定義")
		assert.Contains(t, content, "標準製品仕様書")
		assert.Contains(t, content, "標準作業手順書")
		assert.Contains(t, content, "toc4_01")
	})

	t.Run("format empty results", func(t *testing.T) {
		content := formatSearchResults("存在しないキーワード", []plateaudocsearch.SearchResult{})
		assert.Contains(t, content, "該当する結果が見つかりませんでした")
		assert.Contains(t, content, "検索のヒント")
	})

	t.Run("query required error", func(t *testing.T) {
		req := mcp.CallToolRequest{}
		req.Params.Arguments = map[string]any{}

		_, err := HandleSpecSearch(context.Background(), req)
		require.Error(t, err)
		assert.Contains(t, err.Error(), "query is required")
	})

	t.Run("invalid document type", func(t *testing.T) {
		req := mcp.CallToolRequest{}
		req.Params.Arguments = map[string]any{
			"query":         "test",
			"document_type": "invalid",
		}

		_, err := HandleSpecSearch(context.Background(), req)
		require.Error(t, err)
		assert.Contains(t, err.Error(), "invalid document_type")
	})
}

func TestFormatSearchResults(t *testing.T) {
	t.Run("with results", func(t *testing.T) {
		results := []plateaudocsearch.SearchResult{
			{
				ID:       "toc1",
				DocType:  plateaudocsearch.DocTypeStandard,
				Path:     "toc1",
				Title:    "第1章 概要",
				Score:    2.5,
				Snippets: []string{"PLATEAUプロジェクトの概要"},
			},
		}

		content := formatSearchResults("PLATEAU", results)

		assert.Contains(t, content, "# 検索結果: \"PLATEAU\"")
		assert.Contains(t, content, "1 件の結果")
		assert.Contains(t, content, "第1章 概要")
		assert.Contains(t, content, "標準製品仕様書")
		assert.Contains(t, content, "`toc1`")
		assert.Contains(t, content, "2.50")
		assert.Contains(t, content, "PLATEAUプロジェクトの概要")
		assert.Contains(t, content, "plateau_spec_read")
	})

	t.Run("with procedure document", func(t *testing.T) {
		results := []plateaudocsearch.SearchResult{
			{
				ID:      "toc2",
				DocType: plateaudocsearch.DocTypeProcedure,
				Path:    "toc2",
				Title:   "第2章 作業手順",
				Score:   1.0,
			},
		}

		content := formatSearchResults("手順", results)
		assert.Contains(t, content, "標準作業手順書")
	})

	t.Run("empty results", func(t *testing.T) {
		content := formatSearchResults("存在しない", []plateaudocsearch.SearchResult{})

		assert.Contains(t, content, "該当する結果が見つかりませんでした")
		assert.Contains(t, content, "別のキーワード")
		assert.NotContains(t, content, "件の結果が見つかりました")
	})
}
