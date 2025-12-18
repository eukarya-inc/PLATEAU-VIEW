package datacatalogmcp

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/reearth/reearthx/log"
)

var httpClient = &http.Client{
	Timeout: 60 * time.Second,
}

// getCityGMLAPIBaseURL returns the CityGML API base URL based on the host
func (s *Service) getCityGMLAPIBaseURL() string {
	// Use the host from service configuration
	// Host should be like "http://localhost:8080" or "https://api.plateauview.mlit.go.jp"
	host := s.host
	if host == "" {
		host = "http://localhost:8080"
	}
	return host + "/citygml"
}

// createCityGMLGetAttributesTool creates the plateau_citygml_get_attributes tool definition
func (s *Service) createCityGMLGetAttributesTool() mcp.Tool {
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

// handleCityGMLGetAttributes handles plateau_citygml_get_attributes tool calls
func (s *Service) handleCityGMLGetAttributes(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get URL parameter (required)
	citygmlURL, err := request.RequireString("url")
	if err != nil {
		return mcp.NewToolResultError("url parameter is required"), nil
	}

	// Validate URL
	u, err := url.Parse(citygmlURL)
	if err != nil {
		return mcp.NewToolResultError("invalid url format"), nil
	}
	if u.Scheme != "http" && u.Scheme != "https" {
		return mcp.NewToolResultError("url must be http or https"), nil
	}

	// Get building_ids parameter (required)
	buildingIDs := request.GetStringSlice("building_ids", nil)
	if len(buildingIDs) == 0 {
		return mcp.NewToolResultError("building_ids parameter is required and must not be empty"), nil
	}

	// Get skip_code_list parameter (optional)
	skipCodeList := request.GetBool("skip_code_list", false)

	// Call internal citygml API
	apiURL := fmt.Sprintf("%s/attributes?url=%s&id=%s",
		s.getCityGMLAPIBaseURL(),
		url.QueryEscape(citygmlURL),
		url.QueryEscape(strings.Join(buildingIDs, ",")),
	)
	if skipCodeList {
		apiURL += "&skip_code_list_fetch=1"
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, apiURL, nil)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to create request: %v", err)
		return mcp.NewToolResultError("failed to create request"), nil
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to call API: %v", err)
		return mcp.NewToolResultError("failed to call citygml API"), nil
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return mcp.NewToolResultError(fmt.Sprintf("citygml API returned status %d", resp.StatusCode)), nil
	}

	// Parse response
	var attrs []map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&attrs); err != nil {
		log.Errorfc(ctx, "citygml: failed to decode response: %v", err)
		return mcp.NewToolResultError("failed to decode citygml API response"), nil
	}

	// Convert to response
	response := &GetCityGMLAttributesResponse{
		Attributes: attrs,
	}

	return convertToToolResult(response)
}

// createCityGMLGetFeaturesTool creates the plateau_citygml_get_features tool definition
func (s *Service) createCityGMLGetFeaturesTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_citygml_get_features",
		mcp.WithDescription("CityGMLファイルから指定した空間ID（SpatialID）に交差する地物のIDリストを取得します。\n\n**重要**: 空間IDの仕様と推奨設定については、まずplateau_explain_spatial_idツールを参照してください。建築物検索ではz=17〜19、f=0を推奨します。\n\n使い方の流れ:\n1. plateau_get_citygml_filesでメッシュコードや空間IDを指定してCityGML URLを取得\n2. レスポンスのcities[].files[type][].urlからCityGML URLを取得（typeは'bldg', 'tran'など）\n3. そのURLと空間IDをこのツールに渡す\n4. 返ってきた地物IDをplateau_citygml_get_attributesで使用できる"),
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

