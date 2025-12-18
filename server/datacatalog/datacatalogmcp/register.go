package datacatalogmcp

import (
	"context"
	"fmt"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// ToolRegistrar provides functions to register datacatalogmcp tools to an external MCPServer
type ToolRegistrar struct {
	reposHandler ReposHandler
	host         string
}

// NewToolRegistrar creates a new ToolRegistrar
func NewToolRegistrar(reposHandler ReposHandler, host string) *ToolRegistrar {
	return &ToolRegistrar{
		reposHandler: reposHandler,
		host:         host,
	}
}

// RegisterAllTools registers all datacatalogmcp tools to the given MCPServer
func (r *ToolRegistrar) RegisterAllTools(s *server.MCPServer) {
	r.RegisterDataCatalogTools(s)
	r.RegisterCityGMLTools(s)
	r.RegisterHelperTools(s)
}

// RegisterDataCatalogTools registers data catalog tools (plateau_get_metadata, plateau_search_areas, etc.)
// These tools require CMS metadata in context
func (r *ToolRegistrar) RegisterDataCatalogTools(s *server.MCPServer) {
	// Tool 1: plateau_get_metadata
	s.AddTool(r.createGetMetadataTool(), r.handleGetMetadata)

	// Tool 2: plateau_search_areas
	s.AddTool(r.createSearchAreasTool(), r.handleSearchAreas)

	// Tool 3: plateau_get_area
	s.AddTool(r.createGetAreaTool(), r.handleGetArea)

	// Tool 4: plateau_search_datasets
	s.AddTool(r.createSearchDatasetsTool(), r.handleSearchDatasets)

	// Tool 5: plateau_get_dataset
	s.AddTool(r.createGetDatasetTool(), r.handleGetDataset)

	// Tool 6: plateau_list_dataset_types
	s.AddTool(r.createListDatasetTypesTool(), r.handleListDatasetTypes)
}

// RegisterCityGMLTools registers CityGML tools (plateau_citygml_get_attributes, etc.)
// These tools call the internal CityGML API
func (r *ToolRegistrar) RegisterCityGMLTools(s *server.MCPServer) {
	// Tool 7: plateau_citygml_get_attributes
	s.AddTool(r.createCityGMLGetAttributesTool(), r.handleCityGMLGetAttributes)

	// Tool 8: plateau_citygml_get_features
	s.AddTool(r.createCityGMLGetFeaturesTool(), r.handleCityGMLGetFeatures)

	// Tool 9: plateau_citygml_get_geoid_height
	s.AddTool(r.createCityGMLGetGeoidHeightTool(), r.handleCityGMLGetGeoidHeight)

	// Tool 10: plateau_get_citygml_files
	s.AddTool(r.createGetCityGMLFilesTool(), r.handleGetCityGMLFiles)
}

// RegisterHelperTools registers helper tools (plateau_explain_spatial_id)
func (r *ToolRegistrar) RegisterHelperTools(s *server.MCPServer) {
	// Tool 11: plateau_explain_spatial_id
	s.AddTool(r.createExplainSpatialIDTool(), r.handleExplainSpatialID)
}

// Tool creation functions (delegate to the same implementations as Service)

func (r *ToolRegistrar) createGetMetadataTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_get_metadata",
		mcp.WithDescription("PLATEAU全体のメタデータを取得します。利用可能な年度、PLATEAU仕様バージョン、地域数、データセット数などの統計情報を返します。"),
		mcp.WithReadOnlyHintAnnotation(true),
	)
}

func (r *ToolRegistrar) handleGetMetadata(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	repo, err := r.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	resp, err := TransformMetadata(ctx, repo)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	return convertToToolResult(resp)
}

func (r *ToolRegistrar) createSearchAreasTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_search_areas",
		mcp.WithDescription("地域（都道府県、市区町村）を検索します。"),
		mcp.WithReadOnlyHintAnnotation(true),
		mcp.WithString("parent_code",
			mcp.Description("親地域コード（例: \"13\" で東京都）"),
		),
		mcp.WithArray("dataset_types",
			mcp.Description("データセット種類コードのリスト"),
			mcp.WithStringItems(),
		),
		mcp.WithArray("categories",
			mcp.Description("カテゴリ（PLATEAU, RELATED, GENERIC）"),
			mcp.WithStringItems(),
		),
		mcp.WithArray("area_types",
			mcp.Description("地域タイプ（PREFECTURE, CITY, WARD）"),
			mcp.WithStringItems(),
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

func (r *ToolRegistrar) handleSearchAreas(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	repo, err := r.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

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

	areasInput := convertToAreasInput(input)
	areas, err := repo.Areas(ctx, areasInput)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	resp := TransformSearchAreas(areas, input)
	return convertToToolResult(resp)
}

func (r *ToolRegistrar) createGetAreaTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_get_area",
		mcp.WithDescription("特定の地域の詳細情報を取得します。"),
		mcp.WithReadOnlyHintAnnotation(true),
		mcp.WithString("code",
			mcp.Required(),
			mcp.Description("地域コード（例: \"13101\"）"),
		),
	)
}

