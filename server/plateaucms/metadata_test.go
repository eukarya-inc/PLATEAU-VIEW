package plateaucms

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestMetadata_ShouldUseFlow(t *testing.T) {
	tests := []struct {
		name        string
		converter   string
		features    string
		featureType string
		want        bool
	}{
		{
			name:        "flow mode: always returns true",
			converter:   ConverterFlow,
			features:    "",
			featureType: "bldg",
			want:        true,
		},
		{
			name:        "fme mode: always returns false",
			converter:   ConverterFME,
			features:    "",
			featureType: "bldg",
			want:        false,
		},
		{
			name:        "empty converter: returns false",
			converter:   "",
			features:    "",
			featureType: "bldg",
			want:        false,
		},
		{
			name:        "fme_flow mode: empty features returns false",
			converter:   ConverterFMEFlow,
			features:    "",
			featureType: "bldg",
			want:        false,
		},
		{
			name:        "fme_flow mode: feature in list returns true",
			converter:   ConverterFMEFlow,
			features:    "bldg,tran",
			featureType: "bldg",
			want:        true,
		},
		{
			name:        "fme_flow mode: feature not in list returns false",
			converter:   ConverterFMEFlow,
			features:    "bldg,tran",
			featureType: "fld",
			want:        false,
		},
		{
			name:        "fme_flow mode: single feature in list",
			converter:   ConverterFMEFlow,
			features:    "bldg",
			featureType: "bldg",
			want:        true,
		},
		{
			name:        "fme_flow mode: handles whitespace in list",
			converter:   ConverterFMEFlow,
			features:    "bldg, tran, fld",
			featureType: "tran",
			want:        true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			m := Metadata{
				Converter:           tt.converter,
				FlowEnabledFeatures: tt.features,
			}
			got := m.ShouldUseFlow(tt.featureType)
			assert.Equal(t, tt.want, got)
		})
	}
}
