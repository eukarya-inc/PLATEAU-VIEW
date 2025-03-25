package datacatalogv3

import (
	"context"
	"encoding/json"
	"net/http"
	"testing"

	"github.com/eukarya-inc/reearth-plateauview/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestRepos(t *testing.T) {
	ctx := context.Background()

	httpmock.Activate()
	defer httpmock.DeactivateAndReset()
	mockCMS(t)

	ctx = plateaucms.SetAllCMSMetadataFromContext(ctx, plateaucms.MetadataList{
		{
			ProjectAlias: "prj",
			CMSURL:       "https://example.com",
			WorkspaceID:  "ws",
			ProjectID:    "prj",
		},
	})

	cms := lo.Must(cms.New("https://example.com", "token"))
	pcms := lo.Must(plateaucms.New(plateaucms.Config{
		CMSBaseURL:       "https://example.com",
		CMSMainToken:     "token",
		CMSSystemProject: "sys",
	}))

	repos := NewRepos(pcms)
	err := repos.Prepare(ctx, "prj", 2023, true, cms)
	assert.NoError(t, err)
	assert.Equal(t, []string{"plateau bldg2 bldg: invalid city: city2", "plateau bldg2: city not found: city2"}, repos.Warnings("prj"))

	assertRes := func(t *testing.T, ctx context.Context, r plateauapi.Repo, cityName, cityCode, itemID string, admin bool, stage *string, isPref, found, noCity bool) {
		t.Helper()

		prefCode := cityCode[:2]
		area, err := r.Area(ctx, plateauapi.AreaCode(cityCode))
		assert.NoError(t, err)
		if !isPref {
			if !noCity {
				assert.Equal(t, &plateauapi.City{
					ID:             plateauapi.ID("c_" + cityCode),
					Type:           plateauapi.AreaTypeCity,
					Code:           plateauapi.AreaCode(cityCode),
					Name:           cityName,
					ParentID:       lo.ToPtr(plateauapi.ID("p_" + prefCode)),
					PrefectureID:   plateauapi.ID("p_" + prefCode),
					PrefectureCode: plateauapi.AreaCode(prefCode),
					CitygmlID:      lo.ToPtr(plateauapi.ID("cg_" + cityCode)),
				}, area)
			} else {
				assert.Nil(t, area)
			}
		} else {
			assert.Equal(t, &plateauapi.Prefecture{
				ID:   plateauapi.ID("p_" + cityCode),
				Type: plateauapi.AreaTypePrefecture,
				Code: plateauapi.AreaCode(cityCode),
				Name: cityName,
			}, area)
		}

		dataset, err := r.Datasets(ctx, &plateauapi.DatasetsInput{
			AreaCodes: []plateauapi.AreaCode{plateauapi.AreaCode(cityCode)},
			Shallow:   lo.ToPtr(true),
		})
		assert.NoError(t, err)

		var adminData any
		if admin {
			a := &plateauapi.Admin{
				CMSItemID: itemID,
				CMSURL:    "https://example.com/workspace/ws/project/prj/content/xxx/details/" + itemID,
			}
			if stage != nil {
				a.Stage = *stage
			}
			adminData = a
		}

		var cityID *plateauapi.ID
		var cityCodeGQL *plateauapi.AreaCode
		if !isPref {
			cityID = lo.ToPtr(plateauapi.ID("c_" + cityCode))
			cityCodeGQL = lo.ToPtr(plateauapi.AreaCode(cityCode))
		}

		if found {
			assert.Equal(t, []plateauapi.Dataset{
				&plateauapi.PlateauDataset{
					ID:                 plateauapi.ID("d_" + cityCode + "_bldg"),
					Name:               "建築物モデル（" + cityName + "）",
					Year:               2023,
					RegisterationYear:  2023,
					PrefectureID:       lo.ToPtr(plateauapi.ID("p_00")),
					PrefectureCode:     lo.ToPtr(plateauapi.AreaCode("00")),
					CityID:             cityID,
					CityCode:           cityCodeGQL,
					TypeID:             plateauapi.NewID("bldg_3", plateauapi.TypeDatasetType),
					TypeCode:           "bldg",
					PlateauSpecMinorID: plateauapi.ID("ps_3.2"),
					Ar:                 true,
					Items: []*plateauapi.PlateauDatasetItem{
						{
							ID:       plateauapi.ID("di_" + cityCode + "_bldg_lod1"),
							Format:   plateauapi.DatasetFormatCesium3dtiles,
							URL:      "https://example.com/00000_hoge_city_2023_citygml_1_op_bldg_3dtiles_lod1/tileset.json",
							Name:     "LOD1",
							ParentID: plateauapi.ID("d_" + cityCode + "_bldg"),
							Lod:      lo.ToPtr(1),
							LodEx:    nil,
							Texture:  lo.ToPtr(plateauapi.TextureTexture),
						},
					},
					Admin: adminData,
				},
			}, dataset)
		} else {
			assert.Len(t, dataset, 0)
		}
	}

	repo := repos.Repo("prj")
	assertRes(t, ctx, repo, "PREF", "00", "bldg1", false, nil, true, false, false)
	assertRes(t, ctx, repo, "foo", "00001", "bldg1", false, nil, false, true, false)
	assertRes(t, ctx, repo, "bar", "00002", "bldg1", false, nil, false, false, true)

	ctx2 := AdminContext(ctx, true, true, false)
	assertRes(t, ctx2, repo, "PREF", "00", "bldg0", true, lo.ToPtr(string(stageBeta)), true, true, false)
	assertRes(t, ctx2, repo, "foo", "00001", "bldg1", true, nil, false, true, false)
	assertRes(t, ctx2, repo, "bar", "00002", "bldg1", true, nil, false, false, true)

	assert.NoError(t, repos.UpdateAll(ctx))
}

