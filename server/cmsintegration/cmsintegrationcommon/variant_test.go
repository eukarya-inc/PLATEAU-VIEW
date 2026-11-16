package cmsintegrationcommon

import (
	"context"
	"fmt"
	"testing"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
)

func TestIsJGD2024Asset(t *testing.T) {
	assert.True(t, IsJGD2024Asset("13101_chiyoda-ku_city_2025_citygml_1_op_bldg_3dtiles_lod2_jgd2024.zip"))
	assert.True(t, IsJGD2024Asset("https://example.com/assets/a/13101_chiyoda-ku_city_2025_citygml_1_op_bldg_3dtiles_lod2_full_jgd2024.zip"))
	assert.True(t, IsJGD2024Asset("13101_chiyoda-ku_city_2025_citygml_1_op_bldg_3dtiles_lod2_jgd2024_full"))
	assert.False(t, IsJGD2024Asset("13101_chiyoda-ku_city_2025_citygml_1_op_bldg_3dtiles_lod2_full.zip"))
	assert.False(t, IsJGD2024Asset("https://example.com/13101_chiyoda-ku_city_2025_citygml_1_op_bldg_3dtiles_lod2.zip"))
}

func TestMergeConvertedAssets(t *testing.T) {
	names := map[string]string{
		"a2011":     "x_bldg_3dtiles_lod2.zip",
		"a2011full": "x_bldg_3dtiles_lod2_full.zip",
		"a2024":     "x_bldg_3dtiles_lod2_jgd2024.zip",
		"n2011":     "https://example.com/x_bldg_3dtiles_lod2.zip",
		"n2024":     "https://example.com/x_bldg_3dtiles_lod2_jgd2024.zip",
		"n2024full": "https://example.com/x_bldg_3dtiles_lod2_full_jgd2024.zip",
	}

	res, err := MergeConvertedAssets([]string{"a2011", "a2011full", "a2024"}, []string{"n2024", "n2024full"}, names)
	assert.NoError(t, err)
	assert.Equal(t, []string{"a2011", "a2011full", "n2024", "n2024full"}, res)

	res, err = MergeConvertedAssets([]string{"a2011", "a2011full", "a2024"}, []string{"n2011"}, names)
	assert.NoError(t, err)
	assert.Equal(t, []string{"a2024", "n2011"}, res)

	_, err = MergeConvertedAssets(nil, []string{"n2011", "n2024"}, names)
	assert.ErrorContains(t, err, "mix JGD2011 and JGD2024")

	// an existing asset that no longer exists in CMS is removed
	res, err = MergeConvertedAssets([]string{"deleted", "a2024"}, []string{"n2011"}, names)
	assert.NoError(t, err)
	assert.Equal(t, []string{"a2024", "n2011"}, res)

	_, err = MergeConvertedAssets(nil, []string{"unknown"}, names)
	assert.ErrorContains(t, err, "unknown asset name: unknown")
}

func TestIsCRSIndependentAsset(t *testing.T) {
	assert.True(t, IsCRSIndependentAsset("33211_bizen-shi_city_2023_citygml_1_op_tran_mvt_lod1.zip"))
	assert.True(t, IsCRSIndependentAsset("https://example.com/x_urf_AreaClassification_mvt_lod1.zip"))
	assert.True(t, IsCRSIndependentAsset("x_dm_geometric_attributes.zip"))
	assert.False(t, IsCRSIndependentAsset("x_tran_3dtiles_lod1_jgd2024.zip"))
	assert.False(t, IsCRSIndependentAsset("x_tran_3dtiles_lod2.zip"))
}

func TestMergeConvertedAssets_MVT(t *testing.T) {
	// tran has 3D Tiles and MVT in one field. MVT has no heights, so it has no `_jgd2024`
	// suffix even when it comes from a JGD2024 conversion.
	names := map[string]string{
		"t2011": "x_tran_3dtiles_lod1.zip",
		"t2024": "x_tran_3dtiles_lod1_jgd2024.zip",
		"mvt":   "x_tran_mvt_lod1.zip",
		"n2011": "https://example.com/x_tran_3dtiles_lod1.zip",
		"n2024": "https://example.com/x_tran_3dtiles_lod1_jgd2024.zip",
		"nmvt":  "https://example.com/x_tran_mvt_lod1.zip",
	}

	// a JGD2024 conversion with MVT is not a mix, and its MVT replaces the existing one
	res, err := MergeConvertedAssets([]string{"t2011", "mvt"}, []string{"n2024", "nmvt"}, names)
	assert.NoError(t, err)
	assert.Equal(t, []string{"n2024", "nmvt", "t2011"}, res)

	// a JGD2024 conversion without MVT keeps the existing MVT
	res, err = MergeConvertedAssets([]string{"t2011", "t2024", "mvt"}, []string{"n2024"}, names)
	assert.NoError(t, err)
	assert.Equal(t, []string{"mvt", "n2024", "t2011"}, res)

	// a JGD2011 conversion with MVT keeps the JGD2024 3D Tiles
	res, err = MergeConvertedAssets([]string{"t2011", "t2024", "mvt"}, []string{"n2011", "nmvt"}, names)
	assert.NoError(t, err)
	assert.Equal(t, []string{"n2011", "nmvt", "t2024"}, res)

	// a conversion of MVT only keeps every 3D Tiles
	res, err = MergeConvertedAssets([]string{"t2011", "t2024", "mvt"}, []string{"nmvt"}, names)
	assert.NoError(t, err)
	assert.Equal(t, []string{"nmvt", "t2011", "t2024"}, res)

	// 3D Tiles of both CRSs are still a mix
	_, err = MergeConvertedAssets(nil, []string{"n2011", "n2024", "nmvt"}, names)
	assert.ErrorContains(t, err, "mix JGD2011 and JGD2024")
}

