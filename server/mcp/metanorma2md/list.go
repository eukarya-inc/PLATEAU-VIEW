package metanorma2md

import (
	"strings"
)

// formatListContent handles bullet_list and ordered_list elements
func formatListContent(sb *strings.Builder, content any) {
	if contentMap, ok := content.(map[string]any); ok {
		if items, ok := contentMap["content"].([]any); ok {
			for _, item := range items {
				if itemMap, ok := item.(map[string]any); ok {
					sb.WriteString("- ")
					if children, ok := itemMap["content"].([]any); ok {
						formatListItemChildren(sb, children)
					}
					sb.WriteString("\n")
				}
			}
		}
	}
	sb.WriteString("\n")
}

// formatListItemChildren handles the children of a list item
// Detects pattern: first paragraph is title, rest are description
func formatListItemChildren(sb *strings.Builder, children []any) {
	if len(children) == 0 {
		return
	}

	// Check if we have multiple paragraphs (title + description pattern)
	paragraphs := make([]map[string]any, 0)
	for _, child := range children {
		if childMap, ok := child.(map[string]any); ok {
			childType, _ := childMap["type"].(string)
			if childType == "paragraph" {
				paragraphs = append(paragraphs, childMap)
			}
		}
	}

	if len(paragraphs) >= 2 {
		// Multiple paragraphs: first is title, rest are description
		// Extract title from first paragraph
		var titleSb strings.Builder
		if titleContent, ok := paragraphs[0]["content"].([]any); ok {
			for _, c := range titleContent {
				if cMap, ok := c.(map[string]any); ok {
					formatInlineContent(&titleSb, cMap)
				}
			}
		}
		title := strings.TrimSpace(titleSb.String())

		// Extract description from remaining paragraphs
		var descSb strings.Builder
		for i := 1; i < len(paragraphs); i++ {
			if descContent, ok := paragraphs[i]["content"].([]any); ok {
				for _, c := range descContent {
					if cMap, ok := c.(map[string]any); ok {
						formatInlineContent(&descSb, cMap)
					}
				}
			}
			if i < len(paragraphs)-1 {
				descSb.WriteString(" ")
			}
		}
		desc := strings.TrimSpace(descSb.String())

		// Format as "**Title**：Description"
		if title != "" {
			sb.WriteString("**")
			sb.WriteString(title)
			sb.WriteString("**")
			if desc != "" {
				sb.WriteString("：")
				sb.WriteString(desc)
			}
		}

		// Handle non-paragraph elements (like tables) after paragraphs
		for _, child := range children {
			if childMap, ok := child.(map[string]any); ok {
				childType, _ := childMap["type"].(string)
				if childType != "paragraph" {
					sb.WriteString("\n  ")
					// Format table or other content
					if childType == "tableFigure" {
						formatTableFigureContent(sb, childMap)
					}
				}
			}
		}
	} else {
		// Single paragraph or other structure - just format content
		for _, child := range children {
			if childMap, ok := child.(map[string]any); ok {
				if childContent, ok := childMap["content"].([]any); ok {
					for _, c := range childContent {
						if cMap, ok := c.(map[string]any); ok {
							formatInlineContent(sb, cMap)
						}
					}
				}
			}
		}
	}
}
