package mcp

import (
	"github.com/mark3labs/mcp-go/server"
)

// NewServer creates and configures a new comprehensive PLATEAU MCP server
func NewServer() *server.MCPServer {
	s := server.NewMCPServer(
		"plateau-mcp",
		"1.0.0",
		server.WithToolCapabilities(true),
		server.WithResourceCapabilities(true, false),
		server.WithLogging(),
	)

	// Register all PLATEAU tools and resources
	RegisterTools(s)
	RegisterResources(s)

	return s
}

// RegisterTools registers all PLATEAU MCP tools
func RegisterTools(s *server.MCPServer) {
	// Specification reading tools
	RegisterSpecificationTools(s)
}

// RegisterResources registers all PLATEAU MCP resources
func RegisterResources(s *server.MCPServer) {
	// Register specification resources
	RegisterSpecificationResources(s)
}