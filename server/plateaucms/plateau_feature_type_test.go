package plateaucms

import (
	"testing"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
)

func TestPlateauFeatureTypeFrom(t *testing.T) {
	i := &cms.Item{
		Fields: []*cms.Field{
			{
				Key:   "flow_qc_v1",
				Value: "test1",
			},
			{
				Key:   "flow_conv_v1",
				Value: "test2",
			},
			{
				Key:   "flow_qc_v2",
				Value: "test3",
			},
		},
	}

	res := PlateauFeatureTypeFrom(i)
	assert.Equal(t, &PlateauFeatureType{
		FlowQCV: map[int]string{
			1: "test1",
			2: "test3",
		},
		FlowConvV: map[int]string{
			1: "test2",
		},
	}, res)

	assert.Nil(t, PlateauFeatureTypeFrom(nil))
}

func TestPlateauFeatureType_FlowQCTriggerID(t *testing.T) {
	ft := &PlateauFeatureType{
		FlowQCV: map[int]string{
			1: "test1",
			2: "test3",
		},
		FlowConvV: map[int]string{
			1: "test2",
		},
		FlowQC: "test4",
	}

	assert.Equal(t, "test1", ft.FlowQCTriggerID(1))
	assert.Equal(t, "test3", ft.FlowQCTriggerID(2))
	assert.Equal(t, "test4", ft.FlowQCTriggerID(3))
}

func TestPlateauFeatureType_FlowConvTriggerID(t *testing.T) {
	ft := &PlateauFeatureType{
		FlowQCV: map[int]string{
			1: "test1",
			2: "test3",
		},
		FlowConvV: map[int]string{
			1: "test2",
		},
		FlowConv: "test4",
	}

	assert.Equal(t, "test2", ft.FlowConvTriggerID(1))
	assert.Equal(t, "test4", ft.FlowConvTriggerID(2))
}
