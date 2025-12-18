package plateauspecmcp

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// RegisterTools registers PLATEAU specification reading tools
func RegisterTools(s *server.MCPServer) {
	s.AddTool(specOutlineTool, HandleSpecOutline)
	s.AddTool(specReadTool, HandleSpecRead)
}

var specOutlineTool = mcp.NewTool("plateau_spec_outline",
	mcp.WithDescription("Get the table of contents (outline) of PLATEAU 3D City Model specification documents. Returns a hierarchical structure of chapters and sections with their paths. Use these paths with plateau_spec_read to get the actual content."),
	mcp.WithReadOnlyHintAnnotation(true),
	mcp.WithString("document_type",
		mcp.Description("Document type: 'standard' (3D City Model Standard Product Specification, default) or 'procedure' (3D City Model Standard Work Procedures)"),
	),
	mcp.WithNumber("depth",
		mcp.Description("How deep to fetch the outline hierarchy (1=chapters only, 2=chapters+sections, 3=deeper). Default: 2. Use 1 for faster response."),
	),
	mcp.WithString("chapter",
		mcp.Description("Fetch outline for a specific chapter only (e.g., 'toc4' for data structure chapter). Faster than fetching entire outline."),
	),
	mcp.WithString("format",
		mcp.Description("Output format: 'markdown' (default) or 'json'"),
	),
)

var specReadTool = mcp.NewTool("plateau_spec_read",
	mcp.WithDescription("Read the full content of a section from PLATEAU specification documents, including all child pages. Use the path obtained from plateau_spec_outline. For example, '/plateaudocument/toc1' returns the entire Chapter 1 content."),
	mcp.WithReadOnlyHintAnnotation(true),
	mcp.WithString("path",
		mcp.Required(),
		mcp.Description("The path to read (e.g., '/plateaudocument/toc1' for Chapter 1, '/plateaudocument/toc4' for Chapter 4). Get available paths from plateau_spec_outline."),
	),
	mcp.WithString("document_type",
		mcp.Description("Document type: 'standard' (default) or 'procedure'"),
	),
	mcp.WithBoolean("single_page",
		mcp.Description("If true, only read the specified page without child pages. Default: false (includes children)"),
	),
	mcp.WithBoolean("include_images",
		mcp.Description("If true, include base64-encoded images in markdown output. Default: false (shows placeholder only)"),
	),
)

// HandleSpecOutline handles the plateau_spec_outline tool
func HandleSpecOutline(_ context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	docType := request.GetString("document_type", "standard")
	format := request.GetString("format", "markdown")
	depth := request.GetInt("depth", 2)
	chapter := request.GetString("chapter", "")

	if depth < 1 {
		depth = 1
	}
	if depth > 4 {
		depth = 4
	}

	client := NewClient(docType)

	var outline []OutlineItem
	var err error

	if chapter != "" {
		// Fetch outline for specific chapter only
		outline, err = client.GetChapterOutline(chapter, depth)
	} else {
		// Fetch full outline with specified depth
		outline, err = client.GetOutlineWithDepth(depth)
	}

	if err != nil {
		return nil, fmt.Errorf("failed to get outline: %w", err)
	}

	var content string
	if format == "json" {
		data, err := json.MarshalIndent(outline, "", "  ")
		if err != nil {
			return nil, fmt.Errorf("failed to marshal outline: %w", err)
		}
		content = string(data)
	} else {
		content = formatOutlineAsMarkdown(outline, docType)
	}

	return mcp.NewToolResultText(content), nil
}

// DefaultMaxOutputLength is the default maximum output length in characters
const DefaultMaxOutputLength = 50000

// HandleSpecRead handles the plateau_spec_read tool
func HandleSpecRead(_ context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	path := request.GetString("path", "")
	docType := request.GetString("document_type", "standard")
	singlePage := request.GetBool("single_page", false)
	includeImages := request.GetBool("include_images", false)

	if path == "" {
		return nil, fmt.Errorf("path is required")
	}

	client := NewClient(docType)

	var doc *PlateauDocument
	var err error

	if singlePage {
		doc, err = client.GetContentByPath(path)
	} else {
		doc, err = client.GetContentWithChildren(path)
	}

	if err != nil {
		return nil, fmt.Errorf("failed to read content: %w", err)
	}

	opts := &FormatOptions{IncludeImages: includeImages}
	content := FormatDocumentAsMarkdown(doc, opts)

	// Check if content exceeds the maximum length
	if len(content) > DefaultMaxOutputLength {
		// Truncate and add hint
		truncated := content[:DefaultMaxOutputLength]
		// Find last complete line
		if lastNewline := strings.LastIndex(truncated, "\n"); lastNewline > 0 {
			truncated = truncated[:lastNewline]
		}

		// Get child paths for hint
		childPaths := getChildPaths(client, path)
		hint := formatTruncationHint(path, childPaths)

		content = truncated + "\n\n" + hint
	}

	return mcp.NewToolResultText(content), nil
}