func (r *ToolRegistrar) handleGetArea(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	repo, err := r.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	code, err := request.RequireString("code")
	if err != nil {
		return mcp.NewToolResultError("code parameter is required"), nil
	}

	area, err := repo.Area(ctx, plateauapi.AreaCode(code))
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}
	if area == nil {
		return mcp.NewToolResultError("area not found"), nil
	}

	resp := TransformGetArea(area)
	return convertToToolResult(resp)
}

func (r *ToolRegistrar) createSearchDatasetsTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_search_datasets",
		mcp.WithDescription("データセットを検索します。"),
		mcp.WithReadOnlyHintAnnotation(true),
		mcp.WithArray("area_codes",
			mcp.Description("地域コードのリスト"),
			mcp.WithStringItems(),
		),
		mcp.WithArray("dataset_types",
			mcp.Description("データセット種類コードのリスト"),
			mcp.WithStringItems(),
		),
		mcp.WithArray("categories",
			mcp.Description("カテゴリ（PLATEAU, RELATED, GENERIC）"),
			mcp.WithStringItems(),
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

func (r *ToolRegistrar) handleSearchDatasets(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	repo, err := r.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

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

	datasetsInput := convertToDatasetsInput(input)
	datasets, err := repo.Datasets(ctx, datasetsInput)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	resp := TransformSearchDatasets(datasets, input)
	return convertToToolResult(resp)
}

func (r *ToolRegistrar) createGetDatasetTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_get_dataset",
		mcp.WithDescription("特定のデータセットの詳細情報を取得します。"),
		mcp.WithReadOnlyHintAnnotation(true),
		mcp.WithString("id",
			mcp.Required(),
			mcp.Description("データセットID"),
		),
	)
}

func (r *ToolRegistrar) handleGetDataset(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	repo, err := r.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	id, err := request.RequireString("id")
	if err != nil {
		return mcp.NewToolResultError("id parameter is required"), nil
	}

	node, err := repo.Node(ctx, plateauapi.ID(id))
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	dataset, ok := node.(plateauapi.Dataset)
	if !ok || dataset == nil {
		return mcp.NewToolResultError("dataset not found"), nil
	}

	resp := TransformGetDataset(dataset)
	return convertToToolResult(resp)
}

func (r *ToolRegistrar) createListDatasetTypesTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_list_dataset_types",
		mcp.WithDescription("データセット種類の一覧を取得します。"),
		mcp.WithReadOnlyHintAnnotation(true),
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

func (r *ToolRegistrar) handleListDatasetTypes(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	repo, err := r.getRepoFromContext(ctx)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

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

	typesInput := convertToDatasetTypesInput(input)
	types, err := repo.DatasetTypes(ctx, typesInput)
	if err != nil {
		return mcp.NewToolResultError(err.Error()), nil
	}

	resp := TransformListDatasetTypes(types)
	return convertToToolResult(resp)
}

// CityGML tool creation functions

func (r *ToolRegistrar) getCityGMLAPIBaseURL() string {
	host := r.host
	if host == "" {
		host = "http://localhost:8080"
	}
	return host + "/citygml"
}

func (r *ToolRegistrar) createCityGMLGetAttributesTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_citygml_get_attributes",
		mcp.WithDescription("CityGMLファイルから指定した建物IDの属性情報を取得します。\n\n使い方の流れ:\n1. plateau_get_citygml_filesでメッシュコードや空間IDを指定してCityGML URLを取得\n2. レスポンスのcities[].files[type][].urlからCityGML URLを取得（typeは'bldg', 'tran'など）\n3. そのURLと建物IDをこのツールに渡す"),
		mcp.WithReadOnlyHintAnnotation(true),
		mcp.WithString("url",
			mcp.Required(),
			mcp.Description("CityGMLファイルのURL"),
		),
		mcp.WithArray("building_ids",
			mcp.Required(),
			mcp.Description("取得する建物IDのリスト"),
			mcp.WithStringItems(),
		),
		mcp.WithBoolean("skip_code_list",
			mcp.Description("コードリストの取得をスキップするか（デフォルト: false）"),
		),
	)
}

func (r *ToolRegistrar) handleCityGMLGetAttributes(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Delegate to shared handler with host
	return handleCityGMLGetAttributesWithHost(ctx, request, r.getCityGMLAPIBaseURL())
}

