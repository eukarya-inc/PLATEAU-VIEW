package datacatalogmcp

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/labstack/echo/v4"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// Service provides MCP server for PLATEAU data catalog
type Service struct {
	mcpServer    *server.MCPServer
	httpServer   *server.StreamableHTTPServer
	reposHandler ReposHandler
	host         string
}

// NewService creates a new MCP service
func NewService(reposHandler ReposHandler, host string) *Service {
	// Create MCP server
	mcpServer := server.NewMCPServer(
		"PLATEAU Data Catalog",
		"1.0.0",
		server.WithToolCapabilities(true),
	)

	s := &Service{
		mcpServer:    mcpServer,
		reposHandler: reposHandler,
		host:         host,
	}

	// Register tools
	s.registerTools()

	// Create Streamable HTTP server
	s.httpServer = server.NewStreamableHTTPServer(
		mcpServer,
		server.WithEndpointPath("/"),
		server.WithStateLess(true),        // Stateless mode for simplicity
		server.WithDisableStreaming(true), // Disable SSE, use single JSON response
	)

	return s
}

// RegisterRoutes registers MCP routes to the echo group
func (s *Service) RegisterRoutes(g *echo.Group) {
	// Mount Streamable HTTP server as a handler
	// The mcp-go library handles JSON-RPC, tools/list, tools/call automatically
	// Only POST is enabled; GET (SSE streaming) is disabled as we don't use server notifications
	handler := echo.WrapHandler(s.httpServer)
	g.POST("", handler)
}

// registerTools registers all MCP tools
func (s *Service) registerTools() {
	// Data Catalog Tools
	// Tool 1: plateau_get_metadata
	s.mcpServer.AddTool(s.createGetMetadataTool(), s.handleGetMetadata)

	// Tool 2: plateau_search_areas
	s.mcpServer.AddTool(s.createSearchAreasTool(), s.handleSearchAreas)

	// Tool 3: plateau_get_area
	s.mcpServer.AddTool(s.createGetAreaTool(), s.handleGetArea)

	// Tool 4: plateau_search_datasets
	s.mcpServer.AddTool(s.createSearchDatasetsTool(), s.handleSearchDatasets)

	// Tool 5: plateau_get_dataset
	s.mcpServer.AddTool(s.createGetDatasetTool(), s.handleGetDataset)

	// Tool 6: plateau_list_dataset_types
	s.mcpServer.AddTool(s.createListDatasetTypesTool(), s.handleListDatasetTypes)

	// CityGML Tools
	// Tool 7: plateau_citygml_get_attributes
	s.mcpServer.AddTool(s.createCityGMLGetAttributesTool(), s.handleCityGMLGetAttributes)

	// Tool 8: plateau_citygml_get_features
	s.mcpServer.AddTool(s.createCityGMLGetFeaturesTool(), s.handleCityGMLGetFeatures)

	// Tool 9: plateau_citygml_get_geoid_height
	s.mcpServer.AddTool(s.createCityGMLGetGeoidHeightTool(), s.handleCityGMLGetGeoidHeight)

	// Tool 10: plateau_get_citygml_files
	s.mcpServer.AddTool(s.createGetCityGMLFilesTool(), s.handleGetCityGMLFiles)

	// Note: plateau_citygml_get_spatialid_attributes is not implemented yet
	// due to import cycle issues with citygml package
}

// createGetMetadataTool creates the plateau_get_metadata tool definition
func (s *Service) createGetMetadataTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_get_metadata",
		mcp.WithDescription("PLATEAU全体のメタデータを取得します。利用可能な年度、PLATEAU仕様バージョン、地域数、データセット数などの統計情報を返します。"),
	)
}

// handleGetMetadata handles plateau_get_metadata tool calls
func (s *Service) handleGetMetadata(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get repo from context
	repo, err := s.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Get metadata using transform function
	resp, err := TransformMetadata(ctx, repo)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Convert to JSON and return
	return convertToToolResult(resp)
}

