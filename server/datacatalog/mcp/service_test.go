package datacatalogmcp

import (
	"context"
	"encoding/json"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

// mockReposHandler implements ReposHandler interface for testing
type mockReposHandler struct {
	repo plateauapi.Repo
}

func (m *mockReposHandler) PrepareAndGetMergedRepo(ctx context.Context, project string, metadata plateaucms.MetadataList) plateauapi.Repo {
	return m.repo
}

// createTestRepo creates a test repository with sample data
func createTestRepo() plateauapi.Repo {
	// Create prefecture
	pref := &plateauapi.Prefecture{
		ID:   "p_13",
		Type: plateauapi.AreaTypePrefecture,
		Code: "13",
		Name: "東京都",
	}

	// Create city with prefecture relationship
	prefID := plateauapi.ID("p_13")
	city := &plateauapi.City{
		ID:                "c_13101",
		Type:              plateauapi.AreaTypeCity,
		Code:              "13101",
		Name:              "千代田区",
		PrefectureID:      prefID,
		PrefectureCode:    "13",
		ParentID:          &prefID,
		PlanarCrsEpsgCode: lo.ToPtr("6677"),
	}

	// Create dataset with area relationships
	cityID := plateauapi.ID("c_13101")
	dataset := &plateauapi.PlateauDataset{
		ID:                "d_bldg_13101",
		Name:              "千代田区_建築物モデル",
		TypeID:            "dt_bldg",
		TypeCode:          "bldg",
		Year:              2023,
		RegisterationYear: 2023,
		PrefectureID:      &prefID,
		PrefectureCode:    lo.ToPtr(plateauapi.AreaCode("13")),
		Prefecture:        pref, // Set actual prefecture object
		CityID:            &cityID,
		CityCode:          lo.ToPtr(plateauapi.AreaCode("13101")),
		City:              city, // Set actual city object
		Type: &plateauapi.PlateauDatasetType{
			ID:       "dt_bldg",
			Code:     "bldg",
			Name:     "建築物モデル",
			Category: plateauapi.DatasetTypeCategoryPlateau,
			Year:     2023,
		},
		Items: []*plateauapi.PlateauDatasetItem{
			{
				ID:      "item_1",
				Name:    "LOD2（テクスチャなし）",
				Format:  plateauapi.DatasetFormatCesium3dtiles,
				URL:     "https://example.com/data.zip",
				Lod:     lo.ToPtr(2),
				Texture: lo.ToPtr(plateauapi.TextureNone),
				Layers:  []string{},
			},
		},
	}

	return plateauapi.NewInMemoryRepo(&plateauapi.InMemoryRepoContext{
		Name: "test",
		PlateauSpecs: []plateauapi.PlateauSpec{
			{
				ID:           "spec_2023",
				MajorVersion: 3,
				Year:         2023,
				MinorVersions: []*plateauapi.PlateauSpecMinor{
					{
						ID:      "spec_2023_0",
						Name:    "第3.0版",
						Version: "3.0",
					},
				},
			},
		},
		Areas: plateauapi.Areas{
			plateauapi.AreaTypePrefecture: []plateauapi.Area{pref},
			plateauapi.AreaTypeCity:       []plateauapi.Area{city},
		},
		Datasets: plateauapi.Datasets{
			plateauapi.DatasetTypeCategoryPlateau: []plateauapi.Dataset{dataset},
		},
		DatasetTypes: plateauapi.DatasetTypes{
			plateauapi.DatasetTypeCategoryPlateau: []plateauapi.DatasetType{
				&plateauapi.PlateauDatasetType{
					ID:       "dt_bldg",
					Code:     "bldg",
					Name:     "建築物モデル",
					Category: plateauapi.DatasetTypeCategoryPlateau,
					Year:     2023,
				},
			},
		},
		Years: []int{2020, 2021, 2022, 2023},
	})
}

// createTestContext creates a context with CMS metadata
func createTestContext() context.Context {
	metadata := plateaucms.MetadataList{
		{Name: "test_metadata"},
	}
	return plateaucms.SetAllCMSMetadataFromContext(context.Background(), metadata)
}

func TestService_HandleGetMetadata(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name:      "plateau_get_metadata",
			Arguments: map[string]interface{}{},
		},
	}

	result, err := service.handleGetMetadata(ctx, request)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Len(t, result.Content, 1)

	textContent, ok := result.Content[0].(mcp.TextContent)
	assert.True(t, ok)
	assert.Equal(t, "text", textContent.Type)

	// Parse JSON response
	var resp GetMetadataResponse
	err = json.Unmarshal([]byte(textContent.Text), &resp)
	assert.NoError(t, err)
	assert.NotEmpty(t, resp.AvailableYears)
	assert.NotEmpty(t, resp.PlateauSpecs)
}