// handleCityGMLGetFeatures handles plateau_citygml_get_features tool calls
func (s *Service) handleCityGMLGetFeatures(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get URL parameter (required)
	citygmlURL, err := request.RequireString("url")
	if err != nil {
		return mcp.NewToolResultError("url parameter is required"), nil
	}

	// Validate URL
	u, err := url.Parse(citygmlURL)
	if err != nil {
		return mcp.NewToolResultError("invalid url format"), nil
	}
	if u.Scheme != "http" && u.Scheme != "https" {
		return mcp.NewToolResultError("url must be http or https"), nil
	}

	// Get spatial_ids parameter (required)
	spatialIDs := request.GetStringSlice("spatial_ids", nil)
	if len(spatialIDs) == 0 {
		return mcp.NewToolResultError("spatial_ids parameter is required and must not be empty"), nil
	}

	// Call internal citygml API
	apiURL := fmt.Sprintf("%s/features?url=%s&sid=%s",
		s.getCityGMLAPIBaseURL(),
		url.QueryEscape(citygmlURL),
		url.QueryEscape(strings.Join(spatialIDs, ",")),
	)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, apiURL, nil)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to create request: %v", err)
		return mcp.NewToolResultError("failed to create request"), nil
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to call API: %v", err)
		return mcp.NewToolResultError("failed to call citygml API"), nil
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return mcp.NewToolResultError(fmt.Sprintf("citygml API returned status %d", resp.StatusCode)), nil
	}

	// Parse response
	var result struct {
		FeatureIDs []string `json:"featureIds"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		log.Errorfc(ctx, "citygml: failed to decode response: %v", err)
		return mcp.NewToolResultError("failed to decode citygml API response"), nil
	}

	// Convert to response
	response := &GetCityGMLFeaturesResponse{
		FeatureIDs: result.FeatureIDs,
	}
	if response.FeatureIDs == nil {
		response.FeatureIDs = []string{}
	}

	// Add hint when no features found
	if len(response.FeatureIDs) == 0 {
		response.Reason = "NO_FEATURES_IN_AREA"
		response.Hint = &GetCityGMLFeaturesHint{
			Message:         "指定した空間IDの範囲に地物が見つかりませんでした。ズームレベル(z)を17〜19に、fインデックスを0にして再試行してください。詳細はplateau_explain_spatial_idツールを参照してください。",
			RecommendedZoom: []int{17, 18, 19},
		}
	}

	return convertToToolResult(response)
}

// createCityGMLGetGeoidHeightTool creates the plateau_citygml_get_geoid_height tool definition
func (s *Service) createCityGMLGetGeoidHeightTool() mcp.Tool {
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

// handleCityGMLGetGeoidHeight handles plateau_citygml_get_geoid_height tool calls
func (s *Service) handleCityGMLGetGeoidHeight(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get latitude parameter (required)
	lat, err := request.RequireFloat("latitude")
	if err != nil {
		return mcp.NewToolResultError("latitude parameter is required"), nil
	}

	// Get longitude parameter (required)
	lng, err := request.RequireFloat("longitude")
	if err != nil {
		return mcp.NewToolResultError("longitude parameter is required"), nil
	}

	// Validate coordinates
	if lat < -90 || lat > 90 {
		return mcp.NewToolResultError("latitude must be between -90 and 90"), nil
	}
	if lng < -180 || lng > 180 {
		return mcp.NewToolResultError("longitude must be between -180 and 180"), nil
	}

	// Call internal citygml API
	apiURL := fmt.Sprintf("%s/geoid_height?lat=%s&lng=%s",
		s.getCityGMLAPIBaseURL(),
		strconv.FormatFloat(lat, 'f', -1, 64),
		strconv.FormatFloat(lng, 'f', -1, 64),
	)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, apiURL, nil)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to create request: %v", err)
		return mcp.NewToolResultError("failed to create request"), nil
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to call API: %v", err)
		return mcp.NewToolResultError("failed to call citygml API"), nil
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return mcp.NewToolResultError(fmt.Sprintf("citygml API returned status %d", resp.StatusCode)), nil
	}

	// Parse response
	var result struct {
		Lat         float64 `json:"lat"`
		Lng         float64 `json:"lng"`
		GeoidHeight float64 `json:"geoid_height"`
		Geoid       string  `json:"geoid"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		log.Errorfc(ctx, "citygml: failed to decode response: %v", err)
		return mcp.NewToolResultError("failed to decode citygml API response"), nil
	}

	// Convert to response
	response := &GetGeoidHeightResponse{
		Latitude:    result.Lat,
		Longitude:   result.Lng,
		GeoidHeight: result.GeoidHeight,
		Geoid:       result.Geoid,
	}

	return convertToToolResult(response)
}

