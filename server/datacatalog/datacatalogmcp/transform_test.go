package datacatalogmcp

import (
	"context"
	"fmt"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestTransformMetadata(t *testing.T) {
	ctx := context.Background()

	repo := plateauapi.NewInMemoryRepo(&plateauapi.InMemoryRepoContext{
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
			plateauapi.AreaTypePrefecture: []plateauapi.Area{
				&plateauapi.Prefecture{ID: "pref_13", Code: "13", Name: "東京都"},
			},
			plateauapi.AreaTypeCity: []plateauapi.Area{
				&plateauapi.City{ID: "city_13101", Code: "13101", Name: "千代田区"},
			},
		},
		Datasets: plateauapi.Datasets{
			plateauapi.DatasetTypeCategoryPlateau: []plateauapi.Dataset{
				&plateauapi.PlateauDataset{ID: "ds_1", Name: "データセット1"},
				&plateauapi.PlateauDataset{ID: "ds_2", Name: "データセット2"},
			},
		},
		Years: []int{2020, 2021, 2022, 2023},
	})

	resp, err := TransformMetadata(ctx, repo)

	assert.NoError(t, err)
	assert.NotNil(t, resp)
	assert.Equal(t, []int{2020, 2021, 2022, 2023}, resp.AvailableYears)
	assert.Equal(t, 1, len(resp.PlateauSpecs))
	assert.Equal(t, 3, resp.PlateauSpecs[0].MajorVersion)
	assert.Equal(t, 2023, resp.PlateauSpecs[0].Year)
	// In InMemory repo, total areas/datasets come from iterating through all areas/datasets
	assert.GreaterOrEqual(t, resp.TotalAreas, 1)
	assert.GreaterOrEqual(t, resp.TotalDatasets, 1)
}

func TestTransformSearchAreas_Normal(t *testing.T) {
	areas := []plateauapi.Area{
		&plateauapi.Prefecture{
			ID:   "pref_13",
			Type: plateauapi.AreaTypePrefecture,
			Code: "13",
			Name: "東京都",
		},
		&plateauapi.City{
			ID:             "city_13101",
			Type:           plateauapi.AreaTypeCity,
			Code:           "13101",
			Name:           "千代田区",
			PrefectureCode: "13",
			ParentID:       lo.ToPtr(plateauapi.ID("pref_13")),
		},
	}

	resp := TransformSearchAreas(areas, nil)

	assert.NotNil(t, resp)
	assert.Equal(t, 2, len(resp.Areas))
	assert.Equal(t, "pref_13", resp.Areas[0].ID)
	assert.Equal(t, "13", resp.Areas[0].Code)
	assert.Equal(t, "東京都", resp.Areas[0].Name)
	assert.Equal(t, "PREFECTURE", resp.Areas[0].Type)
	assert.Nil(t, resp.Areas[0].ParentID)

	assert.Equal(t, "city_13101", resp.Areas[1].ID)
	assert.Equal(t, "13101", resp.Areas[1].Code)
	assert.Equal(t, "千代田区", resp.Areas[1].Name)
	assert.Equal(t, "CITY", resp.Areas[1].Type)
	assert.NotNil(t, resp.Areas[1].ParentID)
	assert.Equal(t, "pref_13", *resp.Areas[1].ParentID)

	assert.Equal(t, 2, resp.Metadata.TotalCount)
	assert.Equal(t, 2, resp.Metadata.ReturnedCount)
	assert.False(t, resp.Metadata.HasMore)
	assert.Empty(t, resp.Metadata.RefinementSuggestions)
}

func TestTransformSearchAreas_Over100Items(t *testing.T) {
	// 150個の地域を作成
	areas := make([]plateauapi.Area, 150)
	for i := 0; i < 150; i++ {
		areas[i] = &plateauapi.Prefecture{
			ID:   plateauapi.ID(fmt.Sprintf("pref_%d", i)),
			Code: plateauapi.AreaCode(fmt.Sprintf("%d", i)),
			Name: fmt.Sprintf("都道府県%d", i),
		}
	}

	resp := TransformSearchAreas(areas, nil)

	assert.NotNil(t, resp)
	assert.Equal(t, 100, len(resp.Areas)) // 100件に制限
	assert.Equal(t, 150, resp.Metadata.TotalCount)
	assert.Equal(t, 100, resp.Metadata.ReturnedCount)
	assert.True(t, resp.Metadata.HasMore)
	assert.NotEmpty(t, resp.Metadata.RefinementSuggestions)
}

