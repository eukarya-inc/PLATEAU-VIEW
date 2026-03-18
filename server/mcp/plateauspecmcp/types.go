package plateauspecmcp

// OutlineItem represents a hierarchical outline item
type OutlineItem struct {
	ID       string        `json:"id"`
	Title    string        `json:"title"`
	Path     string        `json:"path"`
	Children []OutlineItem `json:"children,omitempty"`
}
