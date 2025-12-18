package metanorma2md

import (
	"fmt"
	"strings"
)

// formatTableContent handles table elements
func formatTableContent(sb *strings.Builder, content any) {
	formatTableContentWithOpts(sb, content, nil)
}

// formatTableContentWithOpts handles table elements with options
func formatTableContentWithOpts(sb *strings.Builder, content any, opts *Options) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		return
	}

	// Handle table with table_row structure (PLATEAU spec API format)
	if rows, ok := contentMap["content"].([]any); ok {
		// Track rowspan for each column: remaining rows to span
		var colSpans []int // remaining rowspan count for each column
		var numCols int    // expected number of columns

		for i, row := range rows {
			if rowMap, ok := row.(map[string]any); ok {
				if rowMap["type"] == "table_row" {
					cells, ok := rowMap["content"].([]any)
					if !ok {
						continue
					}

					// Initialize column tracking on first row
					if i == 0 {
						numCols = countEffectiveColumns(cells)
						colSpans = make([]int, numCols)
					}

					sb.WriteString("|")
					cellIdx := 0
					for col := 0; col < numCols; col++ {
						if colSpans[col] > 0 {
							// This column is still being spanned from above (explicit rowspan)
							_, _ = fmt.Fprintf(sb, " ↑ |")
							colSpans[col]--
						} else if cellIdx < len(cells) {
							cell := cells[cellIdx]
							cellText := extractCellTextWithOpts(cell, opts)
							rowspan := getCellRowspan(cell)
							colspan := getCellColspan(cell)

							// Output cell content (empty cells stay empty)
							_, _ = fmt.Fprintf(sb, " %s |", cellText)

							// Track explicit rowspan for subsequent rows
							if rowspan > 1 {
								colSpans[col] = rowspan - 1
							}

							// Handle colspan: output "←" for additional columns
							for c := 1; c < colspan; c++ {
								col++
								if col < numCols {
									_, _ = fmt.Fprintf(sb, " ← |")
								}
							}

							cellIdx++
						} else {
							// Missing cell - output empty
							_, _ = fmt.Fprintf(sb, "  |")
						}
					}
					sb.WriteString("\n")

					// Add header separator after first row
					if i == 0 {
						sb.WriteString("|")
						for c := 0; c < numCols; c++ {
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

// countEffectiveColumns counts the effective number of columns in a row,
// taking colspan into account
func countEffectiveColumns(cells []any) int {
	count := 0
	for _, cell := range cells {
		colspan := getCellColspan(cell)
		if colspan < 1 {
			colspan = 1
		}
		count += colspan
	}
	return count
}

// getCellRowspan gets the rowspan attribute from a cell (default 1)
func getCellRowspan(cell any) int {
	cellMap, ok := cell.(map[string]any)
	if !ok {
		return 1
	}
	attrs, ok := cellMap["attrs"].(map[string]any)
	if !ok {
		return 1
	}
	// Handle both float64 (from JSON number) and string (from JSON string)
	switch v := attrs["rowspan"].(type) {
	case float64:
		if v > 1 {
			return int(v)
		}
	case string:
		if v != "" && v != "null" {
			var rowspan int
			if _, err := fmt.Sscanf(v, "%d", &rowspan); err == nil && rowspan > 1 {
				return rowspan
			}
		}
	}
	return 1
}

// getCellColspan gets the colspan attribute from a cell (default 1)
func getCellColspan(cell any) int {
	cellMap, ok := cell.(map[string]any)
	if !ok {
		return 1
	}
	attrs, ok := cellMap["attrs"].(map[string]any)
	if !ok {
		return 1
	}
	// Handle both float64 (from JSON number) and string (from JSON string)
	switch v := attrs["colspan"].(type) {
	case float64:
		if v > 1 {
			return int(v)
		}
	case string:
		if v != "" && v != "null" {
			var colspan int
			if _, err := fmt.Sscanf(v, "%d", &colspan); err == nil && colspan > 1 {
				return colspan
			}
		}
	}
	return 1
}

// extractCellText extracts text from a table cell
func extractCellText(cell any) string {
	return extractCellTextWithOpts(cell, nil)
}

// extractCellTextWithOpts extracts text from a table cell with options
func extractCellTextWithOpts(cell any, opts *Options) string {
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
		extractCellTextRecursive(&sb, cMap, opts)
	}
	return sb.String()
}

// extractCellTextRecursive recursively extracts text from nested content
// using the same dispatch pattern as formatContent but optimized for table cells
func extractCellTextRecursive(sb *strings.Builder, node map[string]any, opts *Options) {
	nodeType, _ := node["type"].(string)

	// Handle text node directly
	if text, ok := node["text"].(string); ok {
		sb.WriteString(text)
		return
	}

	// Dispatch based on type - reuse formatContent logic via formatContentForCell
	formatContentForCell(sb, Content{Type: nodeType, Content: node}, opts)
}

// ImagePlaceholder is the placeholder text for images when IncludeImages is false
const ImagePlaceholder = "[Image]"

// formatContentForCell formats content for table cells (strips newlines)
func formatContentForCell(sb *strings.Builder, content Content, opts *Options) {
	var cellSb strings.Builder

	switch content.Type {
	case "text":
		if contentMap, ok := content.Content.(map[string]any); ok {
			if text, ok := contentMap["text"].(string); ok {
				cellSb.WriteString(text)
			}
		}
	case "code":
		cellSb.WriteString("`")
		if contentMap, ok := content.Content.(map[string]any); ok {
			if contents, ok := contentMap["content"].([]any); ok {
				for _, c := range contents {
					if cMap, ok := c.(map[string]any); ok {
						extractCellTextRecursive(&cellSb, cMap, opts)
					}
				}
			}
		}
		cellSb.WriteString("`")
	case "image":
		formatImageForCell(&cellSb, content.Content, opts)
	case "figure":
		// Process figure content (may contain images)
		if contentMap, ok := content.Content.(map[string]any); ok {
			if contents, ok := contentMap["content"].([]any); ok {
				for _, c := range contents {
					if cMap, ok := c.(map[string]any); ok {
						childType, _ := cMap["type"].(string)
						formatContentForCell(&cellSb, Content{Type: childType, Content: cMap}, opts)
					}
				}
			}
		}
	case "external_link":
		// Format as markdown link
		var linkSb strings.Builder
		formatExternalLinkContent(&linkSb, content.Content)
		cellSb.WriteString(linkSb.String())
	default:
		// Recursively process children
		if contentMap, ok := content.Content.(map[string]any); ok {
			if contents, ok := contentMap["content"].([]any); ok {
				for _, c := range contents {
					if cMap, ok := c.(map[string]any); ok {
						extractCellTextRecursive(&cellSb, cMap, opts)
					}
				}
			}
		}
	}

	// Write result, stripping excessive newlines for table cells
	result := cellSb.String()
	result = strings.ReplaceAll(result, "\n\n", " ")
	result = strings.ReplaceAll(result, "\n", " ")
	sb.WriteString(result)
}

// formatImageForCell formats an image for table cells
func formatImageForCell(sb *strings.Builder, content any, opts *Options) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		sb.WriteString(ImagePlaceholder)
		return
	}

	src := ""
	alt := "Image"
	if attrs, ok := contentMap["attrs"].(map[string]any); ok {
		if s, ok := attrs["src"].(string); ok {
			src = s
		}
		if a, ok := attrs["alt"].(string); ok && a != "" {
			alt = a
		}
	}

	if src == "" {
		sb.WriteString(ImagePlaceholder)
		return
	}

	// Check if we should include images
	if opts != nil && opts.IncludeImages {
		_, _ = fmt.Fprintf(sb, "![%s](%s)", alt, src)
	} else {
		sb.WriteString(ImagePlaceholder)
	}
}

// formatTableFigureContent handles tableFigure wrapper
func formatTableFigureContent(sb *strings.Builder, content any) {
	formatTableFigureContentWithOpts(sb, content, nil)
}

// formatTableFigureContentWithOpts handles tableFigure wrapper with options
func formatTableFigureContentWithOpts(sb *strings.Builder, content any, opts *Options) {
	contentMap, ok := content.(map[string]any)
	if !ok {
		return
	}

	// tableFigure > content > (figCaption + table)
	if contents, ok := contentMap["content"].([]any); ok {
		// First pass: find and output figCaption (table name)
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				cType, _ := cMap["type"].(string)
				if cType == "figCaption" {
					caption := extractFigCaption(cMap)
					if caption != "" {
						_, _ = fmt.Fprintf(sb, "**表 %s**\n\n", caption)
					}
				}
			}
		}
		// Second pass: output table
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				cType, _ := cMap["type"].(string)
				if cType == "table" {
					formatTableContentWithOpts(sb, cMap, opts)
				}
			}
		}
	}
}

// extractFigCaption extracts text from figCaption element
func extractFigCaption(content map[string]any) string {
	if contents, ok := content["content"].([]any); ok {
		var result strings.Builder
		for _, c := range contents {
			if cMap, ok := c.(map[string]any); ok {
				if text, ok := cMap["text"].(string); ok {
					result.WriteString(text)
				}
			}
		}
		return result.String()
	}
	return ""
}
