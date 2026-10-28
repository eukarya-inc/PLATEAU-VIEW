package plateauapi

import (
	"testing"

	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestFilterPlateauDatasetItems(t *testing.T) {
	item := func(id string, crs *Crs, set *AttributeSet) *PlateauDatasetItem {
		return &PlateauDatasetItem{ID: ID(id), Crs: crs, AttributeSet: set}
	}
	j2011, j2024 := lo.ToPtr(CrsJgd2011), lo.ToPtr(CrsJgd2024)
	def, full := lo.ToPtr(AttributeSetDefault), lo.ToPtr(AttributeSetFull)

	items := []*PlateauDatasetItem{
		item("lod1", j2011, def),
		item("lod1_full", j2011, full),
		// only the full item exists for LOD2 in JGD2011
		item("lod2_full", j2011, full),
		item("lod1_jgd2024", j2024, def),
		item("lod1_jgd2024_full", j2024, full),
		item("mvt", nil, nil),
		// an item from before the variants were introduced
		item("legacy", nil, nil),
	}
	ids := func(items []*PlateauDatasetItem) []string {
		return lo.Map(items, func(i *PlateauDatasetItem, _ int) string { return i.ID.String() })
	}

	assert.Equal(t, []string{"lod1", "lod2_full", "mvt", "legacy"}, ids(FilterPlateauDatasetItems(items, nil, nil)))
	assert.Equal(t, []string{"lod1", "mvt", "legacy"}, ids(FilterPlateauDatasetItems(items, nil, def)))
	assert.Equal(t, []string{"lod1_full", "lod2_full"}, ids(FilterPlateauDatasetItems(items, j2011, full)))
	assert.Equal(t, []string{"lod1_jgd2024", "mvt", "legacy"}, ids(FilterPlateauDatasetItems(items, j2024, nil)))
	assert.Equal(t, []string{"lod1_jgd2024_full"}, ids(FilterPlateauDatasetItems(items, j2024, full)))

	// JGD2011 items are never returned in place of missing JGD2024 ones
	assert.Equal(t, []string{"mvt"}, ids(FilterPlateauDatasetItems([]*PlateauDatasetItem{item("lod2", j2011, def), item("mvt", nil, nil)}, j2024, nil)))
}
