package metanorma2md

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFormatTermWithDefinition_Basic(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "termXrefLabel",
				"content": []any{
					map[string]any{"type": "text", "text": "1.5.1"},
				},
			},
			map[string]any{
				"type": "term",
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "3D都市モデル"},
						},
					},
				},
			},
			map[string]any{
				"type": "definition",
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "都市の3Dモデルを表現したデータ。"},
						},
					},
				},
			},
		},
	}

	formatTermWithDefinition(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "### 1.5.1 3D都市モデル")
	assert.Contains(t, result, "都市の3Dモデルを表現したデータ。")
}

func TestFormatTermWithDefinition_TermOnly(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "term",
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "用語のみ"},
						},
					},
				},
			},
		},
	}

	formatTermWithDefinition(&sb, content)
	assert.Contains(t, sb.String(), "### 用語のみ")
}

func TestFormatTermWithDefinition_Nil(t *testing.T) {
	var sb strings.Builder
	formatTermWithDefinition(&sb, nil)
	assert.Empty(t, sb.String())
}

func TestFormatTermWithDefinition_InvalidType(t *testing.T) {
	var sb strings.Builder
	formatTermWithDefinition(&sb, "not a map")
	assert.Empty(t, sb.String())
}

func TestFormatTermWithDefinition_WithLink(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "termXrefLabel",
				"content": []any{
					map[string]any{"type": "text", "text": "2.1"},
				},
			},
			map[string]any{
				"type": "term",
				"content": []any{
					map[string]any{"type": "text", "text": "参照用語"},
				},
			},
			map[string]any{
				"type": "definition",
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "詳細は"},
							map[string]any{
								"type": "external_link",
								"attrs": map[string]any{
									"href": "https://example.com",
								},
								"content": []any{
									map[string]any{"text": "こちら"},
								},
							},
							map[string]any{"type": "text", "text": "を参照。"},
						},
					},
				},
			},
		},
	}

	formatTermWithDefinition(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "### 2.1 参照用語")
	assert.Contains(t, result, "[こちら](https://example.com)")
}

func TestFormatTermWithDefinition_WithSource(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "termXrefLabel",
				"content": []any{
					map[string]any{"type": "text", "text": "1.5.2"},
				},
			},
			map[string]any{
				"type": "term",
				"content": []any{
					map[string]any{
						"type": "strong",
						"content": []any{
							map[string]any{"type": "text", "text": "BIM（Building Information Modeling）"},
						},
					},
				},
			},
			map[string]any{
				"type": "definition",
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "コンピュータ上に作成した建築物情報モデルを構築するもの。"},
						},
					},
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "［出典： PLATEAU Handbook #03-1]"},
						},
					},
				},
			},
		},
	}

	formatTermWithDefinition(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "### 1.5.2 BIM（Building Information Modeling）")
	assert.Contains(t, result, "コンピュータ上に作成した建築物情報モデルを構築するもの。")
	assert.Contains(t, result, "［出典： PLATEAU Handbook #03-1]")
}

func TestExtractTermText(t *testing.T) {
	tests := []struct {
		name     string
		content  map[string]any
		expected string
	}{
		{
			name: "simple text",
			content: map[string]any{
				"content": []any{
					map[string]any{"type": "text", "text": "Simple"},
				},
			},
			expected: "Simple",
		},
		{
			name: "paragraph wrapper",
			content: map[string]any{
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "In Paragraph"},
						},
					},
				},
			},
			expected: "In Paragraph",
		},
		{
			name: "span wrapper",
			content: map[string]any{
				"content": []any{
					map[string]any{
						"type": "span",
						"content": []any{
							map[string]any{"type": "text", "text": "In Span"},
						},
					},
				},
			},
			expected: "In Span",
		},
		{
			name: "nested content",
			content: map[string]any{
				"content": []any{
					map[string]any{
						"type": "strong",
						"content": []any{
							map[string]any{"type": "text", "text": "Strong"},
						},
					},
				},
			},
			expected: "Strong",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := extractTermText(tt.content)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestExtractDefinitionParagraphs(t *testing.T) {
	tests := []struct {
		name     string
		content  map[string]any
		expected []string
	}{
		{
			name: "single paragraph",
			content: map[string]any{
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "Definition text."},
						},
					},
				},
			},
			expected: []string{"Definition text."},
		},
		{
			name: "multiple paragraphs",
			content: map[string]any{
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "First paragraph."},
						},
					},
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "Second paragraph."},
						},
					},
				},
			},
			expected: []string{"First paragraph.", "Second paragraph."},
		},
		{
			name: "empty content",
			content: map[string]any{
				"content": []any{},
			},
			expected: nil,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := extractDefinitionParagraphs(tt.content)
			assert.Equal(t, tt.expected, result)
		})
	}
}