func TestTransformSearchAreas_Suggestions(t *testing.T) {
	areas := make([]plateauapi.Area, 150)
	for i := 0; i < 150; i++ {
		areas[i] = &plateauapi.Prefecture{
			ID:   plateauapi.ID(fmt.Sprintf("pref_%d", i)),
			Code: plateauapi.AreaCode(fmt.Sprintf("%d", i)),
			Name: fmt.Sprintf("都道府県%d", i),
		}
	}

	input := &SearchAreasInput{}

	resp := TransformSearchAreas(areas, input)

	// 全パラメータが未指定なので、4つのsuggestionが返る
	assert.Equal(t, 4, len(resp.Metadata.RefinementSuggestions))
}

func TestTransformGetArea(t *testing.T) {
	// Prefecture (no parent, no children)
	area := &plateauapi.Prefecture{
		ID:   "pref_13",
		Type: plateauapi.AreaTypePrefecture,
		Code: "13",
		Name: "東京都",
	}

	resp := TransformGetArea(area)

	assert.NotNil(t, resp)
	assert.Equal(t, "pref_13", resp.ID)
	assert.Equal(t, "PREFECTURE", resp.Type)
	assert.Equal(t, "13", resp.Code)
	assert.Equal(t, "東京都", resp.Name)
	assert.Nil(t, resp.Parent)
	assert.Empty(t, resp.Children)
	assert.Nil(t, resp.PlanarCrsEpsgCode) // Prefecture doesn't have PlanarCrsEpsgCode
}

func TestTransformGetArea_City(t *testing.T) {
	parent := &plateauapi.Prefecture{
		ID:   "pref_13",
		Type: plateauapi.AreaTypePrefecture,
		Code: "13",
		Name: "東京都",
	}

	area := &plateauapi.City{
		ID:                "city_13101",
		Type:              plateauapi.AreaTypeCity,
		Code:              "13101",
		Name:              "千代田区",
		PrefectureCode:    "13",
		Parent:            parent,
		PlanarCrsEpsgCode: lo.ToPtr("6677"),
	}

	resp := TransformGetArea(area)

	assert.NotNil(t, resp)
	assert.Equal(t, "city_13101", resp.ID)
	assert.Equal(t, "CITY", resp.Type)
	assert.Equal(t, "13101", resp.Code)
	assert.Equal(t, "千代田区", resp.Name)

	assert.NotNil(t, resp.Parent)
	assert.Equal(t, "pref_13", resp.Parent.ID)
	assert.Equal(t, "13", resp.Parent.Code)
	assert.Equal(t, "東京都", resp.Parent.Name)

	assert.NotNil(t, resp.PlanarCrsEpsgCode)
	assert.Equal(t, "6677", *resp.PlanarCrsEpsgCode)
}

func TestTransformSearchDatasets_Normal(t *testing.T) {
	datasets := []plateauapi.Dataset{
		&plateauapi.PlateauDataset{
			ID:                "ds_bldg_13101",
			Name:              "千代田区_建築物モデル",
			Description:       lo.ToPtr("説明文"),
			TypeID:            "dt_bldg",
			TypeCode:          "bldg",
			Year:              2023,
			RegisterationYear: 2023,
			Type: &plateauapi.PlateauDatasetType{
				ID:       "dt_bldg",
				Code:     "bldg",
				Name:     "建築物モデル",
				Category: plateauapi.DatasetTypeCategoryPlateau,
				Year:     2023,
			},
			Prefecture: &plateauapi.Prefecture{
				ID:   "pref_13",
				Type: plateauapi.AreaTypePrefecture,
				Code: "13",
				Name: "東京都",
			},
			City: &plateauapi.City{
				ID:   "city_13101",
				Type: plateauapi.AreaTypeCity,
				Code: "13101",
				Name: "千代田区",
			},
			PlateauSpecMinor: &plateauapi.PlateauSpecMinor{
				ID:      "spec_2023_0",
				Name:    "第3.0版",
				Version: "3.0",
			},
			Items: []*plateauapi.PlateauDatasetItem{
				{
					ID:     "item_1",
					Name:   "LOD2",
					Format: plateauapi.DatasetFormatCesium3dtiles,
					URL:    "https://example.com/data",
				},
			},
		},
	}

	resp := TransformSearchDatasets(datasets, nil)

	assert.NotNil(t, resp)
	assert.Equal(t, 1, len(resp.Datasets))

	ds := resp.Datasets[0]
	assert.Equal(t, "ds_bldg_13101", ds.ID)
	assert.Equal(t, "千代田区_建築物モデル", ds.Name)
	assert.NotNil(t, ds.Description)
	assert.Equal(t, "説明文", *ds.Description)
	assert.Equal(t, "bldg", ds.Type.Code)
	assert.Equal(t, "建築物モデル", ds.Type.Name)
	assert.Equal(t, "PLATEAU", ds.Type.Category)
	assert.NotNil(t, ds.Area.Prefecture)
	assert.Equal(t, "東京都", *ds.Area.Prefecture)
	assert.NotNil(t, ds.Area.City)
	assert.Equal(t, "千代田区", *ds.Area.City)
	assert.Equal(t, 2023, ds.Year)
	assert.Equal(t, 2023, ds.RegistrationYear)
	assert.NotNil(t, ds.PlateauSpec)
	assert.Equal(t, "第3.0版", *ds.PlateauSpec)
	assert.Equal(t, 1, ds.ItemCount)

	assert.Equal(t, 1, resp.Metadata.TotalCount)
	assert.Equal(t, 1, resp.Metadata.ReturnedCount)
	assert.False(t, resp.Metadata.HasMore)
}

