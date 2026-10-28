package datacatalogv3

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/cmsintegration/ckan"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestToCityGMLs(t *testing.T) {
	regYear := 2023

	all := &AllData{
		City: []*CityItem{
			{
				ID:         "city1",
				Prefecture: "東京都",
				CityName:   "東京都23区",
				CityNameEn: "tokyo23ku",
				CityCode:   "13100",
				Spec:       "第3.3版",
				Year:       "2023",
				SDKPublic:  true,
				CodeLists: &cms.PublicAsset{
					Asset: cms.Asset{
						URL: "https://example.com/codelists.zip",
					},
				},
			},
		},
		GeospatialjpDataItems: []*GeospatialjpDataItem{
			{
				City:        "city1",
				CityGML:     "https://example.com/city1.gml",
				CityGMLSize: 100,
				MaxLOD:      "https://example.com/maxlod1.csv",
				Plateau:     "https://example.com/13100_3dtiles_mvt.zip",
				PlateauSize: 200,
				Related:     "https://example.com/13100_related.zip",
			},
		},
		Gspatialjp: NewGspatialjpPackages("https://ckan.example.com/", &GspatialjpPackage{
			Name: "plateau-tokyo23ku-2023",
			Resources: []ckan.Resource{
				{Name: "データ目録（v4）", URL: "data:text/markdown;base64,AAAA"},
				{Name: "CityGML（v4）", URL: "https://example.com/city1_v4.zip"},
				{Name: "データ目録（v3）", URL: "data:text/markdown;base64,AAAA"},
				{Name: "索引図（v3）", URL: "https://example.com/13100_indexmap_op.pdf", Format: "PDF"},
				// G空間情報センター still has an older zip than the catalog
				{Name: "CityGML（v3）", URL: "https://example.com/city1_old.zip"},
				{Name: "3D Tiles, MVT（v3）", URL: "https://example.com/13100_3dtiles_mvt.zip"},
				{Name: "関連データセット（v3）", URL: "https://example.com/13100_related.zip"},
				{Name: "その他", URL: "https://example.com/other.zip"},
			},
		}),
		Plateau: map[string][]*PlateauFeatureItem{
			"bldg": {
				{
					City:    "city1",
					CityGML: "https://example.com/city3.gml",
				},
			},
			"ubld": {
				{
					City:    "city1",
					Sample:  true,
					CityGML: "https://example.com/city2.gml",
					MaxLOD:  "https://example.com/maxlod2.csv",
				},
			},
		},
		FeatureTypes: FeatureTypes{
			Plateau: []FeatureType{
				{
					Code: "bldg",
				},
				{
					Code: "ubld",
				},
			},
		},
	}

	gspatialjpURL := "https://ckan.example.com/dataset/plateau-tokyo23ku-2023"
	expected := map[plateauapi.ID]*plateauapi.CityGMLDataset{
		"cg_13100": {
			ID:                   "cg_13100",
			Year:                 2023,
			RegistrationYear:     regYear,
			URL:                  "https://example.com/city1.gml",
			FileSize:             lo.ToPtr(100),
			GspatialjpDatasetURL: &gspatialjpURL,
			PrefectureID:         "p_13",
			PrefectureCode:       "13",
			CityID:               "c_13100",
			CityCode:             "13100",
			PlateauSpecMinorID:   "ps_3.3",
			FeatureTypes:         []string{"bldg", "ubld"},
			MetadataZipUrls:      []string{"https://example.com/codelists.zip"},
			GspatialjpFiles: []*plateauapi.GspatialjpFile{
				{Kind: "citygml", Name: "CityGML（v3）", URL: "https://example.com/city1_old.zip"},
				{Kind: "3dtiles_mvt", Name: "3D Tiles, MVT（v3）", URL: "https://example.com/13100_3dtiles_mvt.zip", FileSize: 200},
				{Kind: "related", Name: "関連データセット（v3）", URL: "https://example.com/13100_related.zip"},
				{Kind: "other", Name: "その他", URL: "https://example.com/other.zip"},
			},
			Admin: &plateauapi.Admin{
				CMSItemID: "city1",
				CityGMLURLs: []string{
					"https://example.com/city1.gml",
					"https://example.com/city2.gml",
				},
				MaxLODURLs: []string{
					"https://example.com/maxlod1.csv",
					"https://example.com/maxlod2.csv",
				},
			},
		},
	}

	res, warnings := toCityGMLs(all, regYear)
	assert.Nil(t, warnings)
	assert.Equal(t, expected, res)
}

func TestToCityGMLs_NoGspatialjpPackage(t *testing.T) {
	newAll := func(g *GspatialjpPackages) *AllData {
		return &AllData{
			City: []*CityItem{{ID: "city1", CityCode: "13120", CityName: "nerima-ku", CityNameEn: "nerima-ku", Spec: "第5.0版", Year: "2025"}},
			GeospatialjpDataItems: []*GeospatialjpDataItem{
				{City: "city1", CityGML: "https://example.com/city1.zip", MaxLOD: "https://example.com/maxlod.csv"},
			},
			Gspatialjp: g,
		}
	}

	// G空間情報センター has no dataset of the city and year
	res, _ := toCityGMLs(newAll(NewGspatialjpPackages("https://ckan.example.com",
		&GspatialjpPackage{Name: "plateau-13120-nerima-ku-2024", Resources: []ckan.Resource{{Name: "CityGML（v5）", URL: "https://example.com/x.zip"}}},
	)), 2025)
	assert.Nil(t, res["cg_13120"].GspatialjpDatasetURL)
	assert.Nil(t, res["cg_13120"].GspatialjpFiles)

	// G空間情報センター could not be fetched: fall back to the conventional dataset URL
	res, _ = toCityGMLs(newAll(nil), 2025)
	assert.Equal(t, lo.ToPtr("https://www.geospatial.jp/ckan/dataset/plateau-13120-nerima-ku-2025"), res["cg_13120"].GspatialjpDatasetURL)
	assert.Nil(t, res["cg_13120"].GspatialjpFiles)
}