// createGetCityGMLFilesTool creates the plateau_get_citygml_files tool definition
func (s *Service) createGetCityGMLFilesTool() mcp.Tool {
	return mcp.NewTool(
		"plateau_get_citygml_files",
		mcp.WithDescription("指定した条件でCityGMLファイルを検索します。メッシュコード、空間ID、または矩形範囲で検索できます。\\n\\n**重要**: 空間IDを使用する場合は、まずplateau_explain_spatial_idツールで仕様と推奨設定を確認してください。\\n\\n条件フォーマット:\\n- メッシュコード: m:53393580,53393581 (カンマ区切りで複数指定可)\\n- 空間ID: s:15/0/29134/12950,15/0/29134/12951 (カンマ区切りで複数指定可)\\n- 矩形範囲: r:139.7,35.6,139.8,35.7 (西経度,南緯度,東経度,北緯度)\\n\\n地物型フィルタ:\\nfeature_typesパラメータで地物型を絞り込むことができます（例: [\\\"bldg\\\", \\\"tran\\\"]）\\n主な地物型: bldg(建築物), tran(交通), luse(土地利用), dem(地形), fld(洪水), lsld(土砂災害), urf(都市計画)\\n利用可能な全ての地物型はplateau_list_dataset_typesツールで取得できます。\\n\\n使い方の流れ:\\n1. このツールでCityGMLファイルURLを取得\\n2. 取得したURLをplateau_citygml_get_featuresまたはplateau_citygml_get_attributesで使用"),
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

// handleGetCityGMLFiles handles plateau_get_citygml_files tool calls
func (s *Service) handleGetCityGMLFiles(ctx context.Context, request mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	// Get condition parameter (required)
	condition, err := request.RequireString("condition")
	if err != nil {
		return mcp.NewToolResultError("condition parameter is required"), nil
	}

	// Validate condition format
	if condition == "" {
		return mcp.NewToolResultError("condition must not be empty"), nil
	}
	// Check if condition starts with m:, s:, or r:
	if !strings.HasPrefix(condition, "m:") && !strings.HasPrefix(condition, "s:") && !strings.HasPrefix(condition, "r:") {
		return mcp.NewToolResultError("condition must start with m:, s:, or r:"), nil
	}

	// Get feature_types parameter (optional)
	featureTypes := request.GetStringSlice("feature_types", nil)

	// Call internal datacatalog citygml API
	// Note: This endpoint is different from other citygml endpoints
	// It's mounted at /datacatalog/citygml/:conditions instead of /citygml
	host := s.host
	if host == "" {
		host = "http://localhost:8080"
	}
	apiURL := fmt.Sprintf("%s/datacatalog/citygml/%s",
		host,
		url.PathEscape(condition),
	)

	// Add feature_types query parameter if specified
	if len(featureTypes) > 0 {
		apiURL += "?types=" + url.QueryEscape(strings.Join(featureTypes, ","))
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, apiURL, nil)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to create request: %v", err)
		return mcp.NewToolResultError("failed to create request"), nil
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to call API: %v", err)
		return mcp.NewToolResultError("failed to call citygml API"), nil
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return mcp.NewToolResultError(fmt.Sprintf("citygml API returned status %d", resp.StatusCode)), nil
	}

	// Parse response
	var result GetCityGMLFilesResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		log.Errorfc(ctx, "citygml: failed to decode response: %v", err)
		return mcp.NewToolResultError("failed to decode citygml API response"), nil
	}

	// Ensure Cities is not nil
	if result.Cities == nil {
		result.Cities = []CityGMLFilesCity{}
	}

	// Add hint when no cities found
	if len(result.Cities) == 0 {
		result.Reason = "NO_DATA_IN_AREA"
		result.Hint = &GetCityGMLFilesHint{
			Message: "指定した条件に該当するCityGMLファイルが見つかりませんでした。PLATEAUデータは整備済み都市のみ利用可能です。plateau_search_areasで対象都市を確認するか、メッシュコード(m:)での直接指定を試してください。詳細はplateau_explain_spatial_idツールを参照してください。",
		}
	}

	return convertToToolResult(result)
}

