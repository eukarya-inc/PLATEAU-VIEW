package metanorma2md

import (
	"fmt"
	"regexp"
	"strings"
)

// formatTermWithDefinition handles termWithDefinition elements
// These are used for glossary/terminology entries
func formatTermWithDefinition(sb *strings.Builder, content any) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		return
	}

	contents, ok := contentMap["content"].([]any)
	if !ok {
		return
	}

	var label, term, definition string

	for _, c := range contents {
		cMap, ok := c.(map[string]any)
		if !ok {
			continue
		}

		cType, _ := cMap["type"].(string)

		switch cType {
		case "termLabel":
			// Extract label like "1.5.1"
			label = extractTermText(cMap)
		case "term":
			// Extract term like "3D都市モデル"
			term = extractTermText(cMap)
		case "termDefinition":
			// Extract definition text
			definition = extractTermDefinition(cMap)
		}
	}

	// Format as bold header with definition
	if label != "" && term != "" {
		_, _ = fmt.Fprintf(sb, "**%s %s**\n", label, term)
	} else if term != "" {
		_, _ = fmt.Fprintf(sb, "**%s**\n", term)
	}

	if definition != "" {
		sb.WriteString(definition)
		sb.WriteString("\n")
	}
	sb.WriteString("\n")
}

// extractTermText extracts text from term or termLabel elements
func extractTermText(contentMap map[string]any) string {
	var sb strings.Builder

	if contents, ok := contentMap["content"].([]any); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				cType, _ := cMap["type"].(string)
				if cType == "text" {
					if text, ok := cMap["text"].(string); ok {
						sb.WriteString(text)
					}
				} else if cType == "paragraph" || cType == "span" {
					// Recursively extract from nested elements
					sb.WriteString(extractTermText(cMap))
				} else if innerContents, ok := cMap["content"].([]any); ok {
					for _, inner := range innerContents {
						if innerMap, ok := inner.(map[string]any); ok {
							if text, ok := innerMap["text"].(string); ok {
								sb.WriteString(text)
							}
						}
					}
				}
			}
		}
	}

	return strings.TrimSpace(sb.String())
}

// extractTermDefinition extracts definition text from termDefinition element
func extractTermDefinition(contentMap map[string]any) string {
	var sb strings.Builder

	if contents, ok := contentMap["content"].([]any); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				cType, _ := cMap["type"].(string)
				if cType == "paragraph" {
					// Extract paragraph content
					if innerContents, ok := cMap["content"].([]any); ok {
						for _, inner := range innerContents {
							if innerMap, ok := inner.(map[string]any); ok {
								innerType, _ := innerMap["type"].(string)
								switch innerType {
								case "text":
									if text, ok := innerMap["text"].(string); ok {
										sb.WriteString(text)
									}
								case "external_link":
									// Handle links in definition
									var linkSb strings.Builder
									formatExternalLinkContent(&linkSb, innerMap)
									sb.WriteString(linkSb.String())
								default:
									// Recursively extract other content
									sb.WriteString(extractTextRecursive(innerMap))
								}
							}
						}
					}
				}
			}
		}
	}

	result := strings.TrimSpace(sb.String())
	// Clean up extra whitespace
	re := regexp.MustCompile(`\s+`)
	result = re.ReplaceAllString(result, " ")
	return result
}
