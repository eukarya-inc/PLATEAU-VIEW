package mvttilejson

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestParseSpec(t *testing.T) {
	lod1 := 1
	tests := []struct {
		name    string
		in      string
		want    Spec
		wantErr bool
	}{
		{
			name: "no lod, exact year",
			in:   "13101-luse-2025",
			want: Spec{CityCode: "13101", Type: "luse", Year: 2025},
		},
		{
			name: "no lod, latest",
			in:   "13101-luse-latest",
			want: Spec{CityCode: "13101", Type: "luse", YearMode: YearLatest},
		},
		{
			name: "with lod",
			in:   "13101-fld-lod1-2025",
			want: Spec{CityCode: "13101", Type: "fld", LOD: &lod1, Year: 2025},
		},
		{
			name: "with lod and latest",
			in:   "13101-fld-lod1-latest",
			want: Spec{CityCode: "13101", Type: "fld", LOD: &lod1, YearMode: YearLatest},
		},
		{
			name:    "non-5-digit area",
			in:      "13-luse-2025",
			wantErr: true,
		},
		{
			name:    "non-numeric area",
			in:      "tokyo-luse-2025",
			wantErr: true,
		},
		{
			name:    "missing year",
			in:      "13101-luse",
			wantErr: true,
		},
		{
			name:    "invalid year",
			in:      "13101-luse-abcd",
			wantErr: true,
		},
		{
			name:    "trailing junk",
			in:      "13101-luse-2025-extra",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ParseSpec(tt.in)
			if tt.wantErr {
				assert.Error(t, err)
				return
			}
			assert.NoError(t, err)
			assert.Equal(t, tt.want, got)
		})
	}
}
