package metanorma2md

import (
	"fmt"
	"strings"
)

// formatExternalLinkContent handles external link elements
func formatExternalLinkContent(sb *strings.Builder, content any) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		return
	}

	href := ""
	if attrs, ok := contentMap["attrs"].(map[string]any); ok {
		if h, ok := attrs["href"].(string); ok {
			href = h
		}
	}

	// Extract link text from content
	var linkText strings.Builder
	if children, ok := contentMap["content"].([]any); ok {
		for _, child := range children {
			if childMap, ok := child.(map[string]any); ok {
				if text, ok := childMap["text"].(string); ok {
					linkText.WriteString(text)
				} else if innerContents, ok := childMap["content"].([]any); ok {
					for _, inner := range innerContents {
						if innerMap, ok := inner.(map[string]any); ok {
							if text, ok := innerMap["text"].(string); ok {
								linkText.WriteString(text)
							}
						}
					}
				}
			}
		}
	}

	text := linkText.String()
	if text == "" {
		text = href
	}

	if href != "" {
		_, _ = fmt.Fprintf(sb, "[%s](%s)", text, href)
	} else {
		sb.WriteString(text)
	}
}

// formatImageContent handles image elements
func formatImageContent(sb *strings.Builder, content any, opts *Options) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		return
	}

	// Get image source from attrs
	src := ""
	alt := ""
	if attrs, ok := contentMap["attrs"].(map[string]any); ok {
		if s, ok := attrs["src"].(string); ok {
			src = s
		}
		if a, ok := attrs["alt"].(string); ok {
			alt = a
		}
	}

	// Check if this is a base64 image
	isBase64 := strings.HasPrefix(src, "data:")

	if isBase64 && !opts.IncludeImages {
		// Show placeholder for base64 images when not including them
		sb.WriteString("*[Image]*\n\n")
		return
	}

	if src != "" {
		if alt == "" {
			alt = "Image"
		}
		_, _ = fmt.Fprintf(sb, "![%s](%s)\n\n", alt, src)
	}
}

// formatFigureContent handles figure elements (which may contain images)
func formatFigureContent(sb *strings.Builder, content any, opts *Options) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		return
	}

	// figure > content > image
	if contents, ok := contentMap["content"].([]any); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				cType, _ := cMap["type"].(string)
				switch cType {
				case "image":
					formatImageContent(sb, cMap, opts)
				case "figCaption":
					// Skip figure captions for now
				default:
					// Try to process other content types
					if innerContents, ok := cMap["content"].([]any); ok {
						for _, inner := range innerContents {
							if innerMap, ok := inner.(map[string]any); ok {
								innerType, _ := innerMap["type"].(string)
								if innerType == "image" {
									formatImageContent(sb, innerMap, opts)
								}
							}
						}
					}
				}
			}
		}
	}
}
