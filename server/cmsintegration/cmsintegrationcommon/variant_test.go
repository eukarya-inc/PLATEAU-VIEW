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
