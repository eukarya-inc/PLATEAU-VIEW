package cmsintflow

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFlowResult_Internal(t *testing.T) {
	id := ID{
		ItemID:      "item",
		ProjectID:   "project",
		FeatureType: "bldg",
	}.Sign("secret")

	r := FlowResult{
		TriggerID:    "trigger",
		RunID:        "run",
		DeploymentID: "deployment",
		Status:       "succeeded",
		Logs:         []string{"https://example.com/logs.log"},
		Outputs: []string{
			"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_dic.json",
			"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_maxLod.csv",
			"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_qc_result.zip",
			"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_qc_result_ok",
			"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_3dtiles_lod1.zip",
			"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_3dtiles_lod2.zip",
			"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_3dtiles_lod2_no_texture.zip",
		},
		ID: id,
	}

	expected := FlowInternalResult{
		Conv: map[string][]string{
			"bldg_3dtiles": {
				"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_3dtiles_lod1.zip",
				"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_3dtiles_lod2.zip",
				"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_3dtiles_lod2_no_texture.zip",
			},
		},
		Dic:      "https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_dic.json",
		QCResult: "https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_qc_result.zip",
		QCOK:     true,
	}

	res := r.Internal()
	assert.Equal(t, expected, res)
}

func TestFlowResult_Internal_DictionaryJson(t *testing.T) {
	// Test that dictionary.json (without prefix) is also recognized as dictionary file
	r := FlowResult{
		Status: "succeeded",
		Outputs: []string{
			"https://example.com/artifacts/dictionary.json",
			"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_3dtiles_lod1.zip",
		},
	}

	expected := FlowInternalResult{
		Conv: map[string][]string{
			"bldg_3dtiles": {
				"https://example.com/13999_hoge-shi_city_2023_citygml_1_op_bldg_3dtiles_lod1.zip",
			},
		},
		Dic: "https://example.com/artifacts/dictionary.json",
	}

	res := r.Internal()
	assert.Equal(t, expected, res)
}

func TestFlowResult_Internal_Urf(t *testing.T) {
	// Test urf feature type with UseGroups=true pattern
	r := FlowResult{
		Status: "succeeded",
		Outputs: []string{
			"https://example.com/artifacts/dictionary.json",
			"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_UseDistrict_mvt_lod1.zip",
			"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_FirePreventionDistrict_mvt_lod1.zip",
			"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_UrbanPlanningArea_mvt_lod1.zip",
		},
	}

	expected := FlowInternalResult{
		Conv: map[string][]string{
			"urf_UseDistrict_mvt": {
				"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_UseDistrict_mvt_lod1.zip",
			},
			"urf_FirePreventionDistrict_mvt": {
				"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_FirePreventionDistrict_mvt_lod1.zip",
			},
			"urf_UrbanPlanningArea_mvt": {
				"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_UrbanPlanningArea_mvt_lod1.zip",
			},
		},
		Dic: "https://example.com/artifacts/dictionary.json",
	}

	res := r.Internal()
	assert.Equal(t, expected, res)
}

func TestFlowResult_Internal_Veg(t *testing.T) {
	// Test veg feature type with UseGroups=true pattern
	r := FlowResult{
		Status: "succeeded",
		Outputs: []string{
			"https://example.com/12345_city-name_city_2025_citygml_1_op_veg_PlantCover_mvt_lod1.zip",
			"https://example.com/12345_city-name_city_2025_citygml_1_op_veg_SolitaryVegetationObject_mvt_lod1.zip",
		},
	}

	expected := FlowInternalResult{
		Conv: map[string][]string{
			"veg_PlantCover_mvt": {
				"https://example.com/12345_city-name_city_2025_citygml_1_op_veg_PlantCover_mvt_lod1.zip",
			},
			"veg_SolitaryVegetationObject_mvt": {
				"https://example.com/12345_city-name_city_2025_citygml_1_op_veg_SolitaryVegetationObject_mvt_lod1.zip",
			},
		},
	}

	res := r.Internal()
	assert.Equal(t, expected, res)
}

func TestFlowResult_Internal_Veg_3dtiles(t *testing.T) {
	// Test veg feature type with 3dtiles pattern (actual production pattern)
	// Note: dm_geometric_attributes is treated as mvt format by datacatalogv3
	r := FlowResult{
		Status: "succeeded",
		Outputs: []string{
			"https://example.com/13999_tokyo_udx-mlit_2023_citygml_2_sample-takeshiba_op_veg_PlantCover_3dtiles_lod1.zip",
			"https://example.com/13999_tokyo_udx-mlit_2023_citygml_2_sample-takeshiba_op_veg_PlantCover_3dtiles_lod2.zip",
			"https://example.com/13999_tokyo_udx-mlit_2023_citygml_2_sample-takeshiba_op_veg_PlantCover_dm_geometric_attributes.zip",
			"https://example.com/13999_tokyo_udx-mlit_2023_citygml_2_sample-takeshiba_op_veg_SolitaryVegetationObject_3dtiles_lod1.zip",
			"https://example.com/artifacts/dictionary.json",
		},
	}

	expected := FlowInternalResult{
		Conv: map[string][]string{
			"veg_PlantCover_3dtiles": {
				"https://example.com/13999_tokyo_udx-mlit_2023_citygml_2_sample-takeshiba_op_veg_PlantCover_3dtiles_lod1.zip",
				"https://example.com/13999_tokyo_udx-mlit_2023_citygml_2_sample-takeshiba_op_veg_PlantCover_3dtiles_lod2.zip",
			},
			"veg_PlantCover_mvt": {
				"https://example.com/13999_tokyo_udx-mlit_2023_citygml_2_sample-takeshiba_op_veg_PlantCover_dm_geometric_attributes.zip",
			},
			"veg_SolitaryVegetationObject_3dtiles": {
				"https://example.com/13999_tokyo_udx-mlit_2023_citygml_2_sample-takeshiba_op_veg_SolitaryVegetationObject_3dtiles_lod1.zip",
			},
		},
		Dic: "https://example.com/artifacts/dictionary.json",
	}

	res := r.Internal()
	assert.Equal(t, expected, res)
}

func TestFlowResult_Internal_Fld(t *testing.T) {
	// Test fld (flood) feature type with UseGroups=true pattern
	r := FlowResult{
		Status: "succeeded",
		Outputs: []string{
			"https://example.com/13999_city-name_city_2023_citygml_1_op_fld_natl_tone-river_3dtiles_l1.zip",
			"https://example.com/13999_city-name_city_2023_citygml_1_op_fld_natl_tone-river_3dtiles_l2.zip",
			"https://example.com/13999_city-name_city_2023_citygml_1_op_fld_pref_ara-river_3dtiles_l1.zip",
			"https://example.com/artifacts/dictionary.json",
		},
	}

	expected := FlowInternalResult{
		Conv: map[string][]string{
			"fld_natl_tone-river_3dtiles": {
				"https://example.com/13999_city-name_city_2023_citygml_1_op_fld_natl_tone-river_3dtiles_l1.zip",
				"https://example.com/13999_city-name_city_2023_citygml_1_op_fld_natl_tone-river_3dtiles_l2.zip",
			},
			"fld_pref_ara-river_3dtiles": {
				"https://example.com/13999_city-name_city_2023_citygml_1_op_fld_pref_ara-river_3dtiles_l1.zip",
			},
		},
		Dic: "https://example.com/artifacts/dictionary.json",
	}

	res := r.Internal()
	assert.Equal(t, expected, res)
}

