// Package metanorma2md converts Metanorma JSON content to Markdown format.
// Metanorma is a document authoring framework used by PLATEAU specification documents.
package metanorma2md

import (
	"fmt"
	"regexp"
	"strings"
)

// Document represents a Metanorma document
type Document struct {
	Title    string
	Path     string
	Content  []Content
	Metadata map[string]any
}

// Content represents a content block in a Metanorma document
type Content struct {
	Type    string
	Content any
}

// Options configures the Markdown conversion
type Options struct {
	IncludeImages bool // If true, include base64 images; if false, show placeholder
}

// Convert converts a Metanorma Document to Markdown format
func Convert(doc *Document, opts *Options) string {
	if doc == nil {
		return ""
	}

	if opts == nil {
		opts = &Options{}
	}

	var sb strings.Builder

	// Add title
	if doc.Title != "" {
		title := normalizeTitle(doc.Title)
		_, _ = fmt.Fprintf(&sb, "# %s\n\n", title)
	}

	// Format each content block
	for _, content := range doc.Content {
		formatContent(&sb, content, opts)
	}

	return sb.String()
}

// normalizeTitle cleans up title formatting
func normalizeTitle(title string) string {
	// Replace tabs with spaces
	title = strings.ReplaceAll(title, "\t", " ")
	// Replace various Unicode space characters with regular space
	// U+3000 (IDEOGRAPHIC SPACE - 全角スペース)
	title = strings.ReplaceAll(title, "\u3000", " ")
	// U+2004 (THREE-PER-EM SPACE)
	title = strings.ReplaceAll(title, "\u2004", " ")
	// U+2003 (EM SPACE)
	title = strings.ReplaceAll(title, "\u2003", " ")
	// U+2002 (EN SPACE)
	title = strings.ReplaceAll(title, "\u2002", " ")
	// U+00A0 (NO-BREAK SPACE)
	title = strings.ReplaceAll(title, "\u00A0", " ")
	// Collapse multiple spaces
	re := regexp.MustCompile(`\s+`)
	title = re.ReplaceAllString(title, " ")
	return strings.TrimSpace(title)
}

// formatContent dispatches to the appropriate formatter based on content type
func formatContent(sb *strings.Builder, content Content, opts *Options) {
	switch content.Type {
	case "title":
		formatTitleContent(sb, content.Content)
	case "paragraph":
		formatParagraphContent(sb, content.Content)
	case "table":
		formatTableContentWithOpts(sb, content.Content, opts)
	case "tableFigure":
		formatTableFigureContentWithOpts(sb, content.Content, opts)
	case "bullet_list", "ordered_list":
		formatListContent(sb, content.Content)
	case "figure":
		formatFigureContent(sb, content.Content, opts)
	case "image":
		formatImageContent(sb, content.Content, opts)
	case "code":
		formatCodeContent(sb, content.Content)
	case "heading":
		formatHeadingContent(sb, content.Content)
	case "external_link":
		formatExternalLinkContent(sb, content.Content)
	case "termWithDefinition":
		formatTermWithDefinition(sb, content.Content)
	case "bibitem":
		formatBibitemContent(sb, content.Content)
	default:
		// Try to extract text from unknown types
		formatGenericContent(sb, content.Content, opts)
	}
}
