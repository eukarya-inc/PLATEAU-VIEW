package plateauspecmcp

import (
	"context"
	"encoding/json"
	"testing"

	"github.com/eukarya-inc/plateau-spec/plateaudoc"
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

func TestHandleSpecOutline(t *testing.T) {
	// Override client factory to use mock client
	origFactory := clientFactory
	clientFactory = func() *Client {
		return &Client{
			client: &mockPlateaudocsClient{
				index: &plateaudoc.Index{
					DocumentType: "standard",
					Title:        "3D都市モデル標準製品仕様書",
					Chapters: []plateaudoc.Chapter{
						{Path: "toc1", Title: "第1章 概要", Children: nil},
						{Path: "toc2", Title: "第2章 データ構造", Children: nil},
					},
				},
			},
		}
	}
	defer func() { clientFactory = origFactory }()

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
		assert.Contains(t, textContent.Text, "toc1")
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
	// Override client factory
	origFactory := clientFactory
	clientFactory = func() *Client {
		return &Client{
			client: &mockPlateaudocsClient{
				index: &plateaudoc.Index{
					DocumentType: "standard",
					Title:        "3D都市モデル標準製品仕様書",
					Chapters: []plateaudoc.Chapter{
						{Path: "toc1", Title: "Test Chapter", Children: nil},
					},
				},
				markdownContent: map[string]string{
					"standard/toc1": "# Test Chapter\n\nThis is test content.",
				},
			},
		}
	}
	defer func() { clientFactory = origFactory }()

	t.Run("read single page content", func(t *testing.T) {
		req := mcp.CallToolRequest{}
		req.Params.Arguments = map[string]any{
			"path":        "toc1",
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
			Path:  "toc1",
			Children: []OutlineItem{
				{
					ID:    "sec1",
					Title: "Section 1.1",
					Path:  "toc1_01",
				},
			},
		},
	}

	result := formatOutlineAsMarkdown(items, "standard")
	assert.Contains(t, result, "# 3D都市モデル標準製品仕様書 目次")
	assert.Contains(t, result, "Chapter 1")
	assert.Contains(t, result, "Section 1.1")
	assert.Contains(t, result, "toc1")
}

func TestLimitDepth(t *testing.T) {
	items := []OutlineItem{
		{
			ID:    "toc1",
			Title: "Chapter 1",
			Path:  "toc1",
			Children: []OutlineItem{
				{
					ID:    "sec1",
					Title: "Section 1.1",
					Path:  "toc1_01",
					Children: []OutlineItem{
						{
							ID:    "sub1",
							Title: "Subsection 1.1.1",
							Path:  "toc1_01_01",
						},
					},
				},
			},
		},
	}

	t.Run("depth 1", func(t *testing.T) {
		result := limitDepth(items, 1)
		require.Len(t, result, 1)
		assert.Equal(t, "Chapter 1", result[0].Title)
		assert.Nil(t, result[0].Children)
	})

	t.Run("depth 2", func(t *testing.T) {
		result := limitDepth(items, 2)
		require.Len(t, result, 1)
		require.Len(t, result[0].Children, 1)
		assert.Equal(t, "Section 1.1", result[0].Children[0].Title)
		assert.Nil(t, result[0].Children[0].Children)
	})

	t.Run("depth 3", func(t *testing.T) {
		result := limitDepth(items, 3)
		require.Len(t, result, 1)
		require.Len(t, result[0].Children, 1)
		require.Len(t, result[0].Children[0].Children, 1)
		assert.Equal(t, "Subsection 1.1.1", result[0].Children[0].Children[0].Title)
	})
}

// mockPlateaudocsClient is a mock implementation of plateaudocsClient interface
type mockPlateaudocsClient struct {
	index           *plateaudoc.Index
	markdownContent map[string]string
}

func (c *mockPlateaudocsClient) GetIndex(_ context.Context, _ string) (*plateaudoc.Index, error) {
	return c.index, nil
}

func (c *mockPlateaudocsClient) GetMarkdown(_ context.Context, docType, path string) (string, error) {
	key := docType + "/" + path
	if content, ok := c.markdownContent[key]; ok {
		return content, nil
	}
	return "", nil
}