// Shared handler functions that can be used by both Service and ToolRegistrar

// handleCityGMLGetAttributesWithHost handles plateau_citygml_get_attributes with a specific host
func handleCityGMLGetAttributesWithHost(ctx context.Context, request mcp.CallToolRequest, citygmlAPIBase string) (*mcp.CallToolResult, error) {
	citygmlURL, err := request.RequireString("url")
	if err != nil {
		return mcp.NewToolResultError("url parameter is required"), nil
	}

	u, err := url.Parse(citygmlURL)
	if err != nil {
		return mcp.NewToolResultError("invalid url format"), nil
	}
	if u.Scheme != "http" && u.Scheme != "https" {
		return mcp.NewToolResultError("url must be http or https"), nil
	}

	buildingIDs := request.GetStringSlice("building_ids", nil)
	if len(buildingIDs) == 0 {
		return mcp.NewToolResultError("building_ids parameter is required and must not be empty"), nil
	}

	skipCodeList := request.GetBool("skip_code_list", false)

	apiURL := fmt.Sprintf("%s/attributes?url=%s&id=%s",
		citygmlAPIBase,
		url.QueryEscape(citygmlURL),
		url.QueryEscape(strings.Join(buildingIDs, ",")),
	)
	if skipCodeList {
		apiURL += "&skip_code_list_fetch=1"
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, apiURL, nil)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to create request: %v", err)
		return mcp.NewToolResultError("failed to create request"), nil
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to call API: %v", err)
		return mcp.NewToolResultError("failed to call citygml API"), nil
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return mcp.NewToolResultError(fmt.Sprintf("citygml API returned status %d", resp.StatusCode)), nil
	}

	var attrs []map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&attrs); err != nil {
		log.Errorfc(ctx, "citygml: failed to decode response: %v", err)
		return mcp.NewToolResultError("failed to decode citygml API response"), nil
	}

	response := &GetCityGMLAttributesResponse{
		Attributes: attrs,
	}

	return convertToToolResult(response)
}

