package metanorma2md

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestConvert_NilDocument(t *testing.T) {
	result := Convert(nil, nil)
	assert.Empty(t, result)
}

func TestConvert_EmptyDocument(t *testing.T) {
	doc := &Document{}
	result := Convert(doc, nil)
	assert.Empty(t, result)
}

func TestConvert_WithTitle(t *testing.T) {
	doc := &Document{
		Title: "Test Title",
	}
	result := Convert(doc, nil)
	assert.Equal(t, "# Test Title\n\n", result)
}

func TestConvert_TitleNormalization(t *testing.T) {
	tests := []struct {
		name     string
		title    string
		expected string
	}{
		{
			name:     "tabs to spaces",
			title:    "1.1\tIntroduction",
			expected: "# 1.1 Introduction\n\n",
		},
		{
			name:     "multiple spaces collapsed",
			title:    "1.1   Introduction",
			expected: "# 1.1 Introduction\n\n",
		},
		{
			name:     "trim whitespace",
			title:    "  Title  ",
			expected: "# Title\n\n",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			doc := &Document{Title: tt.title}
			result := Convert(doc, nil)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestConvert_WithParagraph(t *testing.T) {
	doc := &Document{
		Content: []Content{
			{
				Type: "paragraph",
				Content: map[string]any{
					"content": []any{
						map[string]any{
							"type": "text",
							"text": "This is a paragraph.",
						},
					},
				},
			},
		},
	}
	result := Convert(doc, nil)
	assert.Equal(t, "This is a paragraph.\n\n", result)
}

func TestConvert_WithTable(t *testing.T) {
	doc := &Document{
		Content: []Content{
			{
				Type: "table",
				Content: map[string]any{
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
												map[string]any{
													"type": "text",
													"text": "Header 1",
												},
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
												map[string]any{
													"type": "text",
													"text": "Header 2",
												},
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
												map[string]any{
													"type": "text",
													"text": "Cell 1",
												},
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
												map[string]any{
													"type": "text",
													"text": "Cell 2",
												},
											},
										},
									},
								},
							},
						},
					},
				},
			},
		},
	}
	result := Convert(doc, nil)
	assert.Contains(t, result, "| Header 1 |")
	assert.Contains(t, result, "| --- |")
	assert.Contains(t, result, "| Cell 1 |")
}

func TestConvert_WithList(t *testing.T) {
	doc := &Document{
		Content: []Content{
			{
				Type: "bullet_list",
				Content: map[string]any{
					"content": []any{
						map[string]any{
							"type": "list_item",
							"content": []any{
								map[string]any{
									"type": "paragraph",
									"content": []any{
										map[string]any{
											"type": "text",
											"text": "Item 1",
										},
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
										map[string]any{
											"type": "text",
											"text": "Item 2",
										},
									},
								},
							},
						},
					},
				},
			},
		},
	}
	result := Convert(doc, nil)
	assert.Contains(t, result, "- Item 1")
	assert.Contains(t, result, "- Item 2")
}

func TestConvert_WithExternalLink(t *testing.T) {
	doc := &Document{
		Content: []Content{
			{
				Type: "external_link",
				Content: map[string]any{
					"attrs": map[string]any{
						"href": "https://example.com",
					},
					"content": []any{
						map[string]any{
							"text": "Example",
						},
					},
				},
			},
		},
	}
	result := Convert(doc, nil)
	assert.Contains(t, result, "[Example](https://example.com)")
}

func TestConvert_WithImage_IncludeImages(t *testing.T) {
	doc := &Document{
		Content: []Content{
			{
				Type: "image",
				Content: map[string]any{
					"attrs": map[string]any{
						"src": "data:image/png;base64,abc123",
						"alt": "Test Image",
					},
				},
			},
		},
	}

	// With include images
	result := Convert(doc, &Options{IncludeImages: true})
	assert.Contains(t, result, "![Test Image](data:image/png;base64,abc123)")

	// Without include images (placeholder)
	result = Convert(doc, &Options{IncludeImages: false})
	assert.Contains(t, result, "*[Image]*")
}

func TestConvert_WithCode(t *testing.T) {
	doc := &Document{
		Content: []Content{
			{
				Type: "code",
				Content: map[string]any{
					"content": []any{
						map[string]any{
							"type": "text",
							"text": "console.log('hello')",
						},
					},
				},
			},
		},
	}
	result := Convert(doc, nil)
	assert.Contains(t, result, "`console.log('hello')`")
}

func TestConvert_WithHeading(t *testing.T) {
	doc := &Document{
		Content: []Content{
			{
				Type: "heading",
				Content: map[string]any{
					"level": float64(3),
					"text":  "Section Title",
				},
			},
		},
	}
	result := Convert(doc, nil)
	assert.Equal(t, "### Section Title\n\n", result)
}

func TestConvert_WithTitle_Content(t *testing.T) {
	doc := &Document{
		Content: []Content{
			{
				Type: "title",
				Content: map[string]any{
					"content": []any{
						map[string]any{
							"type": "text",
							"text": "7.2.3\tファイル名称",
						},
					},
				},
			},
		},
	}
	result := Convert(doc, nil)
	assert.Contains(t, result, "## 7.2.3 ファイル名称")
}
