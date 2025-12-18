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

func TestFormatTableContent_WithRowspan(t *testing.T) {
	var sb strings.Builder
	// Table like:
	// | Col1 | Col2 |
	// |------|------|
	// | A    | X    |  <- X has rowspan=3
	// | B    | ↑    |
	// | C    | ↑    |
	content := map[string]any{
		"content": []any{
			// Header row
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Col1"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Col2"}},
					},
				},
			},
			// Data row 1 - with rowspan=3 on second cell
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "A"}},
					},
					map[string]any{
						"type":    "table_cell",
						"attrs":   map[string]any{"rowspan": float64(3)},
						"content": []any{map[string]any{"type": "text", "text": "X"}},
					},
				},
			},
			// Data row 2 - missing second cell due to rowspan
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "B"}},
					},
				},
			},
			// Data row 3 - missing second cell due to rowspan
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "C"}},
					},
				},
			},
		},
	}

	formatTableContent(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "| Col1 | Col2 |")
	assert.Contains(t, result, "| A | X |")
	assert.Contains(t, result, "| B | ↑ |")
	assert.Contains(t, result, "| C | ↑ |")
}

func TestFormatTableContent_WithEmptyCells(t *testing.T) {
	var sb strings.Builder
	// Table with empty cells - empty cells stay empty (not treated as rowspan)
	// | Col1 | Col2 |
	// |------|------|
	// | A    | X    |
	// | B    |      |  <- empty cell stays empty
	// | C    |      |
	content := map[string]any{
		"content": []any{
			// Header row
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Col1"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Col2"}},
					},
				},
			},
			// Data row 1
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "A"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "X"}},
					},
				},
			},
			// Data row 2 - empty second cell
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "B"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": ""}},
					},
				},
			},
			// Data row 3 - empty second cell
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "C"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": ""}},
					},
				},
			},
		},
	}

	formatTableContent(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "| Col1 | Col2 |")
	assert.Contains(t, result, "| A | X |")
	assert.Contains(t, result, "| B |  |")
	assert.Contains(t, result, "| C |  |")
}

func TestGetCellRowspan(t *testing.T) {
	tests := []struct {
		name     string
		cell     any
		expected int
	}{
		{
			name:     "nil cell",
			cell:     nil,
			expected: 1,
		},
		{
			name:     "no attrs",
			cell:     map[string]any{"type": "table_cell"},
			expected: 1,
		},
		{
			name: "rowspan=1 (float64)",
			cell: map[string]any{
				"attrs": map[string]any{"rowspan": float64(1)},
			},
			expected: 1,
		},
		{
			name: "rowspan=3 (float64)",
			cell: map[string]any{
				"attrs": map[string]any{"rowspan": float64(3)},
			},
			expected: 3,
		},
		{
			name: "rowspan=5 (string)",
			cell: map[string]any{
				"attrs": map[string]any{"rowspan": "5"},
			},
			expected: 5,
		},
		{
			name: "rowspan=null (string)",
			cell: map[string]any{
				"attrs": map[string]any{"rowspan": "null"},
			},
			expected: 1,
		},
		{
			name: "rowspan empty string",
			cell: map[string]any{
				"attrs": map[string]any{"rowspan": ""},
			},
			expected: 1,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := getCellRowspan(tt.cell)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestFormatTableContent_WithImage(t *testing.T) {
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Header"}},
					},
				},
			},
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type": "table_cell",
						"content": []any{
							map[string]any{"type": "text", "text": "Text before "},
							map[string]any{
								"type": "image",
								"attrs": map[string]any{
									"src": "data:image/png;base64,xxxx",
									"alt": "test image",
								},
							},
							map[string]any{"type": "text", "text": " text after"},
						},
					},
				},
			},
		},
	}

	t.Run("without IncludeImages", func(t *testing.T) {
		var sb strings.Builder
		formatTableContentWithOpts(&sb, content, nil)
		result := sb.String()

		assert.Contains(t, result, "| Header |")
		assert.Contains(t, result, "Text before [Image] text after")
	})

	t.Run("with IncludeImages", func(t *testing.T) {
		var sb strings.Builder
		formatTableContentWithOpts(&sb, content, &Options{IncludeImages: true})
		result := sb.String()

		assert.Contains(t, result, "| Header |")
		assert.Contains(t, result, "![test image](data:image/png;base64,xxxx)")
	})
}

func TestFormatTableContent_WithColspan(t *testing.T) {
	var sb strings.Builder
	// Table like:
	// | Col1       | Col2 | Col3 |
	// |------------|------|------|
	// | A (colspan=2)     | X    |
	// | B          | C    | Y    |
	content := map[string]any{
		"content": []any{
			// Header row
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Col1"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Col2"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Col3"}},
					},
				},
			},
			// Data row 1 - with colspan=2 on first cell
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"attrs":   map[string]any{"colspan": "2"}, // string format like in PLATEAU API
						"content": []any{map[string]any{"type": "text", "text": "A"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "X"}},
					},
				},
			},
			// Data row 2 - normal row
			map[string]any{
				"type": "table_row",
				"content": []any{
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "B"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "C"}},
					},
					map[string]any{
						"type":    "table_cell",
						"content": []any{map[string]any{"type": "text", "text": "Y"}},
					},
				},
			},
		},
	}

	formatTableContent(&sb, content)
	result := sb.String()

	assert.Contains(t, result, "| Col1 | Col2 | Col3 |")
	assert.Contains(t, result, "| A | ← | X |")
	assert.Contains(t, result, "| B | C | Y |")
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
