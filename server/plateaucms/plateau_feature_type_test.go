package plateaucms

import (
	"testing"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
)

func TestPlateauFeatureTypeFrom(t *testing.T) {
	i := &cms.Item{
		Fields: []*cms.Field{
			{Key: "code", Value: "bldg"},
			{Key: "name", Value: "Building"},
			{Key: "qc", Value: true},
			{Key: "conv", Value: true},
		},
	}

	res := PlateauFeatureTypeFrom(i)
	assert.Equal(t, "bldg", res.Code)
	assert.Equal(t, "Building", res.Name)
	assert.True(t, res.QC)
	assert.True(t, res.Conv)

	assert.Nil(t, PlateauFeatureTypeFrom(nil))
}

// Note: Flow trigger ID tests have been moved to plateau_spec_test.go
// as Flow settings are now managed in PlateauSpec.FlowTriggers instead of PlateauFeatureType
