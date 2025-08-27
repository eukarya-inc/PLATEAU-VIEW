package mcp

import (
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// RegisterSpecificationTools registers PLATEAU specification reading tools
func RegisterSpecificationTools(s *server.MCPServer) {
	s.AddTool(specListTool, HandleSpecList)
	s.AddTool(specGetContentTool, HandleSpecGetContent)
	s.AddTool(specGetContentsBatchTool, HandleSpecGetContentsBatch)
	s.AddTool(specSearchTool, HandleSpecSearch)
}

// Specification reading tools with spec_ prefix
var specListTool = mcp.NewTool("plateau_spec_list",
	mcp.WithDescription("Navigate PLATEAU 3D City Model specification documents. Browse chapters, sections, and subsections of both Standard Product Specification and Standard Work Procedures."),
	mcp.WithString("path",
		mcp.Description("Navigation path (e.g., '' for root, 'toc4' for chapter, '/toc4/toc4_02' for section)"),
	),
	mcp.WithBoolean("recursive",
		mcp.Description("Show all subsections at once. Default false for step-by-step navigation."),
	),
	mcp.WithString("document_type",
		mcp.Description("Document type: 'standard' (default) or 'procedure'"),
	),
	mcp.WithString("format",
		mcp.Description("Output format: 'tree' (default) or 'json'"),
	),
	mcp.WithNumber("offset",
		mcp.Description("Pagination offset (default: 0)"),
	),
	mcp.WithNumber("limit",
		mcp.Description("Maximum results (default: 100)"),
	),
)

var specGetContentTool = mcp.NewTool("plateau_spec_get_content",
	mcp.WithDescription("Retrieve the actual content of a specific section from PLATEAU specification documents."),
	mcp.WithString("path",
		mcp.Required(),
		mcp.Description("Exact path from list results (e.g., '/plateaudocument/toc4/toc4_03/toc4_03_01')"),
	),
	mcp.WithString("format",
		mcp.Description("Output format: 'markdown' (default), 'json', or 'html'"),
	),
	mcp.WithString("document_type",
		mcp.Description("Document type: 'standard' (default) or 'procedure'"),
	),
)

var specGetContentsBatchTool = mcp.NewTool("plateau_spec_get_contents_batch",
	mcp.WithDescription("Efficiently retrieve multiple content sections at once from PLATEAU specification documents."),
	mcp.WithString("paths",
		mcp.Required(),
		mcp.Description("JSON array of paths or comma-separated string"),
	),
	mcp.WithString("format",
		mcp.Description("Output format: 'markdown' (default), 'json', or 'html'"),
	),
	mcp.WithString("document_type",
		mcp.Description("Document type: 'standard' (default) or 'procedure'"),
	),
	mcp.WithNumber("offset",
		mcp.Description("Starting offset for pagination (default: 0)"),
	),
	mcp.WithNumber("limit",
		mcp.Description("Maximum paths to process (default: 10)"),
	),
)

var specSearchTool = mcp.NewTool("plateau_spec_search",
	mcp.WithDescription("Search for specific terms or topics across PLATEAU specification documents."),
	mcp.WithString("query",
		mcp.Required(),
		mcp.Description("Search query text"),
	),
	mcp.WithString("document_type",
		mcp.Description("Document type: 'standard' (default), 'procedure', or 'all'"),
	),
	mcp.WithString("scope",
		mcp.Description("Search scope: 'titles' (default), 'content', or 'all'"),
	),
	mcp.WithNumber("limit",
		mcp.Description("Maximum results (default: 20)"),
	),
)