func mockCMS(t *testing.T) {
	t.Helper()

	httpmock.RegisterResponder(
		"GET", "https://example.com/api/projects/prj/models",
		httpmock.NewJsonResponderOrPanic(200, models),
	)
	httpmock.RegisterResponder(
		"GET", "https://example.com/api/projects/prj/models/plateau-city/items",
		httpmock.NewJsonResponderOrPanic(200, cities),
	)
	httpmock.RegisterResponder(
		"GET", "https://example.com/api/projects/prj/models/plateau-related/items",
		httpmock.NewJsonResponderOrPanic(200, empty),
	)
	httpmock.RegisterResponder(
		"GET", "https://example.com/api/projects/prj/models/plateau-generic/items",
		httpmock.NewJsonResponderOrPanic(200, empty),
	)
	httpmock.RegisterResponder(
		"GET", "https://example.com/api/projects/prj/models/plateau-sample/items",
		httpmock.NewJsonResponderOrPanic(200, empty),
	)
	httpmock.RegisterResponder(
		"GET", "https://example.com/api/projects/prj/models/plateau-geospatialjp-data/items",
		httpmock.NewJsonResponderOrPanic(200, empty),
	)
	for _, ft := range []string{"bldg"} {
		res := empty
		if ft == "bldg" {
			res = bldg
		}
		httpmock.RegisterResponder(
			"GET", "https://example.com/api/projects/prj/models/plateau-"+ft+"/items",
			httpmock.NewJsonResponderOrPanic(200, res),
		)
	}

	httpmock.RegisterResponder(
		"GET",
		"https://example.com/api/projects/sys/models/plateau-features/items",
		httpmock.NewJsonResponderOrPanic(http.StatusOK, cms.Items{
			PerPage:    1,
			Page:       1,
			TotalCount: 1,
			Items: []cms.Item{
				{
					ID: "bldg",
					Fields: []*cms.Field{
						{Key: "name", Value: "建築物モデル"},
						{Key: "code", Value: "bldg"},
					},
				},
			},
		}),
	)

	httpmock.RegisterResponder(
		"GET",
		"https://example.com/api/projects/sys/models/plateau-dataset-types/items",
		httpmock.NewJsonResponderOrPanic(http.StatusOK, cms.Items{
			PerPage:    1,
			Page:       1,
			TotalCount: 1,
			Items: []cms.Item{
				{
					ID: "landmark",
					Fields: []*cms.Field{
						{Key: "name", Value: "ランドマーク"},
						{Key: "code", Value: "landmark"},
						{Key: "category", Value: "その他のデータセット"},
					},
				},
			},
		}),
	)

	httpmock.RegisterResponder(
		"GET",
		"https://example.com/api/projects/sys/models/plateau-spec/items",
		httpmock.NewJsonResponderOrPanic(http.StatusOK, cms.Items{
			PerPage:    1,
			Page:       1,
			TotalCount: 1,
			Items: []cms.Item{
				{
					ID: "1",
					Fields: []*cms.Field{
						{Key: "major_version", Value: 4},
						{Key: "year", Value: 2024},
						{Key: "max_minor_version", Value: 1},
						{Key: "fme_url", Value: "https://example.com/v4"},
					},
				},
				{
					ID: "2",
					Fields: []*cms.Field{
						{Key: "major_version", Value: 3},
						{Key: "year", Value: 2023},
						{Key: "max_minor_version", Value: 5},
						{Key: "fme_url", Value: "https://example.com/v3"},
					},
				},
			},
		}),
	)
}

