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
