package datacatalogv3

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/stretchr/testify/assert"
)

func TestConvertPlateauRaw_InvalidDatasetTypeContinues(t *testing.T) {
	items := []*PlateauFeatureItem{
		{ID: "a", FeatureType: "aaa", City: "city"},
		{ID: "b", FeatureType: "bldg", City: "unknown"},
	}
	dts := map[string]plateauapi.DatasetType{
		"aaa":  &plateauapi.GenericDatasetType{Code: "aaa", Name: "AAA"},
		"bldg": &plateauapi.PlateauDatasetType{Code: "bldg", Name: "建築物モデル"},
	}
	fts := map[string]*FeatureType{
		"aaa":  {Code: "aaa"},
		"bldg": {Code: "bldg"},
	}

	res, warnings := convertPlateauRaw(items, true, "", false, nil, dts, fts, newInternalContext())
	assert.Empty(t, res)
	assert.Equal(t, []string{
		"plateau aaa: invalid dataset type: AAA",
		"plateau b bldg: invalid city: unknown",
	}, warnings)
}