// createSearchAreasTool creates the plateau_search_areas tool definition
func (s *Service) createSearchAreasTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_search_areas",
		mcp.WithDescription("地域（都道府県、市区町村）を検索します。"),
		mcp.WithString("parent_code",
			mcp.Description("親地域コード（例: \"13\" で東京都）"),
		),
		mcp.WithArray("dataset_types",
			mcp.Description("データセット種類コードのリスト"),
		),
		mcp.WithArray("categories",
			mcp.Description("カテゴリ（PLATEAU, RELATED, GENERIC）"),
		),
		mcp.WithArray("area_types",
			mcp.Description("地域タイプ（PREFECTURE, CITY, WARD）"),
		),
		mcp.WithString("search_text",
			mcp.Description("検索文字列"),
		),
		mcp.WithBoolean("include_parents",
			mcp.Description("親地域を含めるか"),
		),
		mcp.WithBoolean("include_empty",
			mcp.Description("データセットがない地域も含めるか"),
		),
		mcp.WithBoolean("deep",
			mcp.Description("深い階層まで検索するか"),
		),
	)
}

// handleSearchAreas handles plateau_search_areas tool calls
func (s *Service) handleSearchAreas(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get repo from context
	repo, err := s.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Parse input parameters
	input := &SearchAreasInput{}
	if parentCode := request.GetString("parent_code", ""); parentCode != "" {
		input.ParentCode = &parentCode
	}
	if datasetTypes := request.GetStringSlice("dataset_types", nil); len(datasetTypes) > 0 {
		input.DatasetTypes = datasetTypes
	}
	if categories := request.GetStringSlice("categories", nil); len(categories) > 0 {
		input.Categories = categories
	}
	if areaTypes := request.GetStringSlice("area_types", nil); len(areaTypes) > 0 {
		input.AreaTypes = areaTypes
	}
	if searchText := request.GetString("search_text", ""); searchText != "" {
		input.SearchText = &searchText
	}
	if includeParents, err := request.RequireBool("include_parents"); err == nil {
		input.IncludeParents = &includeParents
	}
	if includeEmpty, err := request.RequireBool("include_empty"); err == nil {
		input.IncludeEmpty = &includeEmpty
	}
	if deep, err := request.RequireBool("deep"); err == nil {
		input.Deep = &deep
	}

	// Convert to plateauapi input
	areasInput := convertToAreasInput(input)

	// Get areas from repo
	areas, err := repo.Areas(ctx, areasInput)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Transform to response
	resp := TransformSearchAreas(areas, input)

	// Convert to JSON and return
	return convertToToolResult(resp)
}

// createGetAreaTool creates the plateau_get_area tool definition
func (s *Service) createGetAreaTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_get_area",
		mcp.WithDescription("特定の地域の詳細情報を取得します。"),
		mcp.WithString("code",
			mcp.Required(),
			mcp.Description("地域コード（例: \"13101\"）"),
		),
	)
}

// handleGetArea handles plateau_get_area tool calls
func (s *Service) handleGetArea(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get repo from context
	repo, err := s.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Get code parameter (required)
	code, err := request.RequireString("code")
	if err != nil {
		return mcp.NewToolResultError("code parameter is required"), nil
	}

	// Get area from repo
	area, err := repo.Area(ctx, plateauapi.AreaCode(code))
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}
	if area == nil {
		return mcp.NewToolResultError("area not found"), nil
	}

	// Transform to response
	resp := TransformGetArea(area)

	// Convert to JSON and return
	return convertToToolResult(resp)
}

// createSearchDatasetsTool creates the plateau_search_datasets tool definition
func (s *Service) createSearchDatasetsTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_search_datasets",
		mcp.WithDescription("データセットを検索します。"),
		mcp.WithArray("area_codes",
			mcp.Description("地域コードのリスト"),
		),
		mcp.WithArray("dataset_types",
			mcp.Description("データセット種類コードのリスト"),
		),
		mcp.WithArray("categories",
			mcp.Description("カテゴリ（PLATEAU, RELATED, GENERIC）"),
		),
		mcp.WithString("plateau_spec",
			mcp.Description("PLATEAU仕様バージョン"),
		),
		mcp.WithNumber("year",
			mcp.Description("整備年度"),
		),
		mcp.WithNumber("registration_year",
			mcp.Description("登録年度"),
		),
		mcp.WithString("search_text",
			mcp.Description("検索文字列"),
		),
		mcp.WithBoolean("shallow",
			mcp.Description("詳細情報を省略するか"),
		),
	)
}

