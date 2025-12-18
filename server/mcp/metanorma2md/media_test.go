package metanorma2md

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFormatExternalLinkContent(t *testing.T) {
	tests := []struct {
		name     string
		content  any
		expected string
	}{
		{
			name: "with href and text",
			content: map[string]any{
				"attrs": map[string]any{
					"href": "https://example.com",
				},
				"content": []any{
					map[string]any{"text": "Example"},
				},
			},
			expected: "[Example](https://example.com)",
		},
		{
			name: "with nested content",
			content: map[string]any{
				"attrs": map[string]any{
					"href": "https://example.com",
				},
				"content": []any{
					map[string]any{
						"content": []any{
							map[string]any{"text": "Nested"},
						},
					},
				},
			},
			expected: "[Nested](https://example.com)",
		},
		{
			name: "no href",
			content: map[string]any{
				"content": []any{
					map[string]any{"text": "Just text"},
				},
			},
			expected: "Just text",
		},
		{
			name: "empty text uses href",
			content: map[string]any{
				"attrs": map[string]any{
					"href": "https://example.com/path",
				},
				"content": []any{},
			},
			expected: "[https://example.com/path](https://example.com/path)",
		},
		{
			name:     "nil content",
			content:  nil,
			expected: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var sb strings.Builder
			formatExternalLinkContent(&sb, tt.content)
			assert.Equal(t, tt.expected, sb.String())
		})
	}
}

func TestFormatImageContent(t *testing.T) {
	tests := []struct {
		name     string
		content  any
		opts     *Options
		contains string
	}{
		{
			name: "regular image",
			content: map[string]any{
				"attrs": map[string]any{
					"src": "https://example.com/image.png",
					"alt": "Example Image",
				},
			},
			opts:     nil,
			contains: "![Example Image](https://example.com/image.png)",
		},
		{
			name: "base64 image with include",
			content: map[string]any{
				"attrs": map[string]any{
					"src": "data:image/png;base64,abc123",
					"alt": "Base64 Image",
				},
			},
			opts:     &Options{IncludeImages: true},
			contains: "![Base64 Image](data:image/png;base64,abc123)",
		},
		{
			name: "base64 image without include",
			content: map[string]any{
				"attrs": map[string]any{
					"src": "data:image/png;base64,abc123",
					"alt": "Base64 Image",
				},
			},
			opts:     &Options{IncludeImages: false},
			contains: "*[Image]*",
		},
		{
			name: "default alt text",
			content: map[string]any{
				"attrs": map[string]any{
					"src": "https://example.com/image.png",
				},
			},
			opts:     nil,
			contains: "![Image](https://example.com/image.png)",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var sb strings.Builder
			formatImageContent(&sb, tt.content, tt.opts)
			assert.Contains(t, sb.String(), tt.contains)
		})
	}
}

func TestFormatFigureContent(t *testing.T) {
	var sb strings.Builder
	content := map[string]any{
		"content": []any{
			map[string]any{
				"type": "image",
				"attrs": map[string]any{
					"src": "https://example.com/fig.png",
					"alt": "Figure Image",
				},
			},
			map[string]any{
				"type": "figCaption",
			},
		},
	}

	formatFigureContent(&sb, content, nil)
	assert.Contains(t, sb.String(), "![Figure Image](https://example.com/fig.png)")
}

func TestFormatFigureContent_Nil(t *testing.T) {
	var sb strings.Builder
	formatFigureContent(&sb, nil, nil)
	assert.Empty(t, sb.String())
}

func TestFormatFigureContent_InvalidType(t *testing.T) {
	var sb strings.Builder
	formatFigureContent(&sb, "not a map", nil)
	assert.Empty(t, sb.String())
}
