package plateauspecmcp

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// PlateauClient handles communication with PLATEAU specification API
type PlateauClient struct {
	BaseURL      string
	DocumentType string // "standard" or "procedure"
	HTTPClient   *http.Client
}

// NewClient creates a new PLATEAU specification client
func NewClient(documentType string) *PlateauClient {
	baseURL := "https://www.mlit.go.jp/plateaudocument"
	if documentType == "procedure" {
		baseURL = "https://www.mlit.go.jp/plateaudocument02"
	}

	return &PlateauClient{
		BaseURL:      baseURL,
		DocumentType: documentType,
		HTTPClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// ListChapters retrieves the list of chapters
func (c *PlateauClient) ListChapters() ([]Chapter, error) {
	url := fmt.Sprintf("%s/resource-nav.json", c.BaseURL)

	resp, err := c.HTTPClient.Get(url)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch navigation: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var nav PlateauNavigation
	if err := json.NewDecoder(resp.Body).Decode(&nav); err != nil {
		return nil, fmt.Errorf("failed to decode navigation: %w", err)
	}

	var chapters []Chapter
	for _, child := range nav.Children {
		pathParts := strings.Split(strings.Trim(child.Path, "/"), "/")
		if len(pathParts) >= 2 {
			chapterID := pathParts[1]
			chapters = append(chapters, Chapter{
				ID:    chapterID,
				Title: child.PlainTitle,
				Path:  child.Path,
			})
		}
	}

	return chapters, nil
}

// ListSectionsByPath retrieves sections for a given path
func (c *PlateauClient) ListSectionsByPath(path string) ([]Section, error) {
	if !strings.HasPrefix(path, "/") {
		path = "/" + path
	}

	url := fmt.Sprintf("%s%s/resource-nav.json", c.BaseURL, path)

	resp, err := c.HTTPClient.Get(url)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch navigation from path %s: %w", path, err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code for path %s: %d", path, resp.StatusCode)
	}

	var nav PlateauNavigation
	if err := json.NewDecoder(resp.Body).Decode(&nav); err != nil {
		return nil, fmt.Errorf("failed to decode navigation from path %s: %w", path, err)
	}

	var sections []Section
	for _, child := range nav.Children {
		pathParts := strings.Split(strings.Trim(child.Path, "/"), "/")
		if len(pathParts) > 0 {
			sectionID := pathParts[len(pathParts)-1]
			sections = append(sections, Section{
				ID:      sectionID,
				Title:   child.PlainTitle,
				Path:    child.Path,
				Chapter: path,
			})
		}
	}

	return sections, nil
}

// GetContentByPath retrieves content for a specific path
func (c *PlateauClient) GetContentByPath(path string) (*PlateauDocument, error) {
	if !strings.HasPrefix(path, "/") {
		path = "/" + path
	}

	var url string
	expectedBasePath := "/plateaudocument"
	if c.DocumentType == "procedure" {
		expectedBasePath = "/plateaudocument02"
	}

	if strings.HasPrefix(path, expectedBasePath) {
		url = fmt.Sprintf("https://www.mlit.go.jp%s/resource-content.json", path)
	} else {
		url = fmt.Sprintf("%s%s/resource-content.json", c.BaseURL, path)
	}

	resp, err := c.HTTPClient.Get(url)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch content from path %s: %w", path, err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code for path %s: %d", path, resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response body: %w", err)
	}

	// Parse as raw JSON first to handle flexible content structure
	var rawDoc map[string]interface{}
	if err := json.Unmarshal(body, &rawDoc); err != nil {
		return nil, fmt.Errorf("failed to parse content JSON: %w", err)
	}

	// Convert to our document structure
	doc := &PlateauDocument{
		Path:     path,
		Metadata: make(map[string]interface{}),
	}

	// Extract title
	if title, ok := rawDoc["title"].(string); ok {
		doc.Title = title
	}

	// Process content array if present
	if contents, ok := rawDoc["content"].([]interface{}); ok {
		for _, item := range contents {
			if contentMap, ok := item.(map[string]interface{}); ok {
				content := PlateauContent{
					Type:    "text",
					Content: contentMap,
				}

				// Determine content type
				if contentType, ok := contentMap["type"].(string); ok {
					content.Type = contentType
				}

				doc.Content = append(doc.Content, content)
			}
		}
	}

	// Store any additional metadata
	for key, value := range rawDoc {
		if key != "title" && key != "content" && key != "path" {
			doc.Metadata[key] = value
		}
	}

	return doc, nil
}

// ListRecursive recursively lists all sections under a path
func (c *PlateauClient) ListRecursive(path string, maxDepth int) ([]Section, error) {
	var allSections []Section

	// Get immediate children
	sections, err := c.ListSectionsByPath(path)
	if err != nil {
		return nil, err
	}

	allSections = append(allSections, sections...)

	// Recursively get subsections if depth allows
	if maxDepth > 1 {
		for _, section := range sections {
			subSections, err := c.ListRecursive(section.Path, maxDepth-1)
			if err != nil {
				// Continue on error to get as much data as possible
				continue
			}
			allSections = append(allSections, subSections...)
		}
	}

	return allSections, nil
}
