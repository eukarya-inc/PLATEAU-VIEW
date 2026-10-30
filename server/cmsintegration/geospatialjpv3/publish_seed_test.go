package geospatialjpv3

import (
	"context"
	"testing"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

type seedCMS struct {
	cms.Interface
	items map[string]*cms.Item
}

func (c *seedCMS) GetItem(_ context.Context, id string, _ bool) (*cms.Item, error) {
	if i, ok := c.items[id]; ok {
		return i, nil
	}
	return nil, cms.ErrNotFound
}

func publicAsset(url string, size uint64) map[string]any {
	return map[string]any{"type": "asset", "url": url, "totalSize": size}
}

func TestGetSeed_PlateauJGD2024(t *testing.T) {
	const (
		plateauURL = "https://example.com/13101_chiyoda-ku_city_2025_3dtiles_mvt_1_op.zip"
		jgd2024URL = "https://example.com/13101_chiyoda-ku_city_2025_3dtiles_mvt_1_op_jgd2024.zip"
	)

	newCMS := func(descJGD2024 string) *seedCMS {
		index := []*cms.Field{
			{Key: "desc_plateau", Type: "markdown", Value: "JGD2011 ${{SIZE}}"},
		}
		if descJGD2024 != "" {
			index = append(index, &cms.Field{Key: "desc_plateau_jgd2024", Type: "markdown", Value: descJGD2024})
		}
		return &seedCMS{items: map[string]*cms.Item{
			"data": {ID: "data", Fields: []*cms.Field{
				{Key: "plateau", Type: "asset", Value: publicAsset(plateauURL, 1000)},
				{Key: "plateau_jgd2024", Type: "asset", Value: publicAsset(jgd2024URL, 2000)},
			}},
			"index": {ID: "index", Fields: index},
		}}
	}
	city := &CityItem{GeospatialjpData: "data", GeospatialjpIndex: "index", Year: "2025年度"}

	seed, err := getSeed(context.Background(), newCMS("JGD2024 ${{SIZE}}"), city, "org")
	require.NoError(t, err)
	assert.Equal(t, plateauURL, seed.Plateau)
	assert.Equal(t, jgd2024URL, seed.PlateauJGD2024)
	assert.Equal(t, uint64(2000), seed.PlateauJGD2024Size)
	assert.Equal(t, "JGD2011 1.0 kB", seed.PlateauDescription)
	assert.Equal(t, "JGD2024 2.0 kB", seed.PlateauJGD2024Description)

	// the description of the JGD2011 zip is used when the JGD2024 one is empty
	seed, err = getSeed(context.Background(), newCMS(""), city, "org")
	require.NoError(t, err)
	assert.Equal(t, "JGD2011 2.0 kB", seed.PlateauJGD2024Description)
}

func TestSeed_Valid(t *testing.T) {
	assert.False(t, Seed{}.Valid())
	assert.True(t, Seed{PlateauJGD2024: "https://example.com/a_jgd2024.zip"}.Valid())
}

func TestPlateauJGD2024ResourceName(t *testing.T) {
	name := plateauJGD2024ResourceName(4)
	assert.Equal(t, "3D Tiles, MVT（JGD2024）（v4）", name)
	assert.Equal(t, 4, *extractVersionFromResourceName(name))
}
