package geocoding

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestBuildAreas(t *testing.T) {
	tests := []struct {
		name         string
		code         string
		includeRadii bool
		wantLen      int
		wantFirst    *Area
		wantLast     *Area
		wantErr      bool
	}{
		{
			name:         "simple municipality (千代田区)",
			code:         "13101",
			includeRadii: false,
			wantLen:      2,
			wantFirst: &Area{
				Type:   "municipality",
				Code:   "13101",
				Name:   "千代田区",
				Radius: 0,
			},
			wantLast: &Area{
				Type:   "prefecture",
				Code:   "13",
				Name:   "東京都",
				Radius: 0,
			},
		},
		{
			name:         "simple municipality with radii",
			code:         "13101",
			includeRadii: true,
			wantLen:      2,
			wantFirst: &Area{
				Type:   "municipality",
				Code:   "13101",
				Name:   "千代田区",
				Radius: 3164.256,
			},
			wantLast: &Area{
				Type:   "prefecture",
				Code:   "13",
				Name:   "東京都",
				Radius: 1061800.88,
			},
		},
		{
			name:         "ward with parent city (さいたま市西区)",
			code:         "11101",
			includeRadii: false,
			wantLen:      3,
			wantFirst: &Area{
				Type:   "municipality",
				Code:   "11101",
				Name:   "西区",
				Radius: 0,
			},
			wantLast: &Area{
				Type:   "prefecture",
				Code:   "11",
				Name:   "埼玉県",
				Radius: 0,
			},
		},
		{
			name:         "empty code",
			code:         "",
			includeRadii: false,
			wantLen:      0,
			wantErr:      false,
		},
		{
			name:         "invalid code",
			code:         "99999",
			includeRadii: false,
			wantLen:      0,
			wantErr:      false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			areas, err := BuildAreas(tt.code, tt.includeRadii)

			if tt.wantErr {
				assert.Error(t, err)
				return
			}

			assert.NoError(t, err)

			if tt.wantLen == 0 {
				assert.Nil(t, areas)
				return
			}

			assert.Len(t, areas, tt.wantLen)

			if tt.wantFirst != nil {
				assert.Equal(t, tt.wantFirst.Type, areas[0].Type)
				assert.Equal(t, tt.wantFirst.Code, areas[0].Code)
				assert.Equal(t, tt.wantFirst.Name, areas[0].Name)
				assert.Equal(t, tt.wantFirst.Radius, areas[0].Radius)
			}

			if tt.wantLast != nil {
				last := areas[len(areas)-1]
				assert.Equal(t, tt.wantLast.Type, last.Type)
				assert.Equal(t, tt.wantLast.Code, last.Code)
				assert.Equal(t, tt.wantLast.Name, last.Name)
				assert.Equal(t, tt.wantLast.Radius, last.Radius)
			}
		})
	}
}

func TestBuildAreas_ParentCity(t *testing.T) {
	// Test that さいたま市西区 includes さいたま市 as parent
	areas, err := BuildAreas("11101", false)
	assert.NoError(t, err)
	assert.Len(t, areas, 3)

	// First: 西区
	assert.Equal(t, "municipality", areas[0].Type)
	assert.Equal(t, "11101", areas[0].Code)
	assert.Equal(t, "西区", areas[0].Name)

	// Second: さいたま市 (parent city)
	assert.Equal(t, "municipality", areas[1].Type)
	assert.Equal(t, "11100", areas[1].Code)
	assert.Equal(t, "さいたま市", areas[1].Name)

	// Third: 埼玉県
	assert.Equal(t, "prefecture", areas[2].Type)
	assert.Equal(t, "11", areas[2].Code)
	assert.Equal(t, "埼玉県", areas[2].Name)
}
