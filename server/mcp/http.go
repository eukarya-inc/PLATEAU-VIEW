package mcp

import (
	"github.com/labstack/echo/v4"
	"github.com/mark3labs/mcp-go/server"
)

// Global server instance to reuse across requests
var streamableServer *server.StreamableHTTPServer

func init() {
	// Create MCP server instance
	mcpServer := NewServer()
	
	// Create streamable HTTP server with stateless mode for simplicity
	streamableServer = server.NewStreamableHTTPServer(
		mcpServer,
		server.WithStateLess(true),
	)
}

// ServeHTTP creates an HTTP handler for the MCP server
func ServeHTTP(c echo.Context) error {
	// Use the streamable HTTP server to handle the request
	streamableServer.ServeHTTP(c.Response(), c.Request())
	return nil
}

// RegisterHTTPEndpoint registers the MCP HTTP endpoint with Echo
func RegisterHTTPEndpoint(g *echo.Group) {
	// Handle all MCP-related requests
	// The MCP server will be available at /mcp
	g.Any("", ServeHTTP)
	g.Any("/*", ServeHTTP)
}