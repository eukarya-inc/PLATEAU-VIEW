package metanorma2md

import (
	"fmt"
	"strings"
)

// formatTableContent handles table elements
func formatTableContent(sb *strings.Builder, content any) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		return
	}

	// Handle table with table_row structure (PLATEAU spec API format)
	if rows, ok := contentMap["content"].([]any); ok {
		// First pass: check if table is a placeholder (mostly empty)
		if isPlaceholderTable(rows) {
			sb.WriteString("*[テンプレート用テーブル - 拡張製品仕様書で記入]*\n\n")
			return
		}

		for i, row := range rows {
			if rowMap, ok := row.(map[string]any); ok {
				if rowMap["type"] == "table_row" {
					cells, ok := rowMap["content"].([]any)
					if !ok {
						continue
					}
					sb.WriteString("|")
					for _, cell := range cells {
						cellText := extractCellText(cell)
						_, _ = fmt.Fprintf(sb, " %s |", cellText)
					}
					sb.WriteString("\n")
					// Add header separator after first row
					if i == 0 {
						sb.WriteString("|")
						for range cells {
							sb.WriteString(" --- |")
						}
						sb.WriteString("\n")
					}
				}
			}
		}
		sb.WriteString("\n")
		return
	}

	// Fallback for simple rows format
	if rows, ok := contentMap["rows"].([]any); ok {
		for i, row := range rows {
			if rowData, ok := row.([]any); ok {
				sb.WriteString("|")
				for _, cell := range rowData {
					_, _ = fmt.Fprintf(sb, " %v |", cell)
				}
				sb.WriteString("\n")
				if i == 0 {
					sb.WriteString("|")
					for range rowData {
						sb.WriteString(" --- |")
					}
					sb.WriteString("\n")
				}
			}
		}
		sb.WriteString("\n")
	}
}

// extractCellText extracts text from a table cell
func extractCellText(cell any) string {
	cellMap, ok := cell.(map[string]any)
	if !ok {
		return ""
	}

	contents, ok := cellMap["content"].([]any)
	if !ok {
		return ""
	}

	var sb strings.Builder
	for _, c := range contents {
		cMap, ok := c.(map[string]any)
		if !ok {
			continue
		}
		extractCellTextRecursive(&sb, cMap)
	}
	return sb.String()
}

// extractCellTextRecursive recursively extracts text from nested content
func extractCellTextRecursive(sb *strings.Builder, node map[string]any) {
	nodeType, _ := node["type"].(string)

	// Handle text node
	if text, ok := node["text"].(string); ok {
		sb.WriteString(text)
		return
	}

	// Handle code node - wrap in backticks
	if nodeType == "code" {
		sb.WriteString("`")
		if contents, ok := node["content"].([]any); ok {
			for _, c := range contents {
				if cMap, ok := c.(map[string]any); ok {
					extractCellTextRecursive(sb, cMap)
				}
			}
		}
		sb.WriteString("`")
		return
	}

	// Recursively handle other nodes with content
	if contents, ok := node["content"].([]any); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				extractCellTextRecursive(sb, cMap)
			}
		}
	}
}

// isPlaceholderTable checks if a table is a placeholder (mostly empty cells)
// Placeholder tables have only the header row with content, and body rows are empty or contain only whitespace
func isPlaceholderTable(rows []any) bool {
	if len(rows) < 2 {
		return false // Need at least header + 1 body row
	}

	// Check body rows (skip header row at index 0)
	emptyBodyRows := 0
	totalBodyRows := 0

	for i, row := range rows {
		if i == 0 {
			continue // Skip header row
		}
		rowMap, ok := row.(map[string]any)
		if !ok || rowMap["type"] != "table_row" {
			continue
		}

		cells, ok := rowMap["content"].([]any)
		if !ok {
			continue
		}

		totalBodyRows++
		allCellsEmpty := true
		for _, cell := range cells {
			text := extractCellText(cell)
			// Check if cell is empty or contains only whitespace (including full-width space)
			trimmed := strings.TrimSpace(text)
			trimmed = strings.ReplaceAll(trimmed, "　", "") // Remove full-width spaces
			if trimmed != "" {
				allCellsEmpty = false
				break
			}
		}
		if allCellsEmpty {
			emptyBodyRows++
		}
	}

	// If all body rows are empty, it's a placeholder table
	return totalBodyRows > 0 && emptyBodyRows == totalBodyRows
}

// formatTableFigureContent handles tableFigure wrapper
func formatTableFigureContent(sb *strings.Builder, content any) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		return
	}

	// tableFigure > content > table
	if contents, ok := contentMap["content"].([]any); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				cType, _ := cMap["type"].(string)
				if cType == "table" {
					formatTableContent(sb, cMap)
				}
				// Skip figCaption - we don't need table captions in markdown
			}
		}
	}
}
