package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
)

// HandleSpecList handles the plateau_spec_list tool
func HandleSpecList(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Parse arguments
	path := request.GetString("path", "")
	recursive := request.GetBool("recursive", false)
	docType := request.GetString("document_type", "standard")
	format := request.GetString("format", "tree")
	offset := float64(request.GetInt("offset", 0))
	limit := float64(request.GetInt("limit", 100))

	client := NewClient(docType)

	// Handle root listing
	if path == "" || path == "/" {
		chapters, err := client.ListChapters()
		if err != nil {
			return nil, err
		}

		// Apply pagination
		start := int(offset)
		end := int(offset + limit)
		if start > len(chapters) {
			start = len(chapters)
		}
		if end > len(chapters) {
			end = len(chapters)
		}
		chapters = chapters[start:end]

		// Format output
		var content string
		if format == "json" {
			data, _ := json.MarshalIndent(chapters, "", "  ")
			content = string(data)
		} else {
			content = formatChaptersAsTree(chapters)
		}

		return mcp.NewToolResultText(content), nil
	}

	// Handle section listing
	var sections []Section
	var err error

	if recursive {
		sections, err = client.ListRecursive(path, 5) // Max depth of 5
	} else {
		sections, err = client.ListSectionsByPath(path)
	}

	if err != nil {
		return nil, err
	}

	// Apply pagination
	start := int(offset)
	end := int(offset + limit)
	if start > len(sections) {
		start = len(sections)
	}
	if end > len(sections) {
		end = len(sections)
	}
	sections = sections[start:end]

	// Format output
	var content string
	if format == "json" {
		data, _ := json.MarshalIndent(sections, "", "  ")
		content = string(data)
	} else {
		content = formatSectionsAsTree(sections, path)
	}

	return mcp.NewToolResultText(content), nil
}

// HandleSpecGetContent handles the plateau_spec_get_content tool
func HandleSpecGetContent(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Parse arguments
	path := request.GetString("path", "")
	format := request.GetString("format", "markdown")
	docType := request.GetString("document_type", "standard")

	if path == "" {
		return nil, fmt.Errorf("path is required")
	}

	client := NewClient(docType)
	doc, err := client.GetContentByPath(path)
	if err != nil {
		return nil, err
	}

	// Format content based on requested format
	var content string
	switch format {
	case "json":
		data, _ := json.MarshalIndent(doc, "", "  ")
		content = string(data)
	case "html":
		content = formatAsHTML(doc)
	default: // markdown
		content = formatAsMarkdown(doc)
	}

	return mcp.NewToolResultText(content), nil
}

// HandleSpecGetContentsBatch handles the plateau_spec_get_contents_batch tool
func HandleSpecGetContentsBatch(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Parse arguments
	pathsArg := request.GetString("paths", "")
	format := request.GetString("format", "markdown")
	docType := request.GetString("document_type", "standard")
	offset := float64(request.GetInt("offset", 0))
	limit := float64(request.GetInt("limit", 10))

	if pathsArg == "" {
		return nil, fmt.Errorf("paths is required")
	}

	// Parse paths (support both JSON array and comma-separated)
	var paths []string
	if strings.HasPrefix(pathsArg, "[") {
		if err := json.Unmarshal([]byte(pathsArg), &paths); err != nil {
			// Try comma-separated
			paths = strings.Split(pathsArg, ",")
		}
	} else {
		paths = strings.Split(pathsArg, ",")
	}

	// Clean paths
	for i := range paths {
		paths[i] = strings.TrimSpace(paths[i])
	}

	// Apply pagination
	start := int(offset)
	end := int(offset + limit)
	if start > len(paths) {
		start = len(paths)
	}
	if end > len(paths) {
		end = len(paths)
	}
	paths = paths[start:end]

	client := NewClient(docType)
	var results []map[string]interface{}

	for _, path := range paths {
		doc, err := client.GetContentByPath(path)
		if err != nil {
			results = append(results, map[string]interface{}{
				"path":  path,
				"error": err.Error(),
			})
			continue
		}

		var content string
		switch format {
		case "json":
			content = "json content" // Simplified for batch
		case "html":
			content = formatAsHTML(doc)
		default:
			content = formatAsMarkdown(doc)
		}

		results = append(results, map[string]interface{}{
			"path":    path,
			"title":   doc.Title,
			"content": content,
		})
	}

	data, _ := json.MarshalIndent(results, "", "  ")
	return mcp.NewToolResultText(string(data)), nil
}

