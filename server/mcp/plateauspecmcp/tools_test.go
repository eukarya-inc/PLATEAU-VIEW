package plateauspecmcp

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestRegisterTools(t *testing.T) {
	mcpServer := server.NewMCPServer("test-server", "1.0.0")
	RegisterTools(mcpServer)

	// Verify tools are registered (server should not panic)
	assert.NotNil(t, mcpServer)
}

func TestFormatDocumentAsMarkdown(t *testing.T) {
	t.Run("basic document", func(t *testing.T) {
		doc := &PlateauDocument{
			Title: "Test Document",
			Path:  "/test",
			Content: []PlateauContent{
				{
					Type: "paragraph",
					Content: map[string]any{
						"content": []any{
							map[string]any{"type": "text", "text": "Hello World"},
						},
					},
				},
			},
		}

		markdown := FormatDocumentAsMarkdown(doc, false)
		assert.Contains(t, markdown, "# Test Document")
		assert.Contains(t, markdown, "Hello World")
	})

	t.Run("nil document", func(t *testing.T) {
		markdown := FormatDocumentAsMarkdown(nil, false)
		assert.Empty(t, markdown)
	})
}

func TestHandleSpecOutline(t *testing.T) {
	// Setup mock server
	mockServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		nav := PlateauNavigation{
			Children: []PlateauNavigation{
				{PlainTitle: "第1章 概要", Path: "/plateaudocument/toc1"},
				{PlainTitle: "第2章 データ構造", Path: "/plateaudocument/toc2"},
			},
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(nav)
	}))
	defer mockServer.Close()

	// Override NewClient to use mock server
	origNewClient := newClientFunc
	newClientFunc = func(docType string) *PlateauClient {
		return &PlateauClient{
			BaseURL:      mockServer.URL,
			DocumentType: docType,
			HTTPClient:   http.DefaultClient,
		}
	}
	defer func() { newClientFunc = origNewClient }()

	t.Run("markdown format", func(t *testing.T) {
		req := mcp.CallToolRequest{}
		req.Params.Arguments = map[string]any{
			"format": "markdown",
			"depth":  float64(1),
		}

		result, err := HandleSpecOutline(context.Background(), req)
		require.NoError(t, err)
		require.NotNil(t, result)
		require.Len(t, result.Content, 1)

		textContent, ok := result.Content[0].(mcp.TextContent)
		require.True(t, ok)
		assert.Contains(t, textContent.Text, "第1章 概要")
		assert.Contains(t, textContent.Text, "/plateaudocument/toc1")
	})

	t.Run("json format", func(t *testing.T) {
		req := mcp.CallToolRequest{}
		req.Params.Arguments = map[string]any{
			"format": "json",
			"depth":  float64(1),
		}

		result, err := HandleSpecOutline(context.Background(), req)
		require.NoError(t, err)
		require.NotNil(t, result)

		textContent, ok := result.Content[0].(mcp.TextContent)
		require.True(t, ok)

		// Verify it's valid JSON
		var parsed []OutlineItem
		err = json.Unmarshal([]byte(textContent.Text), &parsed)
		require.NoError(t, err)
		assert.Len(t, parsed, 2)
		assert.Equal(t, "第1章 概要", parsed[0].Title)
	})
}

func TestHandleSpecRead(t *testing.T) {
	// Setup mock server that handles both nav and content requests
	mockServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		switch r.URL.Path {
		case "/test/toc1/resource-nav.json":
			// Navigation response (no children for single page test)
			nav := PlateauNavigation{
				Path:       "/test/toc1",
				PlainTitle: "Test Chapter",
				Children:   []PlateauNavigation{},
			}
			_ = json.NewEncoder(w).Encode(nav)

		case "/test/toc1/resource-content.json":
			// Content response - match the PLATEAU spec API structure
			// The API uses: content.labelInPlainText for title, content.contentDoc.content for actual content
			content := map[string]any{
				"content": map[string]any{
					"labelInPlainText": "Test Chapter",
					"contentDoc": map[string]any{
						"content": []map[string]any{
							{
								"type": "paragraph",
								"content": []map[string]any{
									{"type": "text", "text": "This is test content."},
								},
							},
						},
					},
				},
			}
			_ = json.NewEncoder(w).Encode(content)

		default:
			http.NotFound(w, r)
		}
	}))
	defer mockServer.Close()

	// Override NewClient
	origNewClient := newClientFunc
	newClientFunc = func(docType string) *PlateauClient {
		return &PlateauClient{
			BaseURL:      mockServer.URL,
			DocumentType: docType,
			HTTPClient:   http.DefaultClient,
		}
	}
	defer func() { newClientFunc = origNewClient }()

	t.Run("read single page content", func(t *testing.T) {
		req := mcp.CallToolRequest{}
		req.Params.Arguments = map[string]any{
			"path":        "/test/toc1",
			"single_page": true,
		}

		result, err := HandleSpecRead(context.Background(), req)
		require.NoError(t, err)
		require.NotNil(t, result)

		textContent, ok := result.Content[0].(mcp.TextContent)
		require.True(t, ok)
		assert.Contains(t, textContent.Text, "Test Chapter")
		assert.Contains(t, textContent.Text, "This is test content")
	})

	t.Run("path required error", func(t *testing.T) {
		req := mcp.CallToolRequest{}
		req.Params.Arguments = map[string]any{}

		_, err := HandleSpecRead(context.Background(), req)
		require.Error(t, err)
		assert.Contains(t, err.Error(), "path is required")
	})
}

func TestFormatOutlineAsMarkdown(t *testing.T) {
	items := []OutlineItem{
		{
			ID:    "toc1",
			Title: "Chapter 1",
			Path:  "/doc/toc1",
			Children: []OutlineItem{
				{
					ID:    "sec1",
					Title: "Section 1.1",
					Path:  "/doc/toc1/sec1",
				},
			},
		},
	}

	result := formatOutlineAsMarkdown(items, "standard")
	assert.Contains(t, result, "# 3D都市モデル標準製品仕様書 目次")
	assert.Contains(t, result, "Chapter 1")
	assert.Contains(t, result, "Section 1.1")
	assert.Contains(t, result, "/doc/toc1")
}
