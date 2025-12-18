package metanorma2md

import (
	"fmt"
	"strings"
)

// formatTextContent handles simple text content
func formatTextContent(sb *strings.Builder, content any) {
	switch v := content.(type) {
	case string:
		sb.WriteString(v)
	case map[string]any:
		if text, ok := v["text"].(string); ok {
			sb.WriteString(text)
		}
	}
}

// formatParagraphContent handles paragraph elements
func formatParagraphContent(sb *strings.Builder, content any) {
	if contentMap, ok := content.(map[string]any); ok {
		if children, ok := contentMap["content"].([]any); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]any); ok {
					formatInlineContent(sb, childMap)
				}
			}
		}
	}
	sb.WriteString("\n\n")
}

// formatInlineContent handles inline elements like text, links, underline, bold, etc.
func formatInlineContent(sb *strings.Builder, contentMap map[string]any) {
	childType := "text"
	if t, ok := contentMap["type"].(string); ok {
		childType = t
	}

	switch childType {
	case "text":
		if text, ok := contentMap["text"].(string); ok {
			sb.WriteString(text)
		}
	case "linebreak":
		sb.WriteString("\n")
	case "external_link":
		formatExternalLinkContent(sb, contentMap)
	case "code":
		sb.WriteString("`")
		if innerContents, ok := contentMap["content"].([]any); ok {
			for _, inner := range innerContents {
				if innerMap, ok := inner.(map[string]any); ok {
					formatInlineContent(sb, innerMap)
				}
			}
		}
		sb.WriteString("`")
	case "underline", "bold", "italic", "strikethrough", "subscript", "superscript", "strong", "span":
		// These are inline formatting - extract nested content
		if innerContents, ok := contentMap["content"].([]any); ok {
			for _, inner := range innerContents {
				if innerMap, ok := inner.(map[string]any); ok {
					formatInlineContent(sb, innerMap)
				}
			}
		}
	default:
		// Try to extract text from unknown inline types
		if text, ok := contentMap["text"].(string); ok {
			sb.WriteString(text)
		} else if innerContents, ok := contentMap["content"].([]any); ok {
			for _, inner := range innerContents {
				if innerMap, ok := inner.(map[string]any); ok {
					formatInlineContent(sb, innerMap)
				}
			}
		}
	}
}

// formatTitleContent handles title/heading elements
func formatTitleContent(sb *strings.Builder, content any) {
	if contentMap, ok := content.(map[string]any); ok {
		if children, ok := contentMap["content"].([]any); ok {
			var titleText strings.Builder
			for _, child := range children {
				if childMap, ok := child.(map[string]any); ok {
					formatInlineContent(&titleText, childMap)
				}
			}
			title := normalizeTitle(titleText.String())
			_, _ = fmt.Fprintf(sb, "## %s\n\n", title)
		}
	}
}

// formatHeadingContent handles heading elements with level
func formatHeadingContent(sb *strings.Builder, content any) {
	if contentMap, ok := content.(map[string]any); ok {
		level := 2
		if l, ok := contentMap["level"].(float64); ok {
			level = int(l)
		}
		text := ""
		if t, ok := contentMap["text"].(string); ok {
			text = t
		} else if children, ok := contentMap["children"].([]any); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]any); ok {
					if t, ok := childMap["text"].(string); ok {
						text += t
					}
				}
			}
		}
		if text != "" {
			_, _ = fmt.Fprintf(sb, "%s %s\n\n", strings.Repeat("#", level), text)
		}
	}
}

// formatCodeContent handles code blocks
func formatCodeContent(sb *strings.Builder, content any) {
	if contentMap, ok := content.(map[string]any); ok {
		var code strings.Builder
		if children, ok := contentMap["content"].([]any); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]any); ok {
					if text, ok := childMap["text"].(string); ok {
						code.WriteString(text)
					}
				}
			}
		}
		if code.Len() > 0 {
			// Check if multiline
			codeStr := code.String()
			if strings.Contains(codeStr, "\n") {
				_, _ = fmt.Fprintf(sb, "```\n%s\n```\n\n", codeStr)
			} else {
				_, _ = fmt.Fprintf(sb, "`%s`", codeStr)
			}
		}
	}
}

// formatGenericContent handles unknown content types by extracting nested content
func formatGenericContent(sb *strings.Builder, content any, opts *Options) {
	if contentMap, ok := content.(map[string]any); ok {
		// Try to extract text from nested content
		if children, ok := contentMap["content"].([]any); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]any); ok {
					childType := "text"
					if t, ok := childMap["type"].(string); ok {
						childType = t
					}
					formatContent(sb, Content{Type: childType, Content: childMap}, opts)
				}
			}
		} else if text, ok := contentMap["text"].(string); ok {
			sb.WriteString(text)
		}
	}
}

// formatBibitemContent handles bibliography item elements
// Each bibitem is treated like a paragraph with text content and a trailing newline
func formatBibitemContent(sb *strings.Builder, content any) {
	if contentMap, ok := content.(map[string]any); ok {
		if children, ok := contentMap["content"].([]any); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]any); ok {
					formatInlineContent(sb, childMap)
				}
			}
		}
	}
	sb.WriteString("\n\n")
}

// extractTextRecursive extracts all text from a nested content structure
func extractTextRecursive(contentMap map[string]any) string {
	var sb strings.Builder

	if text, ok := contentMap["text"].(string); ok {
		sb.WriteString(text)
	}

	if contents, ok := contentMap["content"].([]any); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				sb.WriteString(extractTextRecursive(cMap))
			}
		}
	}

	return sb.String()
}
