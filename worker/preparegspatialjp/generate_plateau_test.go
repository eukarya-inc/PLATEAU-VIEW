package preparegspatialjp

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestIncludeInPlateau(t *testing.T) {
	const (
		bldg2011 = "https://example.com/13101_chiyoda-ku_city_2025_citygml_1_op_bldg_3dtiles_lod2.zip"
		bldg2024 = "https://example.com/13101_chiyoda-ku_city_2025_citygml_1_op_bldg_3dtiles_lod2_full_jgd2024.zip"
		tran     = "https://example.com/13101_chiyoda-ku_city_2025_citygml_1_op_tran_mvt_lod1.zip"
		brid     = "https://example.com/13101_chiyoda-ku_city_2025_citygml_1_op_brid_dm_geometric_attributes.zip"
	)

	assert.True(t, includeInPlateau(bldg2011, false))
	assert.False(t, includeInPlateau(bldg2024, false))
	assert.True(t, includeInPlateau(tran, false))

	assert.False(t, includeInPlateau(bldg2011, true))
	assert.True(t, includeInPlateau(bldg2024, true))
	assert.True(t, includeInPlateau(tran, true))
	assert.True(t, includeInPlateau(brid, true))
}

func TestHasJGD2024Data(t *testing.T) {
	m := MergeContext{
		FeatureTypes: []string{"bldg", "tran"},
		AllFeatureItems: map[string]FeatureItem{
			"bldg": {Data: []string{"https://example.com/x_bldg_3dtiles_lod2.zip"}},
			"tran": {Data: []string{"https://example.com/x_tran_mvt_lod1.zip"}},
		},
	}
	assert.False(t, hasJGD2024Data(m))

	m.AllFeatureItems["bldg"] = FeatureItem{Data: []string{"https://example.com/x_bldg_3dtiles_lod2.zip", "https://example.com/x_bldg_3dtiles_lod2_jgd2024.zip"}}
	assert.True(t, hasJGD2024Data(m))
}
