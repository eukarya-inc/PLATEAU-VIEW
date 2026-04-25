package mvttilejson

import (
	"testing"

	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestSelect(t *testing.T) {
	lod1 := "1"
	cityA := lo.ToPtr("13101")
	cityB := lo.ToPtr("14100")
	wardB := lo.ToPtr("14101")

	datasets := []Input{
		// 13101 luse no-LOD year 2024 + 2025 → newest year wins under latest
		{URL: "u24", Format: "MVT", TypeCode: "luse", Year: 2024, CityCode: cityA, Layers: []string{"luse"}},
		{URL: "u25", Format: "MVT", TypeCode: "luse", Year: 2025, CityCode: cityA, Layers: []string{"luse"}},
		// 13101 luse with LOD1 — must NOT match no-lod spec
		{URL: "ulod1", Format: "MVT", TypeCode: "luse", Year: 2025, LOD: &lod1, CityCode: cityA, Layers: []string{"luse"}},
		// 14101 ward code matches via WardCode preference
		{URL: "ward", Format: "MVT", TypeCode: "luse", Year: 2025, CityCode: cityB, WardCode: wardB, Layers: []string{"luse"}},
		// 13101 wrong type
		{URL: "wrongtype", Format: "MVT", TypeCode: "fld", Year: 2025, CityCode: cityA},
		// 13101 wrong format
		{URL: "wrongfmt", Format: "3D Tiles", TypeCode: "luse", Year: 2025, CityCode: cityA},
	}

	t.Run("exact year, no lod", func(t *testing.T) {
		got := Select(datasets, Spec{CityCode: "13101", Type: "luse", Year: 2025})
		if assert.NotNil(t, got) {
			assert.Equal(t, "u25", got.URL)
		}
	})

	t.Run("latest year picks newest", func(t *testing.T) {
		got := Select(datasets, Spec{CityCode: "13101", Type: "luse", YearMode: YearLatest})
		if assert.NotNil(t, got) {
			assert.Equal(t, "u25", got.URL)
		}
	})

	t.Run("with lod selector", func(t *testing.T) {
		one := 1
		got := Select(datasets, Spec{CityCode: "13101", Type: "luse", LOD: &one, Year: 2025})
		if assert.NotNil(t, got) {
			assert.Equal(t, "ulod1", got.URL)
		}
	})

	t.Run("ward code match", func(t *testing.T) {
		got := Select(datasets, Spec{CityCode: "14101", Type: "luse", Year: 2025})
		if assert.NotNil(t, got) {
			assert.Equal(t, "ward", got.URL)
		}
	})

	t.Run("no match returns nil", func(t *testing.T) {
		got := Select(datasets, Spec{CityCode: "99999", Type: "luse", Year: 2025})
		assert.Nil(t, got)
	})
}