func TestService_HandleSearchAreas(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name: "plateau_search_areas",
			Arguments: map[string]interface{}{
				"area_types": []interface{}{"PREFECTURE"},
			},
		},
	}

	result, err := service.handleSearchAreas(ctx, request)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Len(t, result.Content, 1)

	textContent, ok := result.Content[0].(mcp.TextContent)
	assert.True(t, ok)

	// Parse JSON response
	var resp SearchAreasResponse
	err = json.Unmarshal([]byte(textContent.Text), &resp)
	assert.NoError(t, err)
	assert.NotEmpty(t, resp.Areas)
	assert.NotNil(t, resp.Metadata)
}

func TestService_HandleGetArea(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name: "plateau_get_area",
			Arguments: map[string]interface{}{
				"code": "13",
			},
		},
	}

	result, err := service.handleGetArea(ctx, request)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Len(t, result.Content, 1)

	textContent, ok := result.Content[0].(mcp.TextContent)
	assert.True(t, ok)

	// Parse JSON response
	var resp GetAreaResponse
	err = json.Unmarshal([]byte(textContent.Text), &resp)
	assert.NoError(t, err)
	assert.Equal(t, "13", resp.Code)
	assert.Equal(t, "東京都", resp.Name)
}

func TestService_HandleGetArea_NotFound(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name: "plateau_get_area",
			Arguments: map[string]interface{}{
				"code": "99", // Non-existent code
			},
		},
	}

	result, err := service.handleGetArea(ctx, request)

	assert.NoError(t, err) // Handler returns error as tool result
	assert.NotNil(t, result)
	assert.True(t, result.IsError)
}

func TestService_HandleGetArea_MissingCode(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name:      "plateau_get_area",
			Arguments: map[string]interface{}{
				// Missing code parameter
			},
		},
	}

	result, err := service.handleGetArea(ctx, request)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.True(t, result.IsError)
}

func TestService_HandleSearchDatasets(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name: "plateau_search_datasets",
			Arguments: map[string]interface{}{
				"dataset_types": []interface{}{"bldg"},
			},
		},
	}

	result, err := service.handleSearchDatasets(ctx, request)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Len(t, result.Content, 1)

	textContent, ok := result.Content[0].(mcp.TextContent)
	assert.True(t, ok)

	// Parse JSON response
	var resp SearchDatasetsResponse
	err = json.Unmarshal([]byte(textContent.Text), &resp)
	assert.NoError(t, err)
	assert.NotNil(t, resp.Metadata)
}

func TestService_HandleGetDataset(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name: "plateau_get_dataset",
			Arguments: map[string]interface{}{
				"id": "d_bldg_13101",
			},
		},
	}

	result, err := service.handleGetDataset(ctx, request)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Len(t, result.Content, 1)

	textContent, ok := result.Content[0].(mcp.TextContent)
	assert.True(t, ok)

	// Parse JSON response
	var resp GetDatasetResponse
	err = json.Unmarshal([]byte(textContent.Text), &resp)
	assert.NoError(t, err)
	assert.Equal(t, "d_bldg_13101", resp.ID)
	assert.Equal(t, "千代田区_建築物モデル", resp.Name)
}

func TestService_HandleGetDataset_NotFound(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name: "plateau_get_dataset",
			Arguments: map[string]interface{}{
				"id": "invalid_id",
			},
		},
	}

	result, err := service.handleGetDataset(ctx, request)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.True(t, result.IsError)
}

func TestService_HandleListDatasetTypes(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	ctx := createTestContext()
	request := mcp.CallToolRequest{
		Params: mcp.CallToolParams{
			Name: "plateau_list_dataset_types",
			Arguments: map[string]interface{}{
				"category": "PLATEAU",
			},
		},
	}

	result, err := service.handleListDatasetTypes(ctx, request)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Len(t, result.Content, 1)

	textContent, ok := result.Content[0].(mcp.TextContent)
	assert.True(t, ok)

	// Parse JSON response
	var resp ListDatasetTypesResponse
	err = json.Unmarshal([]byte(textContent.Text), &resp)
	assert.NoError(t, err)
	assert.NotEmpty(t, resp.DatasetTypes)
}

func TestService_GetRepoFromContext_NoMetadata(t *testing.T) {
	repo := createTestRepo()
	mockHandler := &mockReposHandler{repo: repo}
	service := NewService(mockHandler)

	// Context without metadata
	ctx := context.Background()

	_, err := service.getRepoFromContext(ctx)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "metadata not found in context")
}