func (r *ToolRegistrar) createCityGMLGetFeaturesTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_citygml_get_features",
		mcp.WithDescription("CityGMLファイルから指定した空間ID（SpatialID）に交差する地物のIDリストを取得します。\n\n使い方の流れ:\n1. plateau_get_citygml_filesでメッシュコードや空間IDを指定してCityGML URLを取得\n2. レスポンスのcities[].files[type][].urlからCityGML URLを取得（typeは'bldg', 'tran'など）\n3. そのURLと空間IDをこのツールに渡す\n4. 返ってきた地物IDをplateau_citygml_get_attributesで使用できる"),
		mcp.WithReadOnlyHintAnnotation(true),
		mcp.WithString("url",
			mcp.Required(),
			mcp.Description("CityGMLファイルのURL"),
		),
		mcp.WithArray("spatial_ids",
			mcp.Required(),
			mcp.Description("検索する空間IDのリスト"),
			mcp.WithStringItems(),
		),
	)
}

func (r *ToolRegistrar) handleCityGMLGetFeatures(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	return handleCityGMLGetFeaturesWithHost(ctx, request, r.getCityGMLAPIBaseURL())
}

func (r *ToolRegistrar) createCityGMLGetGeoidHeightTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_citygml_get_geoid_height",
		mcp.WithDescription("指定した緯度経度のジオイド高を取得します。日本のジオイド2011に基づいています。標高の楕円体高と正標高の変換に使用できます。"),
		mcp.WithReadOnlyHintAnnotation(true),
		mcp.WithNumber("latitude",
			mcp.Required(),
			mcp.Description("緯度（度）"),
		),
		mcp.WithNumber("longitude",
			mcp.Required(),
			mcp.Description("経度（度）"),
		),
	)
}

func (r *ToolRegistrar) handleCityGMLGetGeoidHeight(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	return handleCityGMLGetGeoidHeightWithHost(ctx, request, r.getCityGMLAPIBaseURL())
}

func (r *ToolRegistrar) createGetCityGMLFilesTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_get_citygml_files",
		mcp.WithDescription("指定した条件でCityGMLファイルを検索します。メッシュコード、空間ID、または矩形範囲で検索できます。\\n\\n条件フォーマット:\\n- メッシュコード: m:53393580,53393581 (カンマ区切りで複数指定可)\\n- 空間ID: s:15/0/29134/12950,15/0/29134/12951 (カンマ区切りで複数指定可)\\n- 矩形範囲: r:139.7,35.6,139.8,35.7 (西経度,南緯度,東経度,北緯度)\\n\\n地物型フィルタ:\\nfeature_typesパラメータで地物型を絞り込むことができます（例: [\\\"bldg\\\", \\\"tran\\\"]）\\n主な地物型: bldg(建築物), tran(交通), luse(土地利用), dem(地形), fld(洪水), lsld(土砂災害), urf(都市計画)\\n利用可能な全ての地物型はplateau_list_dataset_typesツールで取得できます。\\n\\n使い方の流れ:\\n1. このツールでCityGMLファイルURLを取得\\n2. 取得したURLをplateau_citygml_get_featuresまたはplateau_citygml_get_attributesで使用"),
		mcp.WithReadOnlyHintAnnotation(true),
		mcp.WithString("condition",
			mcp.Required(),
			mcp.Description("検索条件 (例: \\\"m:53393580\\\", \\\"s:15/0/29134/12950\\\", \\\"r:139.7,35.6,139.8,35.7\\\")"),
		),
		mcp.WithArray("feature_types",
			mcp.Description("取得する地物型のリスト (例: [\\\"bldg\\\", \\\"tran\\\"])。指定しない場合は全ての地物型を取得"),
			mcp.WithStringItems(),
		),
	)
}

func (r *ToolRegistrar) handleGetCityGMLFiles(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	return handleGetCityGMLFilesWithHost(ctx, request, r.host)
}

// Helper tool creation functions

func (r *ToolRegistrar) createExplainSpatialIDTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_explain_spatial_id",
		mcp.WithDescription("空間ID（Spatial ID）の仕様と使い方を解説します。空間IDは3次元空間を一意に識別するための規格で、PLATEAUのCityGMLツールで使用されます。"),
		mcp.WithReadOnlyHintAnnotation(true),
	)
}

func (r *ToolRegistrar) handleExplainSpatialID(_ context.Context, _ mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	return mcp.NewToolResultText(spatialIDExplanation), nil
}

// getRepoFromContext extracts CMS metadata from context and gets the merged repo
func (r *ToolRegistrar) getRepoFromContext(ctx context.Context) (plateauapi.Repo, error) {
	metadata := plateaucms.GetAllCMSMetadataFromContext(ctx)
	if len(metadata) == 0 {
		return nil, fmt.Errorf("metadata not found in context")
	}

	repo := r.reposHandler.PrepareAndGetMergedRepo(ctx, "", metadata)
	if repo == nil {
		return nil, fmt.Errorf("failed to get repo")
	}

	return repo, nil
}
