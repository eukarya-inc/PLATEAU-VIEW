package metanorma2md

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFormatTableContent_Basic(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type": "table_cell",
						"content": []any{
							map[string]any{
								"type": "paragraph",
								"content": []any{
									map[string]any{"type": "text", "text": "A"},
								},
							},
						},
					},
					map[string]any{
						"type": "table_cell",
						"content": []any{
							map[string]any{
								"type": "paragraph",
								"content": []any{
									map[string]any{"type": "text", "text": "B"},
								},
							},
						},
					},
				},
			},
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type": "table_cell",
						"content": []any{
							map[string]any{
								"type": "paragraph",
								"content": []any{
									map[string]any{"type": "text", "text": "1"},
								},
							},
						},
					},
					map[string]any{
						"type": "table_cell",
						"content": []any{
							map[string]any{
								"type": "paragraph",
								"content": []any{
									map[string]any{"type": "text", "text": "2"},
								},
							},
						},
					},
				},
			},
		},
	}

	formatTableContent(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "| A | B |")
	assert.Contains(t, result, "| --- | --- |")
	assert.Contains(t, result, "| 1 | 2 |")
}

func TestFormatTableContent_WithCode(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type": "table_cell",
						"content": []any{
							map[string]any{
								"type": "code",
								"content": []any{
									map[string]any{"type": "text", "text": "filename.gml"},
								},
							},
						},
					},
				},
			},
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type": "table_cell",
						"content": []any{
							map[string]any{
								"type": "code",
								"content": []any{
									map[string]any{"type": "text", "text": "data.xml"},
								},
							},
						},
					},
				},
			},
		},
	}

	formatTableContent(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "`filename.gml`")
	assert.Contains(t, result, "`data.xml`")
}

func TestIsPlaceholderTable(t *testing.T) {
	tests := []struct {
		name     string
		rows     []any
		expected bool
	}{
		{
			name:     "empty slice",
			rows:     []any{},
			expected: false,
		},
		{
			name: "single row (header only)",
			rows: []any{
				map[string]any{
					"type": "table_row",
					"content": []any{
						map[string]any{
							"type": "table_cell",
							"content": []any{
								map[string]any{"type": "text", "text": "Header"},
							},
						},
					},
				},
			},
			expected: false,
		},
		{
			name: "header with empty body rows",
			rows: []any{
				map[string]any{
					"type": "table_row",
					"content": []any{
						map[string]any{
							"type": "table_cell",
							"content": []any{
								map[string]any{"type": "text", "text": "Header"},
							},
						},
					},
				},
				map[string]any{
					"type": "table_row",
					"content": []any{
						map[string]any{
							"type": "table_cell",
							"content": []any{
								map[string]any{"type": "text", "text": ""},
							},
						},
					},
				},
			},
			expected: true,
		},
		{
			name: "header with content in body rows",
			rows: []any{
				map[string]any{
					"type": "table_row",
					"content": []any{
						map[string]any{
							"type": "table_cell",
							"content": []any{
								map[string]any{"type": "text", "text": "Header"},
							},
						},
					},
				},
				map[string]any{
					"type": "table_row",
					"content": []any{
						map[string]any{
							"type": "table_cell",
							"content": []any{
								map[string]any{"type": "text", "text": "Data"},
							},
						},
					},
				},
			},
			expected: false,
		},
		{
			name: "header with full-width space only body rows",
			rows: []any{
				map[string]any{
					"type": "table_row",
					"content": []any{
						map[string]any{
							"type": "table_cell",
							"content": []any{
								map[string]any{"type": "text", "text": "Header"},
							},
						},
					},
				},
				map[string]any{
					"type": "table_row",
					"content": []any{
						map[string]any{
							"type": "table_cell",
							"content": []any{
								map[string]any{"type": "text", "text": "　"},
							},
						},
					},
				},
			},
			expected: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := isPlaceholderTable(tt.rows)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestExtractCellText(t *testing.T) {
	tests := []struct {
		name     string
		cell     any
		expected string
	}{
		{
			name:     "nil cell",
			cell:     nil,
			expected: "",
		},
		{
			name: "simple text",
			cell: map[string]any{
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{"type": "text", "text": "Hello"},
						},
					},
				},
			},
			expected: "Hello",
		},
		{
			name: "code element",
			cell: map[string]any{
				"content": []any{
					map[string]any{
						"type": "code",
						"content": []any{
							map[string]any{"type": "text", "text": "code_value"},
						},
					},
				},
			},
			expected: "`code_value`",
		},
		{
			name: "nested content",
			cell: map[string]any{
				"content": []any{
					map[string]any{
						"type": "paragraph",
						"content": []any{
							map[string]any{
								"type": "span",
								"content": []any{
									map[string]any{"type": "text", "text": "nested"},
								},
							},
						},
					},
				},
			},
			expected: "nested",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := extractCellText(tt.cell)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestFormatTableFigureContent(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "table",
				"content": []any{
					map[string]any{
						"type": "table_row",
						"content": []any{
							map[string]any{
								"type": "table_cell",
								"content": []any{
									map[string]any{"type": "text", "text": "Test"},
								},
							},
						},
					},
					map[string]any{
						"type": "table_row",
						"content": []any{
							map[string]any{
								"type": "table_cell",
								"content": []any{
									map[string]any{"type": "text", "text": "Data"},
								},
							},
						},
					},
				},
			},
			map[string]any{
				"type": "figCaption",
				"content": []any{
					map[string]any{"type": "text", "text": "Table 1"},
				},
			},
		},
	}

	formatTableFigureContent(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "| Test |")
}