func TestTransformSearchDatasets_Over100Items(t *testing.T) {
	datasets := make([]plateauapi.Dataset, 120)
	for i := 0; i < 120; i++ {
		datasets[i] = &plateauapi.PlateauDataset{
			ID:       plateauapi.ID(fmt.Sprintf("ds_%d", i)),
			Name:     fmt.Sprintf("データセット%d", i),
			TypeID:   "dt_bldg",
			TypeCode: "bldg",
			Type: &plateauapi.PlateauDatasetType{
				ID:       "dt_bldg",
				Code:     "bldg",
				Name:     "建築物モデル",
				Category: plateauapi.DatasetTypeCategoryPlateau,
				Year:     2023,
			},
		}
	}

	resp := TransformSearchDatasets(datasets, nil)

	assert.NotNil(t, resp)
	assert.Equal(t, 100, len(resp.Datasets))
	assert.Equal(t, 120, resp.Metadata.TotalCount)
	assert.Equal(t, 100, resp.Metadata.ReturnedCount)
	assert.True(t, resp.Metadata.HasMore)
	assert.NotEmpty(t, resp.Metadata.RefinementSuggestions)
}

func TestTransformGetDataset(t *testing.T) {
	dataset := &plateauapi.PlateauDataset{
		ID:                "ds_bldg_13101",
		Name:              "千代田区_建築物モデル",
		Description:       lo.ToPtr("詳細な説明"),
		TypeID:            "dt_bldg",
		TypeCode:          "bldg",
		Year:              2023,
		RegisterationYear: 2023,
		Groups:            []string{"建築物", "3D都市モデル"},
		OpenDataURL:       lo.ToPtr("https://example.com/opendata"),
		Type: &plateauapi.PlateauDatasetType{
			ID:       "dt_bldg",
			Code:     "bldg",
			Name:     "建築物モデル",
			Category: plateauapi.DatasetTypeCategoryPlateau,
			Year:     2023,
		},
		Prefecture: &plateauapi.Prefecture{
			ID:   "pref_13",
			Type: plateauapi.AreaTypePrefecture,
			Code: "13",
			Name: "東京都",
		},
		City: &plateauapi.City{
			ID:   "city_13101",
			Type: plateauapi.AreaTypeCity,
			Code: "13101",
			Name: "千代田区",
		},
		PlateauSpecMinor: &plateauapi.PlateauSpecMinor{
			ID:      "spec_2023_0",
			Name:    "第3.0版",
			Version: "3.0",
		},
		Items: []*plateauapi.PlateauDatasetItem{
			{
				ID:      "item_1",
				Name:    "LOD2（テクスチャなし）",
				Format:  plateauapi.DatasetFormatCesium3dtiles,
				URL:     "https://example.com/data/lod2",
				Lod:     lo.ToPtr(2),
				Texture: lo.ToPtr(plateauapi.TextureNone),
				Layers:  []string{},
			},
			{
				ID:      "item_2",
				Name:    "LOD3（テクスチャあり）",
				Format:  plateauapi.DatasetFormatCesium3dtiles,
				URL:     "https://example.com/data/lod3",
				Lod:     lo.ToPtr(3),
				Texture: lo.ToPtr(plateauapi.TextureTexture),
				Layers:  []string{},
			},
		},
	}

	resp := TransformGetDataset(dataset)

	assert.NotNil(t, resp)
	assert.Equal(t, "ds_bldg_13101", resp.ID)
	assert.Equal(t, "千代田区_建築物モデル", resp.Name)
	assert.NotNil(t, resp.Description)
	assert.Equal(t, "詳細な説明", *resp.Description)
	assert.Equal(t, "bldg", resp.Type.Code)
	assert.Equal(t, "建築物モデル", resp.Type.Name)
	assert.Equal(t, "PLATEAU", resp.Type.Category)

	assert.NotNil(t, resp.Area.Prefecture)
	assert.Equal(t, "pref_13", resp.Area.Prefecture.ID)
	assert.Equal(t, "東京都", resp.Area.Prefecture.Name)

	assert.NotNil(t, resp.Area.City)
	assert.Equal(t, "city_13101", resp.Area.City.ID)
	assert.Equal(t, "千代田区", resp.Area.City.Name)

	assert.Nil(t, resp.Area.Ward)

	assert.Equal(t, 2023, resp.Year)
	assert.Equal(t, 2023, resp.RegistrationYear)

	assert.NotNil(t, resp.PlateauSpec)
	assert.Equal(t, "第3.0版", resp.PlateauSpec.Name)
	assert.Equal(t, "3.0", resp.PlateauSpec.Version)

	assert.Equal(t, []string{"建築物", "3D都市モデル"}, resp.Groups)

	assert.NotNil(t, resp.OpenDataURL)
	assert.Equal(t, "https://example.com/opendata", *resp.OpenDataURL)

	assert.Equal(t, 2, len(resp.Items))

	item1 := resp.Items[0]
	assert.Equal(t, "item_1", item1.ID)
	assert.Equal(t, "LOD2（テクスチャなし）", item1.Name)
	assert.Equal(t, "CESIUM3DTILES", item1.Format)
	assert.Equal(t, "https://example.com/data/lod2", item1.URL)
	assert.NotNil(t, item1.Lod)
	assert.Equal(t, 2, *item1.Lod)
	assert.NotNil(t, item1.Texture)
	assert.Equal(t, "NONE", *item1.Texture)

	item2 := resp.Items[1]
	assert.Equal(t, "item_2", item2.ID)
	assert.NotNil(t, item2.Lod)
	assert.Equal(t, 3, *item2.Lod)
	assert.NotNil(t, item2.Texture)
	assert.Equal(t, "TEXTURE", *item2.Texture)
}