// handleCityGMLGetFeaturesWithHost handles plateau_citygml_get_features with a specific host
func handleCityGMLGetFeaturesWithHost(ctx context.Context, request mcp.CallToolRequest, citygmlAPIBase string) (*mcp.CallToolResult, error) {
	citygmlURL, err := request.RequireString("url")
	if err != nil {
		return mcp.NewToolResultError("url parameter is required"), nil
	}

	u, err := url.Parse(citygmlURL)
	if err != nil {
		return mcp.NewToolResultError("invalid url format"), nil
	}
	if u.Scheme != "http" && u.Scheme != "https" {
		return mcp.NewToolResultError("url must be http or https"), nil
	}

	spatialIDs := request.GetStringSlice("spatial_ids", nil)
	if len(spatialIDs) == 0 {
		return mcp.NewToolResultError("spatial_ids parameter is required and must not be empty"), nil
	}

	apiURL := fmt.Sprintf("%s/features?url=%s&sid=%s",
		citygmlAPIBase,
		url.QueryEscape(citygmlURL),
		url.QueryEscape(strings.Join(spatialIDs, ",")),
	)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, apiURL, nil)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to create request: %v", err)
		return mcp.NewToolResultError("failed to create request"), nil
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to call API: %v", err)
		return mcp.NewToolResultError("failed to call citygml API"), nil
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return mcp.NewToolResultError(fmt.Sprintf("citygml API returned status %d", resp.StatusCode)), nil
	}

	var result struct {
		FeatureIDs []string `json:"featureIds"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		log.Errorfc(ctx, "citygml: failed to decode response: %v", err)
		return mcp.NewToolResultError("failed to decode citygml API response"), nil
	}

	response := &GetCityGMLFeaturesResponse{
		FeatureIDs: result.FeatureIDs,
	}
	if response.FeatureIDs == nil {
		response.FeatureIDs = []string{}
	}

	return convertToToolResult(response)
}

// handleCityGMLGetGeoidHeightWithHost handles plateau_citygml_get_geoid_height with a specific host
func handleCityGMLGetGeoidHeightWithHost(ctx context.Context, request mcp.CallToolRequest, citygmlAPIBase string) (*mcp.CallToolResult, error) {
	lat, err := request.RequireFloat("latitude")
	if err != nil {
		return mcp.NewToolResultError("latitude parameter is required"), nil
	}

	lng, err := request.RequireFloat("longitude")
	if err != nil {
		return mcp.NewToolResultError("longitude parameter is required"), nil
	}

	if lat < -90 || lat > 90 {
		return mcp.NewToolResultError("latitude must be between -90 and 90"), nil
	}
	if lng < -180 || lng > 180 {
		return mcp.NewToolResultError("longitude must be between -180 and 180"), nil
	}

	apiURL := fmt.Sprintf("%s/geoid_height?lat=%s&lng=%s",
		citygmlAPIBase,
		strconv.FormatFloat(lat, 'f', -1, 64),
		strconv.FormatFloat(lng, 'f', -1, 64),
	)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, apiURL, nil)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to create request: %v", err)
		return mcp.NewToolResultError("failed to create request"), nil
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to call API: %v", err)
		return mcp.NewToolResultError("failed to call citygml API"), nil
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return mcp.NewToolResultError(fmt.Sprintf("citygml API returned status %d", resp.StatusCode)), nil
	}

	var result struct {
		Lat         float64 `json:"lat"`
		Lng         float64 `json:"lng"`
		GeoidHeight float64 `json:"geoid_height"`
		Geoid       string  `json:"geoid"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		log.Errorfc(ctx, "citygml: failed to decode response: %v", err)
		return mcp.NewToolResultError("failed to decode citygml API response"), nil
	}

	response := &GetGeoidHeightResponse{
		Latitude:    result.Lat,
		Longitude:   result.Lng,
		GeoidHeight: result.GeoidHeight,
		Geoid:       result.Geoid,
	}

	return convertToToolResult(response)
}

// handleGetCityGMLFilesWithHost handles plateau_get_citygml_files with a specific host
func handleGetCityGMLFilesWithHost(ctx context.Context, request mcp.CallToolRequest, host string) (*mcp.CallToolResult, error) {
	condition, err := request.RequireString("condition")
	if err != nil {
		return mcp.NewToolResultError("condition parameter is required"), nil
	}

	if condition == "" {
		return mcp.NewToolResultError("condition must not be empty"), nil
	}
	if !strings.HasPrefix(condition, "m:") && !strings.HasPrefix(condition, "s:") && !strings.HasPrefix(condition, "r:") {
		return mcp.NewToolResultError("condition must start with m:, s:, or r:"), nil
	}

	featureTypes := request.GetStringSlice("feature_types", nil)

	if host == "" {
		host = "http://localhost:8080"
	}
	apiURL := fmt.Sprintf("%s/datacatalog/citygml/%s",
		host,
		url.PathEscape(condition),
	)

	if len(featureTypes) > 0 {
		apiURL += "?types=" + url.QueryEscape(strings.Join(featureTypes, ","))
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, apiURL, nil)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to create request: %v", err)
		return mcp.NewToolResultError("failed to create request"), nil
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		log.Errorfc(ctx, "citygml: failed to call API: %v", err)
		return mcp.NewToolResultError("failed to call citygml API"), nil
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return mcp.NewToolResultError(fmt.Sprintf("citygml API returned status %d", resp.StatusCode)), nil
	}

	var result GetCityGMLFilesResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		log.Errorfc(ctx, "citygml: failed to decode response: %v", err)
		return mcp.NewToolResultError("failed to decode citygml API response"), nil
	}

	if result.Cities == nil {
		result.Cities = []CityGMLFilesCity{}
	}

	return convertToToolResult(result)
}
