package mcp

import (
	"context"

	"github.com/mark3labs/mcp-go/server"
)

// RegisterSpecificationResources registers PLATEAU specification resources
func RegisterSpecificationResources(s *server.MCPServer) {
	// Get initial resources
	resources, _ := HandleResourceList(context.Background())
	
	// Add each resource with its handler
	for _, resource := range resources {
		s.AddResource(resource, HandleResourceRead)
	}
}