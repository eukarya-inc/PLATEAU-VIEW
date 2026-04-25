package datacatalog

import (
	"sort"
	"testing"

	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestBuildSpec(t *testing.T) {
	assert.Equal(t, "13101-bldg-lod1-2025", buildSpec("13101", "bldg", "1", nil, 2025))
	assert.Equal(t, "13101-bldg-lod2-texture-2025", buildSpec("13101", "bldg", "2", lo.ToPtr(true), 2025))
	assert.Equal(t, "13101-bldg-lod2-notexture-2025", buildSpec("13101", "bldg", "2", lo.ToPtr(false), 2025))
	assert.Equal(t, "all-bldg-lod1-2025", buildSpec("all", "bldg", "1", nil, 2025))
}

func TestBuildDatasetCompositeURL(t *testing.T) {
	host := "http://api.example.com"
	lod1 := "1"

	t.Run("ward code preferred", func(t *testing.T) {
		u := buildDatasetCompositeURL(host, &SimpleDatasetsResponseDataset{
			Format:   "3D Tiles",
			TypeCode: "bldg",
			LOD:      &lod1,
			Texture:  lo.ToPtr(true),
			CityCode: lo.ToPtr("13100"),
			WardCode: lo.ToPtr("13101"),
			Year:     2025,
		})
		assert.Equal(t, "http://api.example.com/datacatalog/3dtiles/13101-bldg-lod1-texture-2025/tileset.json", u)
	})

	t.Run("city code fallback", func(t *testing.T) {
		u := buildDatasetCompositeURL(host, &SimpleDatasetsResponseDataset{
			Format:   "3D Tiles",
			TypeCode: "bldg",
			LOD:      &lod1,
			Texture:  lo.ToPtr(false),
			CityCode: lo.ToPtr("27100"),
			Year:     2025,
		})
		assert.Equal(t, "http://api.example.com/datacatalog/3dtiles/27100-bldg-lod1-notexture-2025/tileset.json", u)
	})

	t.Run("missing host", func(t *testing.T) {
		u := buildDatasetCompositeURL("", &SimpleDatasetsResponseDataset{
			TypeCode: "bldg", LOD: &lod1, CityCode: lo.ToPtr("27100"), Year: 2025,
		})
		assert.Empty(t, u)
	})

	t.Run("missing area", func(t *testing.T) {
		u := buildDatasetCompositeURL(host, &SimpleDatasetsResponseDataset{
			TypeCode: "bldg", LOD: &lod1, Year: 2025,
		})
		assert.Empty(t, u)
	})
}

func TestBuildCompositeTilesets(t *testing.T) {
	host := "http://api.example.com"
	lod1 := "1"
	lod2 := "2"

	datasets := []*SimpleDatasetsResponseDataset{
		// pref 13: bldg lod1 — only textured exists
		{Format: "3D Tiles", TypeCode: "bldg", Type: "建築物モデル", LOD: &lod1, Texture: lo.ToPtr(true), PrefCode: "13", Pref: "東京都", CityCode: lo.ToPtr("13101"), Year: 2025},
		// pref 13: bldg lod2 — both textured and non-textured exist
		{Format: "3D Tiles", TypeCode: "bldg", Type: "建築物モデル", LOD: &lod2, Texture: lo.ToPtr(true), PrefCode: "13", Pref: "東京都", CityCode: lo.ToPtr("13101"), Year: 2025},
		{Format: "3D Tiles", TypeCode: "bldg", Type: "建築物モデル", LOD: &lod2, Texture: lo.ToPtr(false), PrefCode: "13", Pref: "東京都", CityCode: lo.ToPtr("13101"), Year: 2025},
		// pref 27: bldg lod1
		{Format: "3D Tiles", TypeCode: "bldg", Type: "建築物モデル", LOD: &lod1, Texture: lo.ToPtr(true), PrefCode: "27", Pref: "大阪府", CityCode: lo.ToPtr("27100"), Year: 2025},
		// MVT — must be ignored
		{Format: "MVT", TypeCode: "luse", LOD: &lod1, PrefCode: "13", Pref: "東京都", CityCode: lo.ToPtr("13101"), Year: 2025},
	}

	out := buildCompositeTilesets(host, datasets)
	ids := make([]string, len(out))
	for i, e := range out {
		ids[i] = e.ID
	}
	sort.Strings(ids)

	// expected: all-bldg-lod1-2025 (only textured),
	//           all-bldg-lod2-2025 + texture + notexture (mixed),
	//           13-bldg-lod1-2025, 13-bldg-lod2 (auto+texture+notexture),
	//           27-bldg-lod1-2025
	assert.Equal(t, []string{
		"13-bldg-lod1-2025",
		"13-bldg-lod2-2025",
		"13-bldg-lod2-notexture-2025",
		"13-bldg-lod2-texture-2025",
		"27-bldg-lod1-2025",
		"all-bldg-lod1-2025",
		"all-bldg-lod2-2025",
		"all-bldg-lod2-notexture-2025",
		"all-bldg-lod2-texture-2025",
	}, ids)

	// spot check pref entry has prefCode populated
	for _, e := range out {
		if e.ID == "13-bldg-lod1-2025" {
			assert.Equal(t, "pref", e.Area)
			assert.NotNil(t, e.PrefCode)
			assert.Equal(t, "13", *e.PrefCode)
			assert.Nil(t, e.Texture) // auto variant
			assert.Equal(t, "http://api.example.com/datacatalog/3dtiles/13-bldg-lod1-2025/tileset.json", e.URL)
		}
		if e.ID == "all-bldg-lod1-2025" {
			assert.Equal(t, "all", e.Area)
			assert.Nil(t, e.PrefCode)
		}
	}
}

func TestBuildCompositeTilesets_NoHost(t *testing.T) {
	out := buildCompositeTilesets("", []*SimpleDatasetsResponseDataset{
		{Format: "3D Tiles", TypeCode: "bldg", LOD: lo.ToPtr("1"), PrefCode: "13", Year: 2025},
	})
	assert.Nil(t, out)
}
