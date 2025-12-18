package mcp

import (
	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
	"github.com/mark3labs/mcp-go/server"
)

// HTTPHandler handles MCP HTTP requests
type HTTPHandler struct {
	streamableServer *server.StreamableHTTPServer
	cmsMiddleware    echo.MiddlewareFunc
}

// NewHTTPHandler creates a new HTTP handler for MCP
func NewHTTPHandler(cfg *Config) *HTTPHandler {
	mcpServer := NewServerWithConfig(cfg)

	streamableServer := server.NewStreamableHTTPServer(
		mcpServer,
		server.WithStateLess(true),
	)

	return &HTTPHandler{
		streamableServer: streamableServer,
	}
}

// SetCMSMiddleware sets the CMS middleware for injecting metadata into context
func (h *HTTPHandler) SetCMSMiddleware(m echo.MiddlewareFunc) {
	h.cmsMiddleware = m
}

// ServeHTTP handles MCP HTTP requests
func (h *HTTPHandler) ServeHTTP(c echo.Context) error {
	h.streamableServer.ServeHTTP(c.Response(), c.Request())
	return nil
}

// RegisterRoutes registers MCP routes to the echo group
func (h *HTTPHandler) RegisterRoutes(g *echo.Group) {
	// Apply middlewares
	g.Use(middleware.CORS())
	if h.cmsMiddleware != nil {
		g.Use(h.cmsMiddleware)
	}

	// Handle all MCP-related requests
	g.Any("", h.ServeHTTP)
	g.Any("/*", h.ServeHTTP)
}

// Global server instance for backward compatibility (without datacatalog tools)
var defaultHandler *HTTPHandler

func init() {
	defaultHandler = NewHTTPHandler(nil)
}

// ServeHTTP creates an HTTP handler for the MCP server (for backward compatibility)
func ServeHTTP(c echo.Context) error {
	return defaultHandler.ServeHTTP(c)
}

// RegisterHTTPEndpoint registers the MCP HTTP endpoint with Echo (for backward compatibility)
// This registers only spec tools. For full functionality, use NewHTTPHandler with Config.
func RegisterHTTPEndpoint(g *echo.Group) {
	g.Any("", ServeHTTP)
	g.Any("/*", ServeHTTP)
}
