package mcp

import (
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogmcp"
)

// Config holds configuration for the comprehensive MCP server
type Config struct {
	// Host is the base URL for API calls (e.g., "https://api.plateauview.mlit.go.jp")
	Host string

	// DataCatalogReposHandler is the handler for data catalog operations
	// If nil, data catalog tools will not be registered
	DataCatalogReposHandler datacatalogmcp.ReposHandler
}
