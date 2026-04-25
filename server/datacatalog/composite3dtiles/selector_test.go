package composite3dtiles

import (
	"sort"
	"testing"

	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestSelect(t *testing.T) {
	lod1 := "1"
	lod2 := "2"
	tex := lo.ToPtr(true)
	noTex := lo.ToPtr(false)
	cityA := lo.ToPtr("13101")
	cityB := lo.ToPtr("14100")
	wardB := lo.ToPtr("14101")
	cityC := lo.ToPtr("27100")

	datasets := []Input{
		// area A (Tokyo / Chiyoda): textured + non-textured at lod2 → textured wins
		{URL: "u1", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod2, Texture: tex, PrefCode: "13", CityCode: cityA},
		{URL: "u2", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod2, Texture: noTex, PrefCode: "13", CityCode: cityA},
		// area A: lod1 → not selected when spec is lod2
		{URL: "u3", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod1, Texture: tex, PrefCode: "13", CityCode: cityA},
		// area B (Kanagawa / Yokohama Ward): ward code preferred over city code
		{URL: "u4", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod2, Texture: tex, PrefCode: "14", CityCode: cityB, WardCode: wardB},
		// area C (Osaka): another prefecture
		{URL: "u8", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod2, Texture: tex, PrefCode: "27", CityCode: cityC},
		// wrong year
		{URL: "u5", Format: "3D Tiles", TypeCode: "bldg", Year: 2024, LOD: &lod2, Texture: tex, PrefCode: "13", CityCode: cityA},
		// wrong type
		{URL: "u6", Format: "3D Tiles", TypeCode: "tran", Year: 2025, LOD: &lod2, Texture: tex, PrefCode: "13", CityCode: cityA},
		// wrong format
		{URL: "u7", Format: "MVT", TypeCode: "bldg", Year: 2025, LOD: &lod2, Texture: tex, PrefCode: "13", CityCode: cityA},
	}

	all := Area{Kind: AreaAll}

	t.Run("texture preferred", func(t *testing.T) {
		got := Select(datasets, Spec{Area: all, Type: "bldg", LOD: 2, Year: 2025})
		sort.Slice(got, func(i, j int) bool { return got[i].AreaCode < got[j].AreaCode })
		assert.Equal(t, []Candidate{
			{URL: "u1", AreaCode: "13101"},
			{URL: "u4", AreaCode: "14101"},
			{URL: "u8", AreaCode: "27100"},
		}, got)
	})

	t.Run("notexture only", func(t *testing.T) {
		got := Select(datasets, Spec{Area: all, Type: "bldg", LOD: 2, Texture: TextureNone, Year: 2025})
		assert.Equal(t, []Candidate{{URL: "u2", AreaCode: "13101"}}, got)
	})

	t.Run("texture only", func(t *testing.T) {
		got := Select(datasets, Spec{Area: all, Type: "bldg", LOD: 2, Texture: TextureOnly, Year: 2025})
		sort.Slice(got, func(i, j int) bool { return got[i].AreaCode < got[j].AreaCode })
		assert.Equal(t, []Candidate{
			{URL: "u1", AreaCode: "13101"},
			{URL: "u4", AreaCode: "14101"},
			{URL: "u8", AreaCode: "27100"},
		}, got)
	})

	t.Run("no match returns empty", func(t *testing.T) {
		got := Select(datasets, Spec{Area: all, Type: "bldg", LOD: 3, Year: 2025})
		assert.Empty(t, got)
	})

	t.Run("prefecture filter", func(t *testing.T) {
		got := Select(datasets, Spec{Area: Area{Kind: AreaPref, Code: "13"}, Type: "bldg", LOD: 2, Year: 2025})
		assert.Equal(t, []Candidate{{URL: "u1", AreaCode: "13101"}}, got)
	})

	t.Run("city/ward filter by ward code", func(t *testing.T) {
		got := Select(datasets, Spec{Area: Area{Kind: AreaCity, Code: "14101"}, Type: "bldg", LOD: 2, Year: 2025})
		assert.Equal(t, []Candidate{{URL: "u4", AreaCode: "14101"}}, got)
	})

	t.Run("city/ward filter by city code", func(t *testing.T) {
		got := Select(datasets, Spec{Area: Area{Kind: AreaCity, Code: "27100"}, Type: "bldg", LOD: 2, Year: 2025})
		assert.Equal(t, []Candidate{{URL: "u8", AreaCode: "27100"}}, got)
	})
}

func TestSelectMaxLOD(t *testing.T) {
	lod1 := "1"
	lod2 := "2"
	lod3 := "3"
	tex := lo.ToPtr(true)
	noTex := lo.ToPtr(false)
	cityA := lo.ToPtr("13101")
	cityB := lo.ToPtr("13102")
	cityC := lo.ToPtr("13103")

	datasets := []Input{
		// area A: has LOD1 textured + LOD2 textured -> LOD2 wins under maxlod3
		{URL: "a1", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod1, Texture: tex, PrefCode: "13", CityCode: cityA},
		{URL: "a2", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod2, Texture: tex, PrefCode: "13", CityCode: cityA},
		// area B: only LOD1 -> LOD1 picked under maxlod2
		{URL: "b1", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod1, Texture: tex, PrefCode: "13", CityCode: cityB},
		// area C: LOD2 textured + LOD2 non-textured + LOD3 (above cap)
		{URL: "c1", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod2, Texture: tex, PrefCode: "13", CityCode: cityC},
		{URL: "c2", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod2, Texture: noTex, PrefCode: "13", CityCode: cityC},
		{URL: "c3", Format: "3D Tiles", TypeCode: "bldg", Year: 2025, LOD: &lod3, Texture: tex, PrefCode: "13", CityCode: cityC},
	}

	t.Run("maxlod2 picks highest available <=2 with texture tiebreak", func(t *testing.T) {
		got := Select(datasets, Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 2, LODMode: LODMax, Year: 2025})
		sort.Slice(got, func(i, j int) bool { return got[i].AreaCode < got[j].AreaCode })
		assert.Equal(t, []Candidate{
			{URL: "a2", AreaCode: "13101"}, // LOD2 > LOD1
			{URL: "b1", AreaCode: "13102"}, // only LOD1 available
			{URL: "c1", AreaCode: "13103"}, // LOD3 capped out, LOD2 textured wins over non-textured
		}, got)
	})

	t.Run("maxlod3 includes LOD3", func(t *testing.T) {
		got := Select(datasets, Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 3, LODMode: LODMax, Year: 2025})
		sort.Slice(got, func(i, j int) bool { return got[i].AreaCode < got[j].AreaCode })
		assert.Equal(t, []Candidate{
			{URL: "a2", AreaCode: "13101"},
			{URL: "b1", AreaCode: "13102"},
			{URL: "c3", AreaCode: "13103"}, // LOD3 wins
		}, got)
	})

	t.Run("maxlod2 notexture picks highest non-textured <=2", func(t *testing.T) {
		got := Select(datasets, Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 2, LODMode: LODMax, Texture: TextureNone, Year: 2025})
		assert.Equal(t, []Candidate{{URL: "c2", AreaCode: "13103"}}, got)
	})
}
