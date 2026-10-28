package tiles

import (
	"context"
	"net/http"
	"testing"

	"github.com/jarcoal/httpmock"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestGetTiles_Errors(t *testing.T) {
	ctx := context.Background()
	httpmock.Activate()
	defer httpmock.DeactivateAndReset()

	c := lo.Must(cms.New("https://example.com", "token"))

	// the project has no tiles model
	httpmock.RegisterResponder("GET", "https://example.com/api/projects/none/models/tiles/items",
		httpmock.NewStringResponder(http.StatusNotFound, "not found"))
	httpmock.RegisterResponder("GET", "https://example.com/api/projects/none/models/tiles",
		httpmock.NewStringResponder(http.StatusNotFound, "not found"))

	res, err := getTiles(ctx, c, "none")
	assert.NoError(t, err)
	assert.Nil(t, res)

	// the tiles model exists but the CMS fails
	httpmock.RegisterResponder("GET", "https://example.com/api/projects/prj/models/tiles/items",
		httpmock.NewStringResponder(http.StatusInternalServerError, "error"))
	httpmock.RegisterResponder("GET", "https://example.com/api/projects/prj/models/tiles",
		httpmock.NewJsonResponderOrPanic(http.StatusOK, cms.Model{ID: "tiles"}))

	res, err = getTiles(ctx, c, "prj")
	assert.Error(t, err)
	assert.Nil(t, res)
}

func TestTiles_Find(t *testing.T) {
	tiles := Tiles{
		"name": TileEntry{
			URLs: []lo.Entry[Range, string]{
				{
					Key:   Range{ZMin: 0, ZMax: 0, XMin: -1, XMax: -1, YMin: 10, YMax: 20},
					Value: "a",
				},
				{
					Key:   Range{ZMin: 1, ZMax: -1, XMin: 1, XMax: 2, YMin: 10, YMax: 20},
					Value: "b",
				},
			},
		},
	}

	assert.Equal(t, "a", tiles.Find("name", 0, 0, 10))
	assert.Equal(t, "a", tiles.Find("name", 0, 0, 20))
	assert.Equal(t, "b", tiles.Find("name", 2, 1, 10))
	assert.Equal(t, "", tiles.Find("name", 2, 3, 11))
}

func TestRange_In(t *testing.T) {
	assert.True(t, Range{ZMin: -1, ZMax: -1, XMin: -1, XMax: -1, YMin: -1, YMax: -1}.In(0, 0, 10))
	assert.True(t, Range{ZMin: 0, ZMax: 0, XMin: -1, XMax: -1, YMin: 10, YMax: 20}.In(0, 0, 10))
	assert.True(t, Range{ZMin: 0, ZMax: 0, XMin: -1, XMax: -1, YMin: 10, YMax: 20}.In(0, 0, 20))
	assert.True(t, Range{ZMin: 1, ZMax: -1, XMin: 1, XMax: 2, YMin: 10, YMax: 20}.In(2, 1, 10))
	assert.False(t, Range{ZMin: 1, ZMax: -1, XMin: 1, XMax: 2, YMin: 10, YMax: 20}.In(2, 3, 11))
}

func TestExtractRange(t *testing.T) {
	assert.Equal(t, Range{
		ZMin: 0, ZMax: 0, YMin: -1, YMax: -1, XMin: -1, XMax: -1,
	}, extractRange("hogeohoge_z0.zip"))
	assert.Equal(t, Range{
		ZMin: 1, ZMax: 1, YMin: -1, YMax: -1, XMin: -1, XMax: -1,
	}, extractRange("hogeohoge_z1.zip"))
	assert.Equal(t, Range{
		ZMin: 10, ZMax: 10, YMin: -1, YMax: -1, XMin: -1, XMax: -1,
	}, extractRange("hogeohoge_z10.zip"))
	assert.Equal(t, Range{
		ZMin: 9, ZMax: 12, YMin: -1, YMax: -1, XMin: 1, XMax: 1,
	}, extractRange("hogeohoge_z9-12_x1.zip"))
	assert.Equal(t, Range{
		ZMin: -1, ZMax: -1, YMin: -1, YMax: -1, XMin: -1, XMax: -1,
	}, extractRange("hogeohoge.zip"))
}
