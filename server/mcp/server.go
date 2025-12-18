package mcp

import (
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogmcp"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/mcp/plateauspecmcp"
	"github.com/mark3labs/mcp-go/server"
)

// NewServer creates and configures a new comprehensive PLATEAU MCP server
// This creates a basic server with only specification tools
func NewServer() *server.MCPServer {
	return NewServerWithConfig(nil)
}

// NewServerWithConfig creates a new comprehensive PLATEAU MCP server with configuration
func NewServerWithConfig(cfg *Config) *server.MCPServer {
	s := server.NewMCPServer(
		"plateau-mcp",
		"1.0.0",
		server.WithToolCapabilities(true),
		server.WithResourceCapabilities(true, false),
		server.WithLogging(),
	)

	// Register all PLATEAU tools and resources
	RegisterToolsWithConfig(s, cfg)
	RegisterResources(s)

	return s
}

// RegisterTools registers all PLATEAU MCP tools (for backward compatibility)
func RegisterTools(s *server.MCPServer) {
	RegisterToolsWithConfig(s, nil)
}

// RegisterToolsWithConfig registers all PLATEAU MCP tools with configuration
func RegisterToolsWithConfig(s *server.MCPServer, cfg *Config) {
	// Specification reading tools (always registered)
	plateauspecmcp.RegisterTools(s)

	// Data catalog tools (only if configured)
	if cfg != nil && cfg.DataCatalogReposHandler != nil {
		registrar := datacatalogmcp.NewToolRegistrar(cfg.DataCatalogReposHandler, cfg.Host)
		registrar.RegisterAllTools(s)
	}
}

// RegisterResources registers all PLATEAU MCP resources
func RegisterResources(s *server.MCPServer) {
	// Register specification resources
	plateauspecmcp.RegisterResources(s)

	// Register datacatalog resources (spatial ID documentation, etc.)
	datacatalogmcp.RegisterResources(s)
}
