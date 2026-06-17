package plateauspecmcp

import (
	"context"
	"fmt"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// RegisterResources registers PLATEAU specification resources
func RegisterResources(s *server.MCPServer) {
	// Get initial resources
	resources, _ := HandleResourceList(context.Background())

	// Add each resource with its handler
	for _, resource := range resources {
		s.AddResource(resource, HandleResourceRead)
	}
}

// HandleResourceList returns a list of available PLATEAU specification resources
func HandleResourceList(_ context.Context) ([]mcp.Resource, error) {
	resources := []mcp.Resource{
		{
			URI:         "plateau://standard/outline",
			Name:        "PLATEAU Standard Product Specification - Table of Contents",
			Description: "Complete table of contents of the 3D City Model Standard Product Specification",
			MIMEType:    "text/markdown",
		},
		{
			URI:         "plateau://procedure/outline",
			Name:        "PLATEAU Standard Work Procedures - Table of Contents",
			Description: "Complete table of contents of the 3D City Model Standard Work Procedures",
			MIMEType:    "text/markdown",
		},
	}

	return resources, nil
}

// HandleResourceRead reads the content of a specific resource
func HandleResourceRead(ctx context.Context, request mcp.ReadResourceRequest) ([]mcp.ResourceContents, error) {
	uri := request.Params.URI
	// Parse URI
	if !strings.HasPrefix(uri, "plateau://") {
		return nil, fmt.Errorf("invalid resource URI: %s", uri)
	}

	parts := strings.Split(strings.TrimPrefix(uri, "plateau://"), "/")
	if len(parts) < 1 {
		return nil, fmt.Errorf("invalid resource URI format: %s", uri)
	}

	category := parts[0]

	// Handle outline requests
	if len(parts) >= 2 && parts[1] == "outline" {
		docType := "standard"
		if category == "procedure" {
			docType = "procedure"
		}

		client := NewClient()
		outline, err := client.GetOutline(ctx, docType)
		if err != nil {
			return nil, fmt.Errorf("failed to get outline: %w", err)
		}

		// Apply depth limit
		outline = limitDepth(outline, 2)

		content := formatOutlineAsMarkdown(outline, docType, "以下のパスを `plateau_spec_read` ツールで指定すると、その節の内容を読むことができます。")
		return []mcp.ResourceContents{
			mcp.TextResourceContents{
				URI:      uri,
				MIMEType: "text/markdown",
				Text:     content,
			},
		}, nil
	}

	return nil, fmt.Errorf("unknown resource: %s", uri)
}
