package metanorma2md

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFormatTextContent_String(t *testing.T) {
	var sb strings.Builder
	formatTextContent(&sb, "Hello World")
	assert.Equal(t, "Hello World", sb.String())
}

func TestFormatTextContent_Map(t *testing.T) {
	var sb strings.Builder
	formatTextContent(&sb, map[string]any{
		"text": "Map Text",
	})
	assert.Equal(t, "Map Text", sb.String())
}

func TestFormatTextContent_Nil(t *testing.T) {
	var sb strings.Builder
	formatTextContent(&sb, nil)
	assert.Empty(t, sb.String())
}

func TestFormatParagraphContent(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{"type": "text", "text": "First "},
			map[string]any{"type": "text", "text": "Second"},
		},
	}

	formatParagraphContent(&sb, content)
	assert.Equal(t, "First Second\n\n", sb.String())
}

func TestFormatInlineContent_Text(t *testing.T) {
	var sb strings.Builder
	formatInlineContent(&sb, map[string]any{
		"type": "text",
		"text": "plain text",
	})
	assert.Equal(t, "plain text", sb.String())
}

func TestFormatInlineContent_Linebreak(t *testing.T) {
	var sb strings.Builder
	formatInlineContent(&sb, map[string]any{
		"type": "linebreak",
	})
	assert.Equal(t, "\n", sb.String())
}

func TestFormatInlineContent_Code(t *testing.T) {
	var sb strings.Builder
	formatInlineContent(&sb, map[string]any{
		"type": "code",
		"content": []any{
			map[string]any{"type": "text", "text": "inline_code"},
		},
	})
	assert.Equal(t, "`inline_code`", sb.String())
}

func TestFormatInlineContent_Bold(t *testing.T) {
	var sb strings.Builder
	formatInlineContent(&sb, map[string]any{
		"type": "bold",
		"content": []any{
			map[string]any{"type": "text", "text": "bold text"},
		},
	})
	// Bold is just extracted as plain text (no markdown bold for inline)
	assert.Equal(t, "bold text", sb.String())
}

func TestFormatInlineContent_Nested(t *testing.T) {
	var sb strings.Builder
	formatInlineContent(&sb, map[string]any{
		"type": "span",
		"content": []any{
			map[string]any{
				"type": "strong",
				"content": []any{
					map[string]any{"type": "text", "text": "nested"},
				},
			},
		},
	})
	assert.Equal(t, "nested", sb.String())
}

func TestFormatTitleContent(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{"type": "text", "text": "Section Title"},
		},
	}

	formatTitleContent(&sb, content)
	assert.Equal(t, "## Section Title\n\n", sb.String())
}

func TestFormatTitleContent_WithTab(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{"type": "text", "text": "7.2.3\tファイル名称"},
		},
	}

	formatTitleContent(&sb, content)
	assert.Contains(t, sb.String(), "## 7.2.3 ファイル名称")
}

func TestFormatHeadingContent(t *testing.T) {
	tests := []struct {
		name     string
		content  any
		expected string
	}{
		{
			name: "level 2",
			content: map[string]any{
				"level": float64(2),
				"text":  "Heading 2",
			},
			expected: "## Heading 2\n\n",
		},
		{
			name: "level 3",
			content: map[string]any{
				"level": float64(3),
				"text":  "Heading 3",
			},
			expected: "### Heading 3\n\n",
		},
		{
			name: "default level",
			content: map[string]any{
				"text": "No Level",
			},
			expected: "## No Level\n\n",
		},
		{
			name: "with children",
			content: map[string]any{
				"level": float64(4),
				"children": []any{
					map[string]any{"text": "Child "},
					map[string]any{"text": "Text"},
				},
			},
			expected: "#### Child Text\n\n",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var sb strings.Builder
			formatHeadingContent(&sb, tt.content)
			assert.Equal(t, tt.expected, sb.String())
		})
	}
}

func TestFormatCodeContent_SingleLine(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{"type": "text", "text": "single line code"},
		},
	}
	formatCodeContent(&sb, content)
	// Single line code uses inline backticks
	assert.Contains(t, sb.String(), "`single line code`")
}

func TestFormatCodeContent_Multiline(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{"type": "text", "text": "line1\nline2"},
		},
	}
	formatCodeContent(&sb, content)
	result := sb.String()

	assert.True(t, strings.HasPrefix(result, "```\n"))
	assert.Contains(t, result, "line1\nline2")
	assert.True(t, strings.HasSuffix(result, "\n```\n\n"))
}

func TestExtractTextRecursive(t *testing.T) {
	tests := []struct {
		name     string
		content  map[string]any
		expected string
	}{
		{
			name: "simple text",
			content: map[string]any{
				"text": "simple",
			},
			expected: "simple",
		},
		{
			name: "nested content",
			content: map[string]any{
				"content": []any{
					map[string]any{"text": "first "},
					map[string]any{"text": "second"},
				},
			},
			expected: "first second",
		},
		{
			name: "deeply nested",
			content: map[string]any{
				"content": []any{
					map[string]any{
						"content": []any{
							map[string]any{"text": "deep"},
						},
					},
				},
			},
			expected: "deep",
		},
		{
			name:     "empty",
			content:  map[string]any{},
			expected: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := extractTextRecursive(tt.content)
			assert.Equal(t, tt.expected, result)
		})
	}
}
