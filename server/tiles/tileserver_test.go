package tiles

import (
	"testing"

	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestBuildTileURLTemplate(t *testing.T) {
	tests := []struct {
		name     string
		baseURL  string
		expected string
	}{
		{
			name:     "simple URL",
			baseURL:  "https://example.com/tiles/ortho",
			expected: "https://example.com/tiles/ortho/{z}/{x}/{y}.png",
		},
		{
			name:     "URL with trailing slash",
			baseURL:  "https://example.com/tiles/ortho/",
			expected: "https://example.com/tiles/ortho/{z}/{x}/{y}.png",
		},
		{
			name:     "URL with query params",
			baseURL:  "https://example.com/tiles/ortho?token=abc",
			expected: "https://example.com/tiles/ortho/{z}/{x}/{y}.png?token=abc",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := buildTileURLTemplate(tt.baseURL)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestRangeToTileServerRange(t *testing.T) {
	tests := []struct {
		name     string
		input    Range
		expected *TileServerRangeConfig
	}{
		{
			name:     "all unlimited returns nil",
			input:    Range{ZMin: -1, ZMax: -1, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
			expected: nil,
		},
		{
			name:  "zoom range only",
			input: Range{ZMin: 5, ZMax: 15, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
			expected: &TileServerRangeConfig{
				ZMin: lo.ToPtr(uint(5)),
				ZMax: lo.ToPtr(uint(15)),
			},
		},
		{
			name:  "all ranges set",
			input: Range{ZMin: 0, ZMax: 18, XMin: 100, XMax: 200, YMin: 50, YMax: 100},
			expected: &TileServerRangeConfig{
				ZMin: lo.ToPtr(uint(0)),
				ZMax: lo.ToPtr(uint(18)),
				XMin: lo.ToPtr(uint(100)),
				XMax: lo.ToPtr(uint(200)),
				YMin: lo.ToPtr(uint(50)),
				YMax: lo.ToPtr(uint(100)),
			},
		},
		{
			name:  "partial range",
			input: Range{ZMin: 5, ZMax: -1, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
			expected: &TileServerRangeConfig{
				ZMin: lo.ToPtr(uint(5)),
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := rangeToTileServerRange(tt.input)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestTiles_ToTileServerConfig(t *testing.T) {
	tiles := Tiles{
		"ortho": TileEntry{
			Description: "正射画像",
			URLs: []lo.Entry[Range, string]{
				{
					Key:   Range{ZMin: 0, ZMax: 10, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
					Value: "https://example.com/tiles/ortho-low",
				},
				{
					Key:   Range{ZMin: 11, ZMax: 18, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
					Value: "https://example.com/tiles/ortho-high",
				},
			},
		},
		"dem": TileEntry{
			URLs: []lo.Entry[Range, string]{
				{
					Key:   Range{ZMin: -1, ZMax: -1, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
					Value: "https://example.com/tiles/dem",
				},
			},
		},
	}

	config := tiles.ToTileServerConfig("https://api.example.com")

	// Check sources exist (2 tile sources + 2 maplibre styles)
	assert.Len(t, config.Sources, 4)
	assert.Contains(t, config.Sources, "ortho")
	assert.Contains(t, config.Sources, "dem")
	assert.Contains(t, config.Sources, "dark-map")
	assert.Contains(t, config.Sources, "light-map")

	// Check ortho layers
	ortho := config.Sources["ortho"]
	assert.Equal(t, "正射画像", ortho.Description)
	assert.Len(t, ortho.Layers, 2)

	assert.Equal(t, "xyz", ortho.Layers[0].Type)
	assert.Equal(t, "https://example.com/tiles/ortho-low/{z}/{x}/{y}.png", ortho.Layers[0].URL)
	assert.NotNil(t, ortho.Layers[0].Range)
	assert.Equal(t, lo.ToPtr(uint(0)), ortho.Layers[0].Range.ZMin)
	assert.Equal(t, lo.ToPtr(uint(10)), ortho.Layers[0].Range.ZMax)

	assert.Equal(t, "xyz", ortho.Layers[1].Type)
	assert.Equal(t, "https://example.com/tiles/ortho-high/{z}/{x}/{y}.png", ortho.Layers[1].URL)
	assert.NotNil(t, ortho.Layers[1].Range)
	assert.Equal(t, lo.ToPtr(uint(11)), ortho.Layers[1].Range.ZMin)
	assert.Equal(t, lo.ToPtr(uint(18)), ortho.Layers[1].Range.ZMax)

	// Check dem layer
	dem := config.Sources["dem"]
	assert.Len(t, dem.Layers, 1)
	assert.Equal(t, "xyz", dem.Layers[0].Type)
	assert.Equal(t, "https://example.com/tiles/dem/{z}/{x}/{y}.png", dem.Layers[0].URL)
	assert.Nil(t, dem.Layers[0].Range) // All unlimited

	// Check maplibre style sources
	darkMap := config.Sources["dark-map"]
	assert.Len(t, darkMap.Layers, 1)
	assert.Equal(t, "maplibre", darkMap.Layers[0].Type)
	assert.Equal(t, "https://api.example.com/tiles/styles/dark-map", darkMap.Layers[0].URL)

	lightMap := config.Sources["light-map"]
	assert.Len(t, lightMap.Layers, 1)
	assert.Equal(t, "maplibre", lightMap.Layers[0].Type)
	assert.Equal(t, "https://api.example.com/tiles/styles/light-map", lightMap.Layers[0].URL)
}

func TestTiles_ToTileServerConfig_Empty(t *testing.T) {
	tiles := Tiles{}
	config := tiles.ToTileServerConfig("https://api.example.com")

	assert.NotNil(t, config.Sources)
	// Even with no tiles, maplibre styles should be present
	assert.Len(t, config.Sources, 2)
	assert.Contains(t, config.Sources, "dark-map")
	assert.Contains(t, config.Sources, "light-map")
}

func TestIsCOGURL(t *testing.T) {
	tests := []struct {
		url      string
		expected bool
	}{
		{"https://example.com/dem.tif", true},
		{"https://example.com/dem.TIF", true},
		{"https://example.com/dem.tiff", true},
		{"https://example.com/dem.TIFF", true},
		{"https://example.com/tiles.zip", false},
		{"https://example.com/tiles", false},
		{"https://example.com/tiles.png", false},
	}

	for _, tt := range tests {
		t.Run(tt.url, func(t *testing.T) {
			assert.Equal(t, tt.expected, isCOGURL(tt.url))
		})
	}
}

func TestTiles_ToTileServerConfig_WithCOG(t *testing.T) {
	tiles := Tiles{
		"mixed": TileEntry{
			URLs: []lo.Entry[Range, string]{
				{
					Key:   Range{ZMin: 0, ZMax: 10, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
					Value: "https://example.com/tiles/base",
				},
				{
					Key:   Range{ZMin: -1, ZMax: -1, XMin: -1, XMax: -1, YMin: -1, YMax: -1},
					Value: "https://example.com/cog/overlay.tif",
				},
			},
		},
	}

	config := tiles.ToTileServerConfig("https://api.example.com")

	// 1 tile source + 2 maplibre styles
	assert.Len(t, config.Sources, 3)
	assert.Contains(t, config.Sources, "mixed")

	mixed := config.Sources["mixed"]
	assert.Len(t, mixed.Layers, 2)

	// First layer should be XYZ
	assert.Equal(t, "xyz", mixed.Layers[0].Type)
	assert.Equal(t, "https://example.com/tiles/base/{z}/{x}/{y}.png", mixed.Layers[0].URL)
	assert.NotNil(t, mixed.Layers[0].Range)

	// Second layer should be COG
	assert.Equal(t, "cog", mixed.Layers[1].Type)
	assert.Equal(t, "https://example.com/cog/overlay.tif", mixed.Layers[1].URL)
	assert.Equal(t, 1, mixed.Layers[1].Order)
	assert.Nil(t, mixed.Layers[1].Range) // COG doesn't use range
}