func TestTransformListDatasetTypes(t *testing.T) {
	types := []plateauapi.DatasetType{
		&plateauapi.PlateauDatasetType{
			ID:       "dt_bldg",
			Code:     "bldg",
			Name:     "建築物モデル",
			Category: plateauapi.DatasetTypeCategoryPlateau,
			Year:     2023,
			Datasets: []*plateauapi.PlateauDataset{
				{ID: "ds_1"},
				{ID: "ds_2"},
			},
		},
		&plateauapi.RelatedDatasetType{
			ID:       "dt_related",
			Code:     "related1",
			Name:     "関連データ",
			Category: plateauapi.DatasetTypeCategoryRelated,
			Datasets: []*plateauapi.RelatedDataset{
				{ID: "ds_3"},
			},
		},
	}

	resp := TransformListDatasetTypes(types)

	assert.NotNil(t, resp)
	assert.Equal(t, 2, len(resp.DatasetTypes))

	dt1 := resp.DatasetTypes[0]
	assert.Equal(t, "dt_bldg", dt1.ID)
	assert.Equal(t, "bldg", dt1.Code)
	assert.Equal(t, "建築物モデル", dt1.Name)
	assert.Equal(t, "PLATEAU", dt1.Category)
	assert.NotNil(t, dt1.Year)
	assert.Equal(t, 2023, *dt1.Year)
	assert.Equal(t, 2, dt1.DatasetCount)

	dt2 := resp.DatasetTypes[1]
	assert.Equal(t, "dt_related", dt2.ID)
	assert.Equal(t, "related1", dt2.Code)
	assert.Equal(t, "関連データ", dt2.Name)
	assert.Equal(t, "RELATED", dt2.Category)
	assert.Nil(t, dt2.Year) // RelatedDatasetTypeにはYearがない
	assert.Equal(t, 1, dt2.DatasetCount)
}

func TestCreateResponseMetadata(t *testing.T) {
	tests := []struct {
		name        string
		totalCount  int
		suggestions []string
		expected    ResponseMetadata
	}{
		{
			name:        "100件以下",
			totalCount:  50,
			suggestions: []string{},
			expected: ResponseMetadata{
				TotalCount:            50,
				ReturnedCount:         50,
				HasMore:               false,
				RefinementSuggestions: []string{},
			},
		},
		{
			name:        "100件丁度",
			totalCount:  100,
			suggestions: []string{},
			expected: ResponseMetadata{
				TotalCount:            100,
				ReturnedCount:         100,
				HasMore:               false,
				RefinementSuggestions: []string{},
			},
		},
		{
			name:        "100件超過",
			totalCount:  150,
			suggestions: []string{"絞り込んでください"},
			expected: ResponseMetadata{
				TotalCount:            150,
				ReturnedCount:         100,
				HasMore:               true,
				RefinementSuggestions: []string{"絞り込んでください"},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := CreateResponseMetadata(tt.totalCount, tt.suggestions)
			assert.Equal(t, tt.expected, result)
		})
	}
}