// getChildPaths retrieves child paths for a given path
func getChildPaths(client *PlateauClient, path string) []string {
	nav, err := client.fetchNavigation(path)
	if err != nil || nav == nil {
		return nil
	}

	var paths []string
	for _, child := range nav.Children {
		if child.Path != "" {
			paths = append(paths, child.Path)
		}
	}
	return paths
}

// formatTruncationHint creates a hint message for truncated output
func formatTruncationHint(currentPath string, childPaths []string) string {
	var sb strings.Builder
	sb.WriteString("---\n")
	sb.WriteString("⚠️ **Output truncated due to length.**\n\n")
	sb.WriteString("To get the full content, please request smaller sections individually.\n\n")

	if len(childPaths) > 0 {
		sb.WriteString("Available sub-sections:\n")
		for _, p := range childPaths {
			_, _ = fmt.Fprintf(&sb, "- `%s`\n", p)
		}
		sb.WriteString("\nUse `plateau_spec_read` with these paths to get each section's content.\n")
	} else {
		_, _ = fmt.Fprintf(&sb, "Try using `plateau_spec_outline` with `chapter` parameter to find more specific paths under `%s`.\n", currentPath)
	}

	return sb.String()
}

// formatOutlineAsMarkdown formats the outline as markdown
func formatOutlineAsMarkdown(items []OutlineItem, docType string) string {
	var sb strings.Builder

	title := "3D都市モデル標準製品仕様書"
	if docType == "procedure" {
		title = "3D都市モデル標準作業手順書"
	}

	sb.WriteString(fmt.Sprintf("# %s 目次\n\n", title))
	sb.WriteString("以下のパスを `plateau_spec_read` ツールで指定すると、その節の内容を読むことができます。\n\n")

	formatOutlineItems(&sb, items, 0)

	return sb.String()
}

// formatOutlineItems recursively formats outline items
func formatOutlineItems(sb *strings.Builder, items []OutlineItem, depth int) {
	for _, item := range items {
		indent := strings.Repeat("  ", depth)
		_, _ = fmt.Fprintf(sb, "%s- **%s** `%s`\n", indent, item.Title, item.Path)

		if len(item.Children) > 0 {
			formatOutlineItems(sb, item.Children, depth+1)
		}
	}
}

// FormatOptions controls how content is formatted
type FormatOptions struct {
	IncludeImages bool
}

// FormatDocumentAsMarkdown formats a document as clean markdown
func FormatDocumentAsMarkdown(doc *PlateauDocument, opts *FormatOptions) string {
	var sb strings.Builder

	if opts == nil {
		opts = &FormatOptions{}
	}

	// Title
	if doc.Title != "" {
		sb.WriteString(fmt.Sprintf("# %s\n\n", normalizeTitle(doc.Title)))
	}

	// Content sections
	for _, content := range doc.Content {
		formatContent(&sb, content, opts)
	}

	return sb.String()
}

// normalizeTitle cleans up title text by removing unnecessary spaces
func normalizeTitle(title string) string {
	// Remove full-width space (U+3000) entirely
	title = strings.ReplaceAll(title, "\u3000", "")
	// Replace tab with single space
	title = strings.ReplaceAll(title, "\t", " ")
	// Replace multiple spaces with single space
	for strings.Contains(title, "  ") {
		title = strings.ReplaceAll(title, "  ", " ")
	}
	return strings.TrimSpace(title)
}

// formatContent formats a single content item
func formatContent(sb *strings.Builder, content PlateauContent, opts *FormatOptions) {
	switch content.Type {
	case "text":
		formatTextContent(sb, content.Content)
	case "paragraph":
		formatParagraphContent(sb, content.Content)
	case "title":
		formatTitleContent(sb, content.Content)
	case "table":
		formatTableContent(sb, content.Content)
	case "tableFigure":
		formatTableFigureContent(sb, content.Content)
	case "figure":
		formatFigureContent(sb, content.Content, opts)
	case "image":
		formatImageContent(sb, content.Content, opts)
	case "code":
		formatCodeContent(sb, content.Content)
	case "heading":
		formatHeadingContent(sb, content.Content)
	case "bullet_list", "ordered_list":
		formatListContent(sb, content.Content)
	case "external_link":
		formatExternalLinkContent(sb, content.Content)
	case "termWithDefinition":
		formatTermWithDefinition(sb, content.Content)
	default:
		// Try to extract text from unknown types
		formatGenericContent(sb, content.Content, opts)
	}
}