// HandleSpecSearch handles the plateau_spec_search tool
func HandleSpecSearch(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Parse arguments
	query := request.GetString("query", "")
	docType := request.GetString("document_type", "standard")
	scope := request.GetString("scope", "titles")
	limit := request.GetInt("limit", 20)

	if query == "" {
		return nil, fmt.Errorf("query is required")
	}

	// Create client(s) based on document type
	var clients []*PlateauClient
	if docType == "all" {
		clients = append(clients, NewClient("standard"))
		clients = append(clients, NewClient("procedure"))
	} else {
		clients = append(clients, NewClient(docType))
	}

	var allResults []SearchResult
	queryLower := strings.ToLower(query)

	for _, client := range clients {
		// Search in titles first
		if scope == "titles" || scope == "all" {
			chapters, err := client.ListChapters()
			if err == nil {
				for _, chapter := range chapters {
					if strings.Contains(strings.ToLower(chapter.Title), queryLower) {
						allResults = append(allResults, SearchResult{
							Path:    chapter.Path,
							Title:   chapter.Title,
							Snippet: chapter.Title,
							Score:   calculateScore(chapter.Title, query),
							DocType: client.DocumentType,
						})
					}

					// Search in sections
					sections, err := client.ListSectionsByPath(chapter.Path)
					if err == nil {
						for _, section := range sections {
							if strings.Contains(strings.ToLower(section.Title), queryLower) {
								allResults = append(allResults, SearchResult{
									Path:    section.Path,
									Title:   section.Title,
									Snippet: section.Title,
									Score:   calculateScore(section.Title, query),
									DocType: client.DocumentType,
								})
							}
						}
					}
				}
			}
		}

		// Note: Content search would require fetching and searching actual document content
		// This is expensive and should be implemented with caching/indexing in production
		if scope == "content" || scope == "all" {
			// Simplified implementation - would need proper text search
			// This is a placeholder that indicates where content search would go
		}
	}

	// Sort by score and apply limit
	allResults = sortByScore(allResults)
	if len(allResults) > limit {
		allResults = allResults[:limit]
	}

	data, _ := json.MarshalIndent(allResults, "", "  ")
	return mcp.NewToolResultText(string(data)), nil
}

// Helper functions

func formatChaptersAsTree(chapters []Chapter) string {
	var sb strings.Builder
	sb.WriteString("PLATEAU Specification Chapters:\n\n")
	for _, ch := range chapters {
		sb.WriteString(fmt.Sprintf("📁 %s - %s\n", ch.ID, ch.Title))
		sb.WriteString(fmt.Sprintf("   Path: %s\n", ch.Path))
	}
	return sb.String()
}

func formatSectionsAsTree(sections []Section, basePath string) string {
	var sb strings.Builder
	sb.WriteString(fmt.Sprintf("Sections under %s:\n\n", basePath))
	for _, sec := range sections {
		indent := strings.Count(sec.Path, "/") - strings.Count(basePath, "/")
		prefix := strings.Repeat("  ", indent)
		sb.WriteString(fmt.Sprintf("%s📄 %s - %s\n", prefix, sec.ID, sec.Title))
		sb.WriteString(fmt.Sprintf("%s   Path: %s\n", prefix, sec.Path))
	}
	return sb.String()
}

func formatAsMarkdown(doc *PlateauDocument) string {
	var sb strings.Builder
	
	// Title
	if doc.Title != "" {
		sb.WriteString(fmt.Sprintf("# %s\n\n", doc.Title))
	}

	// Path
	sb.WriteString(fmt.Sprintf("**Path:** `%s`\n\n", doc.Path))

	// Content sections
	for _, content := range doc.Content {
		switch content.Type {
		case "text":
			if text, ok := content.Content.(string); ok {
				sb.WriteString(text + "\n\n")
			} else if contentMap, ok := content.Content.(map[string]interface{}); ok {
				// Handle structured content
				data, _ := json.MarshalIndent(contentMap, "", "  ")
				sb.WriteString("```json\n" + string(data) + "\n```\n\n")
			}
		case "table":
			sb.WriteString("*[Table content]*\n\n")
		case "image":
			sb.WriteString("*[Image content]*\n\n")
		case "code":
			sb.WriteString("```\n")
			if code, ok := content.Content.(string); ok {
				sb.WriteString(code)
			}
			sb.WriteString("\n```\n\n")
		}
	}

	return sb.String()
}

func formatAsHTML(doc *PlateauDocument) string {
	var sb strings.Builder
	
	sb.WriteString("<html><body>\n")
	
	if doc.Title != "" {
		sb.WriteString(fmt.Sprintf("<h1>%s</h1>\n", doc.Title))
	}
	
	sb.WriteString(fmt.Sprintf("<p><strong>Path:</strong> <code>%s</code></p>\n", doc.Path))
	
	for _, content := range doc.Content {
		switch content.Type {
		case "text":
			if text, ok := content.Content.(string); ok {
				sb.WriteString(fmt.Sprintf("<p>%s</p>\n", text))
			}
		case "table":
			sb.WriteString("<p><em>[Table content]</em></p>\n")
		case "image":
			sb.WriteString("<p><em>[Image content]</em></p>\n")
		case "code":
			sb.WriteString("<pre><code>")
			if code, ok := content.Content.(string); ok {
				sb.WriteString(code)
			}
			sb.WriteString("</code></pre>\n")
		}
	}
	
	sb.WriteString("</body></html>\n")
	
	return sb.String()
}

func calculateScore(text, query string) float64 {
	textLower := strings.ToLower(text)
	queryLower := strings.ToLower(query)
	
	// Simple scoring based on position and exact match
	if textLower == queryLower {
		return 1.0
	}
	if strings.HasPrefix(textLower, queryLower) {
		return 0.8
	}
	if strings.Contains(textLower, queryLower) {
		return 0.5
	}
	
	// Partial word matching
	words := strings.Fields(queryLower)
	matches := 0
	for _, word := range words {
		if strings.Contains(textLower, word) {
			matches++
		}
	}
	
	if len(words) > 0 {
		return float64(matches) / float64(len(words)) * 0.3
	}
	
	return 0.0
}

func sortByScore(results []SearchResult) []SearchResult {
	// Simple bubble sort for small datasets
	n := len(results)
	for i := 0; i < n-1; i++ {
		for j := 0; j < n-i-1; j++ {
			if results[j].Score < results[j+1].Score {
				results[j], results[j+1] = results[j+1], results[j]
			}
		}
	}
	return results
}