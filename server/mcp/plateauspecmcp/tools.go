package plateauspecmcp

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/mcp/metanorma2md"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// newClientFunc allows overriding NewClient for testing
var newClientFunc = NewClient

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

	client := newClientFunc(docType)

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

	client := newClientFunc(docType)

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

	content := FormatDocumentAsMarkdown(doc, includeImages)

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

// FormatDocumentAsMarkdown formats a document as clean markdown using the metanorma2md package
func FormatDocumentAsMarkdown(doc *PlateauDocument, includeImages bool) string {
	if doc == nil {
		return ""
	}

	// Convert PlateauDocument to metanorma2md.Document
	mdDoc := &metanorma2md.Document{
		Title:    doc.Title,
		Path:     doc.Path,
		Metadata: doc.Metadata,
		Content:  make([]metanorma2md.Content, len(doc.Content)),
	}

	for i, c := range doc.Content {
		mdDoc.Content[i] = metanorma2md.Content{
			Type:    c.Type,
			Content: c.Content,
		}
	}

	opts := &metanorma2md.Options{
		IncludeImages: includeImages,
	}

	return metanorma2md.Convert(mdDoc, opts)
}
