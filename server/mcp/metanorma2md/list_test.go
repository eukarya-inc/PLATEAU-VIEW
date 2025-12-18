package metanorma2md

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFormatListContent_SimpleItems(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "list_item",
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "First item"},
						},
					},
				},
			},
			map[string]any{
				"type": "list_item",
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "Second item"},
						},
					},
				},
			},
		},
	}

	formatListContent(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "- First item")
	assert.Contains(t, result, "- Second item")
}

func TestFormatListContent_TitleDescriptionPattern(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "list_item",
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "空間参照系の識別"},
						},
					},
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "幾何オブジェクトに付与された空間参照系を表す識別子。"},
						},
					},
				},
			},
		},
	}

	formatListContent(&sb, content)
	result := sb.String()

	// Should format as "**Title**：Description"
	assert.Contains(t, result, "**空間参照系の識別**")
	assert.Contains(t, result, "：幾何オブジェクトに付与された空間参照系を表す識別子。")
}

func TestFormatListItemChildren_Empty(t *testing.T) {
	var sb strings.Builder
	formatListItemChildren(&sb, []any{})
	assert.Empty(t, sb.String())
}

func TestFormatListItemChildren_SingleParagraph(t *testing.T) {
	var sb strings.Builder
	children := []any{
		map[string]any{
			"type": "paragraph",
			"content": []any{
				map[string]any{"type": "text", "text": "Single item"},
			},
		},
	}

	formatListItemChildren(&sb, children)
	result := sb.String()

	assert.Contains(t, result, "Single item")
	// Should NOT have bold formatting for single paragraph
	assert.NotContains(t, result, "**")
}

func TestFormatListItemChildren_WithExternalLink(t *testing.T) {
	var sb strings.Builder
	children := []any{
		map[string]any{
			"type": "paragraph",
			"content": []any{
				map[string]any{"type": "text", "text": "Check "},
				map[string]any{
					"type": "external_link",
					"attrs": map[string]any{
						"href": "https://example.com",
					},
					"content": []any{
						map[string]any{"text": "this link"},
					},
				},
			},
		},
	}

	formatListItemChildren(&sb, children)
	result := sb.String()

	assert.Contains(t, result, "Check ")
	assert.Contains(t, result, "[this link](https://example.com)")
}

func TestFormatListContent_Nil(t *testing.T) {
	var sb strings.Builder
	formatListContent(&sb, nil)
	// Should just add trailing newline
	assert.Equal(t, "\n", sb.String())
}