// handleSearchDatasets handles plateau_search_datasets tool calls
func (s *Service) handleSearchDatasets(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get repo from context
	repo, err := s.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Parse input parameters
	input := &SearchDatasetsInput{}
	if areaCodes := request.GetStringSlice("area_codes", nil); len(areaCodes) > 0 {
		input.AreaCodes = areaCodes
	}
	if datasetTypes := request.GetStringSlice("dataset_types", nil); len(datasetTypes) > 0 {
		input.DatasetTypes = datasetTypes
	}
	if categories := request.GetStringSlice("categories", nil); len(categories) > 0 {
		input.Categories = categories
	}
	if plateauSpec := request.GetString("plateau_spec", ""); plateauSpec != "" {
		input.PlateauSpec = &plateauSpec
	}
	if year := request.GetInt("year", 0); year != 0 {
		input.Year = &year
	}
	if regYear := request.GetInt("registration_year", 0); regYear != 0 {
		input.RegistrationYear = &regYear
	}
	if searchText := request.GetString("search_text", ""); searchText != "" {
		input.SearchText = &searchText
	}
	if shallow := request.GetBool("shallow", false); shallow {
		input.Shallow = &shallow
	}

	// Convert to plateauapi input
	datasetsInput := convertToDatasetsInput(input)

	// Get datasets from repo
	datasets, err := repo.Datasets(ctx, datasetsInput)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Transform to response
	resp := TransformSearchDatasets(datasets, input)

	// Convert to JSON and return
	return convertToToolResult(resp)
}

// createGetDatasetTool creates the plateau_get_dataset tool definition
func (s *Service) createGetDatasetTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_get_dataset",
		mcp.WithDescription("特定のデータセットの詳細情報を取得します。"),
		mcp.WithString("id",
			mcp.Required(),
			mcp.Description("データセットID"),
		),
	)
}

// handleGetDataset handles plateau_get_dataset tool calls
func (s *Service) handleGetDataset(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get repo from context
	repo, err := s.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Get id parameter (required)
	id, err := request.RequireString("id")
	if err != nil {
		return mcp.NewToolResultError("id parameter is required"), nil
	}

	// Get dataset from repo
	node, err := repo.Node(ctx, plateauapi.ID(id))
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	dataset, ok := node.(plateauapi.Dataset)
	if !ok || dataset == nil {
		return mcp.NewToolResultError("dataset not found"), nil
	}

	// Transform to response
	resp := TransformGetDataset(dataset)

	// Convert to JSON and return
	return convertToToolResult(resp)
}

// createListDatasetTypesTool creates the plateau_list_dataset_types tool definition
func (s *Service) createListDatasetTypesTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_list_dataset_types",
		mcp.WithDescription("データセット種類の一覧を取得します。"),
		mcp.WithString("category",
			mcp.Description("カテゴリ（PLATEAU, RELATED, GENERIC）"),
		),
		mcp.WithString("plateau_spec",
			mcp.Description("PLATEAU仕様バージョン"),
		),
		mcp.WithNumber("year",
			mcp.Description("対象年度"),
		),
	)
}

// handleListDatasetTypes handles plateau_list_dataset_types tool calls
func (s *Service) handleListDatasetTypes(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get repo from context
	repo, err := s.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Parse input parameters
	input := &ListDatasetTypesInput{}
	if category := request.GetString("category", ""); category != "" {
		input.Category = &category
	}
	if plateauSpec := request.GetString("plateau_spec", ""); plateauSpec != "" {
		input.PlateauSpec = &plateauSpec
	}
	if year := request.GetInt("year", 0); year != 0 {
		input.Year = &year
	}

	// Convert to plateauapi input
	typesInput := convertToDatasetTypesInput(input)

	// Get dataset types from repo
	types, err := repo.DatasetTypes(ctx, typesInput)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	// Transform to response
	resp := TransformListDatasetTypes(types)

	// Convert to JSON and return
	return convertToToolResult(resp)
}

// Helper functions

// getRepoFromContext extracts CMS metadata from context and gets the merged repo
func (s *Service) getRepoFromContext(ctx context.Context) (plateauapi.Repo, error) {
	// Extract metadata from context (set by ReposHandler middleware)
	metadata := plateaucms.GetAllCMSMetadataFromContext(ctx)
	if len(metadata) == 0 {
		return nil, fmt.Errorf("metadata not found in context")
	}

	// Get merged repo using ReposHandler
	repo := s.reposHandler.PrepareAndGetMergedRepo(ctx, "", metadata)
	if repo == nil {
		return nil, fmt.Errorf("failed to get repo")
	}

	return repo, nil
}

// convertToToolResult converts any response to MCP tool result
func convertToToolResult(data interface{}) (*mcp.CallToolResult, error) {
	jsonBytes, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		return mcp.NewToolResultError("failed to marshal response"), nil
	}
	return mcp.NewToolResultText(string(jsonBytes)), nil
}
