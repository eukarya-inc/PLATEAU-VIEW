package datacatalogv3

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestPlateauDatasetItemSeedFrom_Variants(t *testing.T) {
	urls := []string{
		"https://example.com/11111_bar-shi_city_2025_citygml_1_op_bldg_3dtiles_lod2.zip",
		"https://example.com/11111_bar-shi_city_2025_citygml_1_op_bldg_3dtiles_lod2_full.zip",
		// an unreadable name is skipped without dropping the assets after it
		"https://example.com/11111_bar-shi_city_2025_citygml_1_op_bldg_unknown.zip",
		"https://example.com/11111_bar-shi_city_2025_citygml_1_op_bldg_3dtiles_lod2_no_texture_jgd2024.zip",
		"https://example.com/11111_bar-shi_city_2025_citygml_1_op_bldg_3dtiles_lod2_no_texture_full_jgd2024.zip",
		"https://example.com/11111_bar-shi_city_2025_citygml_1_op_bldg_mvt_lod0.zip",
	}
	seed := plateauDatasetSeed{
		AssetURLs:   urls,
		Assets:      ParseAssetUrls(urls),
		TargetArea:  &plateauapi.City{Code: "11111"},
		DatasetType: &plateauapi.PlateauDatasetType{Code: "bldg", Name: "建築物モデル"},
		Year:        2025,
	}

	seeds, warning := plateauDatasetItemSeedFrom(seed)
	assert.Contains(t, warning, "plateau 11111 bldg[2]: invalid asset name ex: 11111_bar-shi_city_2025_citygml_1_op_bldg_unknown")

	items := lo.Map(seeds, func(s plateauDatasetItemSeed, _ int) *plateauapi.PlateauDatasetItem {
		return seedToDatasetItem(s, "11111_bldg", true)
	})

	type got struct {
		ID           string
		Name         string
		Crs          *plateauapi.Crs
		AttributeSet *plateauapi.AttributeSet
	}
	assert.Equal(t, []got{
		{"di_11111_bldg_lod2", "LOD2", lo.ToPtr(plateauapi.CrsJgd2011), lo.ToPtr(plateauapi.AttributeSetDefault)},
		{"di_11111_bldg_lod2_full", "LOD2（全属性）", lo.ToPtr(plateauapi.CrsJgd2011), lo.ToPtr(plateauapi.AttributeSetFull)},
		{"di_11111_bldg_lod2_no_texture_jgd2024", "LOD2（テクスチャなし）", lo.ToPtr(plateauapi.CrsJgd2024), lo.ToPtr(plateauapi.AttributeSetDefault)},
		{"di_11111_bldg_lod2_no_texture_jgd2024_full", "LOD2（テクスチャなし・全属性）", lo.ToPtr(plateauapi.CrsJgd2024), lo.ToPtr(plateauapi.AttributeSetFull)},
		{"di_11111_bldg_lod0", "LOD0", nil, nil},
	}, lo.Map(items, func(i *plateauapi.PlateauDatasetItem, _ int) got {
		return got{i.ID.String(), i.Name, i.Crs, i.AttributeSet}
	}))

	// by default, only the JGD2011 default item and the MVT are returned
	assert.Equal(t, []string{"di_11111_bldg_lod2", "di_11111_bldg_lod0"}, itemIDs(plateauapi.FilterPlateauDatasetItems(items, nil, nil)))
}

func itemIDs(items []*plateauapi.PlateauDatasetItem) []string {
	return lo.Map(items, func(i *plateauapi.PlateauDatasetItem, _ int) string { return i.ID.String() })
}