func TestMergeConvertedItems_MVT(t *testing.T) {
	names := map[string]string{
		"t2011": "x_veg_PlantCover_3dtiles_lod1.zip",
		"mvt":   "x_veg_PlantCover_mvt_lod1.zip",
		"n2024": "x_veg_PlantCover_3dtiles_lod1_jgd2024.zip",
		"nmvt":  "x_veg_PlantCover_mvt_lod1.zip",
	}
	existing := []FeatureItemDatum{
		{ID: "i3", Key: "veg_PlantCover_3dtiles", Data: []string{"t2011"}},
		{ID: "im", Key: "veg_PlantCover_mvt", Data: []string{"mvt"}},
	}

	res, err := MergeConvertedItems(existing, map[string][]string{
		"veg_PlantCover_3dtiles": {"n2024"},
		"veg_PlantCover_mvt":     {"nmvt"},
	}, []string{"veg_PlantCover_3dtiles", "veg_PlantCover_mvt"}, names, func() string { return "new" })
	assert.NoError(t, err)
	assert.Equal(t, []FeatureItemDatum{
		{ID: "i3", Key: "veg_PlantCover_3dtiles", Data: []string{"n2024", "t2011"}},
		{ID: "im", Key: "veg_PlantCover_mvt", Data: []string{"nmvt"}},
	}, res)
}

func TestMergeConvertedItems(t *testing.T) {
	names := map[string]string{
		"a2011": "x_fld_natl_a_3dtiles_l1.zip",
		"a2024": "x_fld_natl_a_3dtiles_l1_jgd2024.zip",
		"b2011": "x_fld_natl_b_3dtiles_l1.zip",
		"n2024": "x_fld_natl_a_3dtiles_l1_jgd2024.zip",
		"m2024": "x_fld_natl_c_3dtiles_l1_jgd2024.zip",
	}
	existing := []FeatureItemDatum{
		{ID: "ia", Key: "a", Data: []string{"a2011", "a2024"}, Name: "name"},
		{ID: "ib", Key: "b", Data: []string{"b2011"}},
	}

	res, err := MergeConvertedItems(existing, map[string][]string{"a": {"n2024"}, "c": {"m2024"}}, []string{"a", "c"}, names, func() string { return "new" })
	assert.NoError(t, err)
	assert.Equal(t, []FeatureItemDatum{
		{ID: "ia", Key: "a", Data: []string{"a2011", "n2024"}},
		{ID: "new", Key: "c", Data: []string{"m2024"}},
		// the JGD2011 group that the JGD2024 conversion did not output is kept
		{ID: "ib", Key: "b", Data: []string{"b2011"}},
	}, res)

	// a JGD2011 conversion replaces every JGD2011 asset, and a group left empty is removed
	res, err = MergeConvertedItems([]FeatureItemDatum{{ID: "ib", Key: "b", Data: []string{"b2011"}}}, map[string][]string{"a": {"a2011"}}, []string{"a"}, names, func() string { return "new" })
	assert.NoError(t, err)
	assert.Equal(t, []FeatureItemDatum{{ID: "new", Key: "a", Data: []string{"a2011"}}}, res)
}

func TestAssetNames(t *testing.T) {
	c := &CMSMock{
		MockAsset: func(ctx context.Context, id string) (*cms.Asset, error) {
			switch id {
			case "a":
				return &cms.Asset{ID: id, Name: "a.zip"}, nil
			case "b":
				return &cms.Asset{ID: id, URL: "https://example.com/b.zip"}, nil
			}
			return nil, fmt.Errorf("failed to get an asset: %w", cms.ErrNotFound)
		},
	}

	names, err := AssetNames(context.Background(), c, []string{"a", "b", "deleted", "a", ""})
	assert.NoError(t, err)
	assert.Equal(t, map[string]string{"a": "a.zip", "b": "https://example.com/b.zip"}, names)
}