func formatTextContent(sb *strings.Builder, content interface{}) {
	switch v := content.(type) {
	case string:
		sb.WriteString(v)
	case map[string]interface{}:
		if text, ok := v["text"].(string); ok {
			sb.WriteString(text)
		}
	}
}

func formatParagraphContent(sb *strings.Builder, content interface{}) {
	if contentMap, ok := content.(map[string]interface{}); ok {
		if children, ok := contentMap["content"].([]interface{}); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]interface{}); ok {
					formatInlineContent(sb, childMap)
				}
			}
		}
	}
	sb.WriteString("\n\n")
}

// formatInlineContent handles inline elements like text, links, underline, bold, etc.
func formatInlineContent(sb *strings.Builder, contentMap map[string]interface{}) {
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
	case "underline", "bold", "italic", "strikethrough", "subscript", "superscript", "strong", "span":
		// These are inline formatting - extract nested content
		if innerContents, ok := contentMap["content"].([]interface{}); ok {
			for _, inner := range innerContents {
				if innerMap, ok := inner.(map[string]interface{}); ok {
					formatInlineContent(sb, innerMap)
				}
			}
		}
	default:
		// Try to extract text from unknown inline types
		if text, ok := contentMap["text"].(string); ok {
			sb.WriteString(text)
		} else if innerContents, ok := contentMap["content"].([]interface{}); ok {
			for _, inner := range innerContents {
				if innerMap, ok := inner.(map[string]interface{}); ok {
					formatInlineContent(sb, innerMap)
				}
			}
		}
	}
}

func formatTitleContent(sb *strings.Builder, content interface{}) {
	// Output as heading with normalized title
	if contentMap, ok := content.(map[string]interface{}); ok {
		if children, ok := contentMap["content"].([]interface{}); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]interface{}); ok {
					if text, ok := childMap["text"].(string); ok {
						sb.WriteString("## " + normalizeTitle(text) + "\n\n")
						return
					}
				}
			}
		}
	}
}