func j(j string) any {
	var v any
	lo.Must0(json.Unmarshal([]byte(j), &v))
	return v
}

var models = j(`{
	"totalCount": 1,
	"page": 1,
	"perPage": 100,
	"models": [
		{
			"id": "xxx",
			"key": "plateau-bldg"
		}
	]
}`)

var cities = j(`{
	"totalCount": 1,
	"items": [
		{
			"id": "city0",
			"fields": [
				{
					"key": "prefecture",
					"value": "PREF"
				},
				{
					"key": "city_name",
					"value": "PREF"
				},
				{
					"key": "city_code",
					"value": "00"
				},
				{
					"key": "bldg",
					"value": "bldg0"
				},
				{
					"key": "spec",
					"value": "第3.2版"
				}
			],
			"metadataFields": [
				{
					"key": "plateau_data_status",
					"value": {
						"name": "確認可能"
					}
				}
			]
		},
		{
			"id": "city1",
			"fields": [
				{
					"key": "prefecture",
					"value": "PREF"
				},
				{
					"key": "city_name",
					"value": "foo"
				},
				{
					"key": "city_code",
					"value": "00001"
				},
				{
					"key": "bldg",
					"value": "bldg1"
				},
				{
					"key": "spec",
					"value": "第3.2版"
				}
			],
			"metadataFields": [
				{
					"key": "bldg_public",
					"value": true
				}
			]
		},
		{
			"id": "city2",
			"fields": [
				{
					"key": "prefecture",
					"value": "PREF"
				},
				{
					"key": "city_name",
					"value": "bar"
				},
				{
					"key": "city_code",
					"value": "00002"
				},
				{
					"key": "bldg",
					"value": "bldg2"
				},
				{
					"key": "spec",
					"value": "第3.2版"
				}
			],
			"metadataFields": [
			]
		}
	]
}`)

var bldg = j(`{
	"totalCount": 1,
	"items": [
		{
			"id": "bldg0",
			"fields": [
				{
					"key": "city",
					"value": "city0"
				},
				{
					"key": "data",
					"value": [{"url": "https://example.com/00000_hoge_city_2023_citygml_1_op_bldg_3dtiles_lod1.zip"}]
				}
			]
		},
		{
			"id": "bldg1",
			"fields": [
				{
					"key": "city",
					"value": "city1"
				},
				{
					"key": "data",
					"value": [{"url": "https://example.com/00000_hoge_city_2023_citygml_1_op_bldg_3dtiles_lod1.zip"}]
				}
			]
		},
		{
			"id": "bldg2",
			"fields": [
				{
					"key": "city",
					"value": "city2"
				},
				{
					"key": "data",
					"value": [{"url": "https://example.com/00000_hoge_city_2023_citygml_1_op_bldg_3dtiles_lod1.zip"}]
				}
			]
		}
	]
}`)

var empty = j(`{
	"totalCount": 0,
	"items": []
}`)
