package cmsintegrationcommon

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestGetBracketContent(t *testing.T) {
	assert.Equal(t, "", GetLastBracketContent("トンネルモデル"))
	assert.Equal(t, "tran", GetLastBracketContent("交通（道路）モデル（tran）"))
}

func TestExtractBaseFeatureType(t *testing.T) {
	tests := []struct {
		code string
		want string
	}{
		{"bldg", "bldg"},
		{"bldg2", "bldg"},
		{"bldg10", "bldg"},
		{"tran", "tran"},
		{"tran2", "tran"},
		{"urf", "urf"},
		{"veg", "veg"},
		{"", ""},
	}
	for _, tt := range tests {
		t.Run(tt.code, func(t *testing.T) {
			assert.Equal(t, tt.want, ExtractBaseFeatureType(tt.code))
		})
	}
}

func TestIsDerivedFeatureType(t *testing.T) {
	tests := []struct {
		code string
		want bool
	}{
		{"bldg", false},
		{"bldg2", true},
		{"bldg10", true},
		{"tran", false},
		{"tran2", true},
		{"urf", false},
		{"", false},
	}
	for _, tt := range tests {
		t.Run(tt.code, func(t *testing.T) {
			assert.Equal(t, tt.want, IsDerivedFeatureType(tt.code))
		})
	}
}