func formatListContent(sb *strings.Builder, content interface{}) {
	if contentMap, ok := content.(map[string]interface{}); ok {
		if items, ok := contentMap["content"].([]interface{}); ok {
			for _, item := range items {
				if itemMap, ok := item.(map[string]interface{}); ok {
					sb.WriteString("- ")
					if children, ok := itemMap["content"].([]interface{}); ok {
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
func formatListItemChildren(sb *strings.Builder, children []interface{}) {
	if len(children) == 0 {
		return
	}

	// Check if we have multiple paragraphs (title + description pattern)
	paragraphs := make([]map[string]interface{}, 0)
	for _, child := range children {
		if childMap, ok := child.(map[string]interface{}); ok {
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
		if titleContent, ok := paragraphs[0]["content"].([]interface{}); ok {
			for _, c := range titleContent {
				if cMap, ok := c.(map[string]interface{}); ok {
					formatInlineContent(&titleSb, cMap)
				}
			}
		}
		title := strings.TrimSpace(titleSb.String())

		// Extract description from remaining paragraphs
		var descSb strings.Builder
		for i := 1; i < len(paragraphs); i++ {
			if descContent, ok := paragraphs[i]["content"].([]interface{}); ok {
				for _, c := range descContent {
					if cMap, ok := c.(map[string]interface{}); ok {
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
			if childMap, ok := child.(map[string]interface{}); ok {
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
			if childMap, ok := child.(map[string]interface{}); ok {
				if childContent, ok := childMap["content"].([]interface{}); ok {
					for _, c := range childContent {
						if cMap, ok := c.(map[string]interface{}); ok {
							formatInlineContent(sb, cMap)
						}
					}
				}
			}
		}
	}
}

func formatGenericContent(sb *strings.Builder, content interface{}, opts *FormatOptions) {
	if contentMap, ok := content.(map[string]interface{}); ok {
		// Try to extract text from nested content
		if children, ok := contentMap["content"].([]interface{}); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]interface{}); ok {
					childType := "text"
					if t, ok := childMap["type"].(string); ok {
						childType = t
					}
					formatContent(sb, PlateauContent{Type: childType, Content: childMap}, opts)
				}
			}
		} else if text, ok := contentMap["text"].(string); ok {
			sb.WriteString(text)
		}
	}
}

func formatHeadingContent(sb *strings.Builder, content interface{}) {
	if contentMap, ok := content.(map[string]interface{}); ok {
		level := 2
		if l, ok := contentMap["level"].(float64); ok {
			level = int(l)
		}
		text := ""
		if t, ok := contentMap["text"].(string); ok {
			text = t
		} else if children, ok := contentMap["children"].([]interface{}); ok {
			for _, child := range children {
				if childMap, ok := child.(map[string]interface{}); ok {
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

func formatTableContent(sb *strings.Builder, content interface{}) {
	contentMap, ok := content.(map[string]interface{})
	if !ok {
		return
	}

	// Handle table with table_row structure (PLATEAU spec API format)
	if rows, ok := contentMap["content"].([]interface{}); ok {
		// First pass: check if table is a placeholder (mostly empty)
		if isPlaceholderTable(rows) {
			sb.WriteString("*[テンプレート用テーブル - 拡張製品仕様書で記入]*\n\n")
			return
		}

		for i, row := range rows {
			if rowMap, ok := row.(map[string]interface{}); ok {
				if rowMap["type"] == "table_row" {
					cells, ok := rowMap["content"].([]interface{})
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
	if rows, ok := contentMap["rows"].([]interface{}); ok {
		for i, row := range rows {
			if rowData, ok := row.([]interface{}); ok {
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
func extractCellText(cell interface{}) string {
	cellMap, ok := cell.(map[string]interface{})
	if !ok {
		return ""
	}

	contents, ok := cellMap["content"].([]interface{})
	if !ok {
		return ""
	}

	var sb strings.Builder
	for _, c := range contents {
		cMap, ok := c.(map[string]interface{})
		if !ok {
			continue
		}
		extractCellTextRecursive(&sb, cMap)
	}
	return sb.String()
}

// extractCellTextRecursive recursively extracts text from nested content
func extractCellTextRecursive(sb *strings.Builder, node map[string]interface{}) {
	nodeType, _ := node["type"].(string)

	// Handle text node
	if text, ok := node["text"].(string); ok {
		sb.WriteString(text)
		return
	}

	// Handle code node - wrap in backticks
	if nodeType == "code" {
		sb.WriteString("`")
		if contents, ok := node["content"].([]interface{}); ok {
			for _, c := range contents {
				if cMap, ok := c.(map[string]interface{}); ok {
					extractCellTextRecursive(sb, cMap)
				}
			}
		}
		sb.WriteString("`")
		return
	}

	// Recursively handle other nodes with content
	if contents, ok := node["content"].([]interface{}); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]interface{}); ok {
				extractCellTextRecursive(sb, cMap)
			}
		}
	}
}

// isPlaceholderTable checks if a table is a placeholder (mostly empty cells)
// Placeholder tables have only the header row with content, and body rows are empty or contain only whitespace
func isPlaceholderTable(rows []interface{}) bool {
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
		rowMap, ok := row.(map[string]interface{})
		if !ok || rowMap["type"] != "table_row" {
			continue
		}

		cells, ok := rowMap["content"].([]interface{})
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
func formatTableFigureContent(sb *strings.Builder, content interface{}) {
	contentMap, ok := content.(map[string]interface{})
	if !ok {
		return
	}

	// tableFigure > content > table
	if contents, ok := contentMap["content"].([]interface{}); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]interface{}); ok {
				if cMap["type"] == "table" {
					formatTableContent(sb, cMap)
				}
			}
		}
	}
}

// formatExternalLinkContent handles external links
func formatExternalLinkContent(sb *strings.Builder, content interface{}) {
	contentMap, ok := content.(map[string]interface{})
	if !ok {
		return
	}

	href := ""
	if attrs, ok := contentMap["attrs"].(map[string]interface{}); ok {
		if h, ok := attrs["href"].(string); ok {
			href = h
		}
	}

	text := ""
	if contents, ok := contentMap["content"].([]interface{}); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]interface{}); ok {
				if t, ok := cMap["text"].(string); ok {
					text += t
				}
			}
		}
	}

	if href != "" && text != "" {
		_, _ = fmt.Fprintf(sb, "[%s](%s)", text, href)
	} else if text != "" {
		sb.WriteString(text)
	}
}

func formatImageContent(sb *strings.Builder, content interface{}, opts *FormatOptions) {
	contentMap, ok := content.(map[string]interface{})
	if !ok {
		return
	}

	// Check attrs.src first (for nested structure)
	if attrs, ok := contentMap["attrs"].(map[string]interface{}); ok {
		if src, ok := attrs["src"].(string); ok {
			alt := "image"
			if a, ok := attrs["alt"].(string); ok {
				alt = a
			}
			// Handle base64 encoded images
			if strings.HasPrefix(src, "data:image/") {
				if opts != nil && opts.IncludeImages {
					_, _ = fmt.Fprintf(sb, "![%s](%s)\n\n", alt, src)
				} else {
					sb.WriteString("*[Image]*\n\n")
				}
				return
			}
			_, _ = fmt.Fprintf(sb, "![%s](%s)\n\n", alt, src)
			return
		}
	}

	// Fallback to direct src
	if src, ok := contentMap["src"].(string); ok {
		alt := "image"
		if a, ok := contentMap["alt"].(string); ok {
			alt = a
		}
		// Handle base64 encoded images
		if strings.HasPrefix(src, "data:image/") {
			if opts != nil && opts.IncludeImages {
				_, _ = fmt.Fprintf(sb, "![%s](%s)\n\n", alt, src)
			} else {
				sb.WriteString("*[Image]*\n\n")
			}
			return
		}
		_, _ = fmt.Fprintf(sb, "![%s](%s)\n\n", alt, src)
	} else {
		sb.WriteString("*[Image]*\n\n")
	}
}

// formatFigureContent handles figure elements with images and captions
func formatFigureContent(sb *strings.Builder, content interface{}, opts *FormatOptions) {
	contentMap, ok := content.(map[string]interface{})
	if !ok {
		return
	}

	contents, ok := contentMap["content"].([]interface{})
	if !ok {
		return
	}

	var caption string
	for _, c := range contents {
		cMap, ok := c.(map[string]interface{})
		if !ok {
			continue
		}

		cType, _ := cMap["type"].(string)
		switch cType {
		case "image":
			formatImageContent(sb, cMap, opts)
		case "figcaption":
			// Extract caption text
			if innerContents, ok := cMap["content"].([]interface{}); ok {
				for _, inner := range innerContents {
					if innerMap, ok := inner.(map[string]interface{}); ok {
						if text, ok := innerMap["text"].(string); ok {
							caption += text
						}
					}
				}
			}
		}
	}

	if caption != "" {
		_, _ = fmt.Fprintf(sb, "%s\n\n", caption)
	}
}

// formatTermWithDefinition handles term definitions (e.g., "1.5.1 3D都市モデル")
func formatTermWithDefinition(sb *strings.Builder, content interface{}) {
	contentMap, ok := content.(map[string]interface{})
	if !ok {
		return
	}

	contents, ok := contentMap["content"].([]interface{})
	if !ok {
		return
	}

	var label, term, definition string

	for _, c := range contents {
		cMap, ok := c.(map[string]interface{})
		if !ok {
			continue
		}

		cType, _ := cMap["type"].(string)
		switch cType {
		case "termXrefLabel":
			// Extract label (e.g., "1.5.1")
			if innerContents, ok := cMap["content"].([]interface{}); ok {
				for _, inner := range innerContents {
					if innerMap, ok := inner.(map[string]interface{}); ok {
						if text, ok := innerMap["text"].(string); ok {
							label += text
						}
					}
				}
			}
		case "term":
			// Extract term text (may contain strong, span, etc.)
			if innerContents, ok := cMap["content"].([]interface{}); ok {
				for _, inner := range innerContents {
					if innerMap, ok := inner.(map[string]interface{}); ok {
						term += extractTextRecursive(innerMap)
					}
				}
			}
		case "definition":
			// Extract definition text
			if innerContents, ok := cMap["content"].([]interface{}); ok {
				for _, inner := range innerContents {
					if innerMap, ok := inner.(map[string]interface{}); ok {
						definition += extractTextRecursive(innerMap)
					}
				}
			}
		}
	}

	// Output with proper spacing: "1.5.1 3D都市モデル\n定義..."
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

// extractTextRecursive extracts all text from nested content
func extractTextRecursive(contentMap map[string]interface{}) string {
	var result string

	if text, ok := contentMap["text"].(string); ok {
		result += text
	}

	if contents, ok := contentMap["content"].([]interface{}); ok {
		for _, c := range contents {
			if cMap, ok := c.(map[string]interface{}); ok {
				result += extractTextRecursive(cMap)
			}
		}
	}

	return result
}

func formatCodeContent(sb *strings.Builder, content interface{}) {
	sb.WriteString("```\n")
	switch v := content.(type) {
	case string:
		sb.WriteString(v)
	case map[string]interface{}:
		if code, ok := v["code"].(string); ok {
			sb.WriteString(code)
		} else if value, ok := v["value"].(string); ok {
			sb.WriteString(value)
		}
	}
	sb.WriteString("\n```\n\n")
}
