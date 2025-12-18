package plateauspecmcp

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

// OutlineItem represents a hierarchical outline item
type OutlineItem struct {
	ID       string        `json:"id"`
	Title    string        `json:"title"`
	Path     string        `json:"path"`
	Children []OutlineItem `json:"children,omitempty"`
}
