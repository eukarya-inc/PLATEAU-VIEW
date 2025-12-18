package metanorma2md

import (
	"fmt"
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

	var label, term string
	var definitionParagraphs []string

	for _, c := range contents {
		cMap, ok := c.(map[string]any)
		if !ok {
			continue
		}

		cType, _ := cMap["type"].(string)

		switch cType {
		case "termXrefLabel":
			// Extract label like "1.5.1"
			label = extractTermText(cMap)
		case "term":
			// Extract term like "3D都市モデル"
			term = extractTermText(cMap)
		case "definition":
			// Extract definition paragraphs
			definitionParagraphs = extractDefinitionParagraphs(cMap)
		}
	}

	// Format as heading with definition
	if label != "" && term != "" {
		_, _ = fmt.Fprintf(sb, "### %s %s\n\n", label, term)
	} else if term != "" {
		_, _ = fmt.Fprintf(sb, "### %s\n\n", term)
	}

	for _, para := range definitionParagraphs {
		sb.WriteString(para)
		sb.WriteString("\n\n")
	}
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

// extractDefinitionParagraphs extracts definition paragraphs from definition element
func extractDefinitionParagraphs(contentMap map[string]any) []string {
	var paragraphs []string

	if contents, ok := contentMap["content"].([]any); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				cType, _ := cMap["type"].(string)
				if cType == "paragraph" {
					// Extract paragraph content
					var sb strings.Builder
					if innerContents, ok := cMap["content"].([]any); ok {
						for _, inner := range innerContents {
							if innerMap, ok := inner.(map[string]any); ok {
								formatInlineContent(&sb, innerMap)
							}
						}
					}
					result := strings.TrimSpace(sb.String())
					if result != "" {
						paragraphs = append(paragraphs, result)
					}
				}
			}
		}
	}

	return paragraphs
}
