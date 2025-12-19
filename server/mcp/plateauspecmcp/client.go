package plateauspecmcp

import (
	"context"
	"fmt"
	"strings"

	"github.com/eukarya-inc/plateau-spec/cmd/plateaudocs"
)

// plateaudocsClient is an interface for the plateaudocs client (for testing)
type plateaudocsClient interface {
	GetIndex(ctx context.Context, docType string) (*plateaudocs.Index, error)
	GetMarkdown(ctx context.Context, docType, path string) (string, error)
}

// Client wraps plateaudocs.Client for MCP usage
type Client struct {
	client plateaudocsClient
}

// NewClient creates a new Client
func NewClient() *Client {
	return &Client{
		client: plateaudocs.New(),
	}
}

// GetOutline retrieves the outline for the given document type
func (c *Client) GetOutline(ctx context.Context, docType string) ([]OutlineItem, error) {
	index, err := c.client.GetIndex(ctx, docType)
	if err != nil {
		return nil, fmt.Errorf("failed to get index: %w", err)
	}

	return convertChaptersToOutline(index.Chapters), nil
}

// GetChapterOutline retrieves the outline for a specific chapter
func (c *Client) GetChapterOutline(ctx context.Context, docType, chapterID string) ([]OutlineItem, error) {
	index, err := c.client.GetIndex(ctx, docType)
	if err != nil {
		return nil, fmt.Errorf("failed to get index: %w", err)
	}

	// Find the chapter and return its children
	for _, ch := range index.Chapters {
		if getPathID(ch.Path) == chapterID {
			return convertChaptersToOutline(ch.Children), nil
		}
	}

	return nil, fmt.Errorf("chapter not found: %s", chapterID)
}

// GetMarkdown retrieves markdown content for the given path
func (c *Client) GetMarkdown(ctx context.Context, docType, path string) (string, error) {
	// Normalize path: remove leading slash and docType prefix if present
	path = normalizePath(path)

	content, err := c.client.GetMarkdown(ctx, docType, path)
	if err != nil {
		return "", fmt.Errorf("failed to get markdown: %w", err)
	}

	return content, nil
}

// GetMarkdownWithChildren retrieves markdown content including child sections
func (c *Client) GetMarkdownWithChildren(ctx context.Context, docType, path string) (string, error) {
	path = normalizePath(path)

	// Get the index to find child paths
	index, err := c.client.GetIndex(ctx, docType)
	if err != nil {
		return "", fmt.Errorf("failed to get index: %w", err)
	}

	// Find all paths under the given path
	paths := collectPaths(index.Chapters, path)
	if len(paths) == 0 {
		// If no children found, just get the single page
		return c.client.GetMarkdown(ctx, docType, path)
	}

	// Fetch all markdown contents
	var sb strings.Builder
	for i, p := range paths {
		content, err := c.client.GetMarkdown(ctx, docType, p)
		if err != nil {
			continue // Skip failed fetches
		}
		if i > 0 {
			sb.WriteString("\n\n---\n\n")
		}
		sb.WriteString(content)
	}

	return sb.String(), nil
}

// GetChildPaths returns the child paths for a given path
func (c *Client) GetChildPaths(ctx context.Context, docType, path string) ([]string, error) {
	path = normalizePath(path)

	index, err := c.client.GetIndex(ctx, docType)
	if err != nil {
		return nil, fmt.Errorf("failed to get index: %w", err)
	}

	// Find the chapter and return its children's paths
	chapter := findChapter(index.Chapters, path)
	if chapter == nil {
		return nil, nil
	}

	var paths []string
	for _, child := range chapter.Children {
		paths = append(paths, child.Path)
	}
	return paths, nil
}

// convertChaptersToOutline converts plateaudocs.Chapter slice to OutlineItem slice
func convertChaptersToOutline(chapters []plateaudocs.Chapter) []OutlineItem {
	items := make([]OutlineItem, 0, len(chapters))
	for _, ch := range chapters {
		item := OutlineItem{
			ID:       getPathID(ch.Path),
			Title:    ch.Title,
			Path:     ch.Path,
			Children: convertChaptersToOutline(ch.Children),
		}
		items = append(items, item)
	}
	return items
}

// getPathID extracts the ID from a path (e.g., "toc1" from "toc1" or "/plateaudocument/toc1")
func getPathID(path string) string {
	parts := strings.Split(strings.Trim(path, "/"), "/")
	if len(parts) > 0 {
		return parts[len(parts)-1]
	}
	return path
}

// normalizePath removes leading slash and document type prefix
func normalizePath(path string) string {
	path = strings.TrimPrefix(path, "/")
	path = strings.TrimPrefix(path, "plateaudocument/")
	path = strings.TrimPrefix(path, "plateaudocument02/")
	return path
}

// collectPaths collects all paths under the given root path (including the root)
func collectPaths(chapters []plateaudocs.Chapter, rootPath string) []string {
	var paths []string

	var collect func(chs []plateaudocs.Chapter, found bool)
	collect = func(chs []plateaudocs.Chapter, found bool) {
		for _, ch := range chs {
			chPath := getPathID(ch.Path)
			if found || chPath == rootPath {
				paths = append(paths, chPath)
				collect(ch.Children, true)
			} else {
				collect(ch.Children, false)
			}
		}
	}

	collect(chapters, false)
	return paths
}

// findChapter finds a chapter by path
func findChapter(chapters []plateaudocs.Chapter, path string) *plateaudocs.Chapter {
	for i := range chapters {
		ch := &chapters[i]
		if getPathID(ch.Path) == path {
			return ch
		}
		if found := findChapter(ch.Children, path); found != nil {
			return found
		}
	}
	return nil
}
