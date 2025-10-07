package mcp

// Types for PLATEAU specification documents

// PlateauNavigation represents the navigation structure of PLATEAU documents
type PlateauNavigation struct {
	Path       string              `json:"path"`
	PlainTitle string              `json:"plainTitle"`
	Children   []PlateauNavigation `json:"children"`
}

// PlateauDocument represents a PLATEAU document content
type PlateauDocument struct {
	Title    string                 `json:"title"`
	Path     string                 `json:"path"`
	Content  []PlateauContent       `json:"content"`
	Metadata map[string]interface{} `json:"metadata,omitempty"`
}

// PlateauContent represents content sections within a document
type PlateauContent struct {
	Type    string      `json:"type"`    // "text", "table", "image", "code"
	Content interface{} `json:"content"` // Actual content varies by type
}

// Chapter represents a top-level chapter in the document
type Chapter struct {
	ID    string `json:"id"`
	Title string `json:"title"`
	Path  string `json:"path"`
}

// Section represents a section within a chapter
type Section struct {
	ID      string `json:"id"`
	Title   string `json:"title"`
	Path    string `json:"path"`
	Chapter string `json:"chapter"`
}

// SearchResult represents a search result from the specification
type SearchResult struct {
	Path      string  `json:"path"`
	Title     string  `json:"title"`
	Snippet   string  `json:"snippet"`
	Score     float64 `json:"score"`
	DocType   string  `json:"doc_type"`
	Highlight string  `json:"highlight,omitempty"`
}

// Resource represents an MCP resource for specification access
type Resource struct {
	URI         string `json:"uri"`
	Name        string `json:"name"`
	Description string `json:"description"`
	MimeType    string `json:"mimeType"`
}