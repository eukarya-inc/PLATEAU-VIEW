package composite3dtiles

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestBuild(t *testing.T) {
	candidates := []Candidate{
		{URL: "https://example.com/b/tileset.json", AreaCode: "13101"},
		{URL: "https://example.com/a/tileset.json", AreaCode: "13102"},
		{URL: "https://example.com/x/tileset.json", AreaCode: "99999"}, // not in govpolygon
	}

	ts := Build(candidates, "bldg")

	assert.Equal(t, "1.1", ts.Asset.Version)
	assert.Equal(t, "ADD", ts.Root.Refine)
	// 99999 is filtered out, remaining sorted by URL
	if assert.Len(t, ts.Root.Children, 2) {
		assert.Equal(t, "https://example.com/a/tileset.json", ts.Root.Children[0].Content.URI)
		assert.Equal(t, "https://example.com/b/tileset.json", ts.Root.Children[1].Content.URI)
		assert.Equal(t, float64(-50), ts.Root.Children[0].BoundingVolume.Region[4])
		assert.Equal(t, float64(500), ts.Root.Children[0].BoundingVolume.Region[5])
	}
}

func TestBuildHeightForDem(t *testing.T) {
	ts := Build([]Candidate{{URL: "u", AreaCode: "13101"}}, "dem")
	if assert.Len(t, ts.Root.Children, 1) {
		assert.Equal(t, float64(-100), ts.Root.Children[0].BoundingVolume.Region[4])
		assert.Equal(t, float64(4000), ts.Root.Children[0].BoundingVolume.Region[5])
	}
}

func TestBuildHeightDefaultForOtherTypes(t *testing.T) {
	for _, typeCode := range []string{"tran", "frn", "veg", "luse", "urf", "unknown"} {
		ts := Build([]Candidate{{URL: "u", AreaCode: "13101"}}, typeCode)
		if assert.Len(t, ts.Root.Children, 1, typeCode) {
			assert.Equal(t, float64(-50), ts.Root.Children[0].BoundingVolume.Region[4], typeCode)
			assert.Equal(t, float64(500), ts.Root.Children[0].BoundingVolume.Region[5], typeCode)
		}
	}
}
