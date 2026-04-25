package composite3dtiles

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestParseSpec(t *testing.T) {
	tests := []struct {
		name    string
		in      string
		want    Spec
		wantErr bool
	}{
		{
			name: "all bldg lod2 textured",
			in:   "all-bldg-lod2-2025",
			want: Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 2, Year: 2025},
		},
		{
			name: "all bldg lod1 no texture",
			in:   "all-bldg-lod1-notexture-2025",
			want: Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 1, Texture: TextureNone, Year: 2025},
		},
		{
			name: "all bldg lod2 texture only",
			in:   "all-bldg-lod2-texture-2025",
			want: Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 2, Texture: TextureOnly, Year: 2025},
		},
		{
			name: "prefecture",
			in:   "13-bldg-lod2-2025",
			want: Spec{Area: Area{Kind: AreaPref, Code: "13"}, Type: "bldg", LOD: 2, Year: 2025},
		},
		{
			name: "city/ward 5-digit",
			in:   "13101-bldg-lod2-2025",
			want: Spec{Area: Area{Kind: AreaCity, Code: "13101"}, Type: "bldg", LOD: 2, Year: 2025},
		},
		{
			name: "maxlod",
			in:   "all-bldg-maxlod2-2025",
			want: Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 2, LODMode: LODMax, Year: 2025},
		},
		{
			name: "maxlod with notexture",
			in:   "all-bldg-maxlod3-notexture-2025",
			want: Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 3, LODMode: LODMax, Texture: TextureNone, Year: 2025},
		},
		{
			name: "latest year",
			in:   "all-bldg-lod2-latest",
			want: Spec{Area: Area{Kind: AreaAll}, Type: "bldg", LOD: 2, YearMode: YearLatest},
		},
		{
			name: "latest year with notexture",
			in:   "13-bldg-maxlod2-notexture-latest",
			want: Spec{Area: Area{Kind: AreaPref, Code: "13"}, Type: "bldg", LOD: 2, LODMode: LODMax, Texture: TextureNone, YearMode: YearLatest},
		},
		{
			name:    "missing year",
			in:      "all-bldg-lod2",
			wantErr: true,
		},
		{
			name:    "non-numeric area",
			in:      "tokyo-bldg-lod2-2025",
			wantErr: true,
		},
		{
			name:    "wrong area length",
			in:      "131-bldg-lod2-2025",
			wantErr: true,
		},
		{
			name:    "no lod prefix",
			in:      "all-bldg-2-2025",
			wantErr: true,
		},
		{
			name:    "invalid year",
			in:      "all-bldg-lod2-abcd",
			wantErr: true,
		},
		{
			name:    "trailing junk",
			in:      "all-bldg-lod2-2025-extra",
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
