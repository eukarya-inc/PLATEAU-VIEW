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
			"https://example.com/13999_hoge-shi_citygml_op_1_bldg_dic.json",
			"https://example.com/13999_hoge-shi_citygml_op_1_bldg_maxLod.csv",
			"https://example.com/13999_hoge-shi_citygml_op_1_bldg_qc_result.zip",
			"https://example.com/13999_hoge-shi_citygml_op_1_bldg_qc_result_ok",
			"https://example.com/13999_hoge-shi_citygml_op_1_bldg_3dtiles_lod1.zip",
			"https://example.com/13999_hoge-shi_citygml_op_1_bldg_3dtiles_lod2.zip",
			"https://example.com/13999_hoge-shi_citygml_op_1_bldg_3dtiles_lod2_no_texture.zip",
		},
		ID: id,
	}

	expected := FlowInternalResult{
		Conv: map[string][]string{
			"hoge-shi_citygml_op_1_bldg_3dtiles": {
				"https://example.com/13999_hoge-shi_citygml_op_1_bldg_3dtiles_lod1.zip",
				"https://example.com/13999_hoge-shi_citygml_op_1_bldg_3dtiles_lod2.zip",
				"https://example.com/13999_hoge-shi_citygml_op_1_bldg_3dtiles_lod2_no_texture.zip",
			},
		},
		Dic:      "https://example.com/13999_hoge-shi_citygml_op_1_bldg_dic.json",
		QCResult: "https://example.com/13999_hoge-shi_citygml_op_1_bldg_qc_result.zip",
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
			"https://example.com/13999_hoge-shi_citygml_op_1_bldg_3dtiles_lod1.zip",
		},
	}

	expected := FlowInternalResult{
		Conv: map[string][]string{
			"hoge-shi_citygml_op_1_bldg_3dtiles": {
				"https://example.com/13999_hoge-shi_citygml_op_1_bldg_3dtiles_lod1.zip",
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
			"urf_UseDistrict": {
				"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_UseDistrict_mvt_lod1.zip",
			},
			"urf_FirePreventionDistrict": {
				"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_FirePreventionDistrict_mvt_lod1.zip",
			},
			"urf_UrbanPlanningArea": {
				"https://example.com/38203_uwajima-shi_city_2025_citygml_1_op_urf_UrbanPlanningArea_mvt_lod1.zip",
			},
		},
		Dic: "https://example.com/artifacts/dictionary.json",
	}

	res := r.InternalWithFeatureType("urf", true)
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
			"veg_PlantCover": {
				"https://example.com/12345_city-name_city_2025_citygml_1_op_veg_PlantCover_mvt_lod1.zip",
			},
			"veg_SolitaryVegetationObject": {
				"https://example.com/12345_city-name_city_2025_citygml_1_op_veg_SolitaryVegetationObject_mvt_lod1.zip",
			},
		},
	}

	res := r.InternalWithFeatureType("veg", true)
	assert.Equal(t, expected, res)
}

func TestTrimOutputKeySuffixes(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"lod1 only", "something_lod1", "something"},
		{"lod2 only", "something_lod2", "something"},
		{"no_texture only", "something_no_texture", "something"},
		{"lod2 + no_texture", "something_lod2_no_texture", "something"},
		{"mvt + lod1", "something_mvt_lod1", "something"},
		{"mvt only", "something_mvt", "something"},
		{"l1 suffix", "something_l1", "something"},
		{"l2 suffix", "something_l2", "something"},
		{"no suffix", "something", "something"},
		{"lod without number", "something_lod", "something"},
		{"combined mvt + lod + no_texture", "test_mvt_lod2_no_texture", "test"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := trimOutputKeySuffixes(tt.input)
			assert.Equal(t, tt.expected, result)
		})
	}
}
