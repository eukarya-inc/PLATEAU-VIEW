package lodstat

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/geo/jisx0410"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestNewLodstatContext(t *testing.T) {
	ctx := newLodstatContext()

	assert.NotNil(t, ctx)
	assert.NotNil(t, ctx.Codes)
	assert.NotNil(t, ctx.LodStat)
	assert.NotNil(t, ctx.Maxlod)
	assert.NotNil(t, ctx.Lod0Count)
	assert.NotNil(t, ctx.Lod1Count)
	assert.NotNil(t, ctx.Lod2Count)
	assert.NotNil(t, ctx.Lod3Count)
	assert.NotNil(t, ctx.Lod4Count)
	assert.Empty(t, ctx.Codes)
	assert.Empty(t, ctx.LodStat)
	assert.Empty(t, ctx.Maxlod)
}

func TestLodstatContext_Collect(t *testing.T) {
	tests := []struct {
		name         string
		level        int
		featureType  string
		cityFile     DatasetFilesResponse
		wantFeatures map[string]bool
		wantLodStat  map[string]int
		wantMaxlod   map[string]int
		wantCounts   map[string]map[string]int
	}{
		{
			name:        "collect all feature types with valid mesh codes",
			level:       3,
			featureType: "all",
			cityFile: DatasetFilesResponse{
				"bldg": []DatasetFilesResponseItem{
					{
						Code:   "53394547",
						MaxLod: 2,
						LOD0:   lo.ToPtr(10),
						LOD1:   lo.ToPtr(20),
						LOD2:   lo.ToPtr(30),
					},
					{
						Code:   "53394548",
						MaxLod: 1,
						LOD0:   lo.ToPtr(5),
						LOD1:   lo.ToPtr(15),
					},
				},
				"tran": []DatasetFilesResponseItem{
					{
						Code:   "53394549",
						MaxLod: 0,
						LOD0:   lo.ToPtr(8),
					},
				},
			},
			wantFeatures: map[string]bool{
				"53394547": true,
				"53394548": true,
				"53394549": true,
			},
			wantLodStat: map[string]int{
				"53394547": 0b00111, // LOD 0,1,2
				"53394548": 0b00011, // LOD 0,1
				"53394549": 0b00001, // LOD 0
			},
			wantMaxlod: map[string]int{
				"53394547": 2,
				"53394548": 1,
				"53394549": 0,
			},
			wantCounts: map[string]map[string]int{
				"lod0": {
					"53394547": 10,
					"53394548": 5,
					"53394549": 8,
				},
				"lod1": {
					"53394547": 20,
					"53394548": 15,
				},
				"lod2": {
					"53394547": 30,
				},
			},
		},
		{
			name:        "collect specific feature type only",
			level:       3,
			featureType: "bldg",
			cityFile: DatasetFilesResponse{
				"bldg": []DatasetFilesResponseItem{
					{
						Code:   "53394547",
						MaxLod: 3,
						LOD0:   lo.ToPtr(10),
						LOD1:   lo.ToPtr(20),
						LOD2:   lo.ToPtr(30),
						LOD3:   lo.ToPtr(40),
					},
				},
				"tran": []DatasetFilesResponseItem{
					{
						Code:   "53394549",
						MaxLod: 0,
						LOD0:   lo.ToPtr(8),
					},
				},
			},
			wantFeatures: map[string]bool{
				"53394547": true,
			},
			wantLodStat: map[string]int{
				"53394547": 0b01111, // LOD 0,1,2,3
			},
			wantMaxlod: map[string]int{
				"53394547": 3,
			},
			wantCounts: map[string]map[string]int{
				"lod0": {
					"53394547": 10,
				},
				"lod1": {
					"53394547": 20,
				},
				"lod2": {
					"53394547": 30,
				},
				"lod3": {
					"53394547": 40,
				},
			},
		},
		{
			name:        "handle LOD4",
			level:       3,
			featureType: "all",
			cityFile: DatasetFilesResponse{
				"bldg": []DatasetFilesResponseItem{
					{
						Code:   "53394547",
						MaxLod: 4,
						LOD0:   lo.ToPtr(10),
						LOD1:   lo.ToPtr(20),
						LOD2:   lo.ToPtr(30),
						LOD3:   lo.ToPtr(40),
						LOD4:   lo.ToPtr(50),
					},
				},
			},
			wantFeatures: map[string]bool{
				"53394547": true,
			},
			wantLodStat: map[string]int{
				"53394547": 0b11111, // LOD 0,1,2,3,4
			},
			wantMaxlod: map[string]int{
				"53394547": 4,
			},
			wantCounts: map[string]map[string]int{
				"lod0": {"53394547": 10},
				"lod1": {"53394547": 20},
				"lod2": {"53394547": 30},
				"lod3": {"53394547": 40},
				"lod4": {"53394547": 50},
			},
		},
		{
			name:        "accumulate counts for same mesh code",
			level:       3,
			featureType: "all",
			cityFile: DatasetFilesResponse{
				"bldg": []DatasetFilesResponseItem{
					{
						Code:   "53394547",
						MaxLod: 2,
						LOD0:   lo.ToPtr(10),
						LOD1:   lo.ToPtr(20),
						LOD2:   lo.ToPtr(30),
					},
					{
						Code:   "53394547", // Same code
						MaxLod: 3,          // Higher maxLod
						LOD0:   lo.ToPtr(5),
						LOD1:   lo.ToPtr(10),
						LOD2:   lo.ToPtr(15),
						LOD3:   lo.ToPtr(25),
					},
				},
			},
			wantFeatures: map[string]bool{
				"53394547": true,
			},
			wantLodStat: map[string]int{
				"53394547": 0b01111, // Combined: LOD 0,1,2,3
			},
			wantMaxlod: map[string]int{
				"53394547": 3, // Maximum of 2 and 3
			},
			wantCounts: map[string]map[string]int{
				"lod0": {"53394547": 15}, // 10 + 5
				"lod1": {"53394547": 30}, // 20 + 10
				"lod2": {"53394547": 45}, // 30 + 15
				"lod3": {"53394547": 25}, // 0 + 25
			},
		},
		{
			name:        "skip invalid mesh codes",
			level:       3,
			featureType: "all",
			cityFile: DatasetFilesResponse{
				"bldg": []DatasetFilesResponseItem{
					{
						Code:   "invalid",
						MaxLod: 2,
						LOD0:   lo.ToPtr(10),
					},
					{
						Code:   "53394547",
						MaxLod: 1,
						LOD0:   lo.ToPtr(5),
					},
				},
			},
			wantFeatures: map[string]bool{
				"53394547": true,
			},
			wantLodStat: map[string]int{
				"53394547": 0b00011,
			},
			wantMaxlod: map[string]int{
				"53394547": 1,
			},
			wantCounts: map[string]map[string]int{
				"lod0": {"53394547": 5},
			},
		},
		{
			name:        "skip mesh codes with wrong level",
			level:       2,
			featureType: "all",
			cityFile: DatasetFilesResponse{
				"bldg": []DatasetFilesResponseItem{
					{
						Code:   "53394547", // Level 3 mesh
						MaxLod: 2,
						LOD0:   lo.ToPtr(10),
					},
					{
						Code:   "533945", // Level 2 mesh
						MaxLod: 1,
						LOD0:   lo.ToPtr(5),
					},
				},
			},
			wantFeatures: map[string]bool{
				"533945": true,
			},
			wantLodStat: map[string]int{
				"533945": 0b00011,
			},
			wantMaxlod: map[string]int{
				"533945": 1,
			},
			wantCounts: map[string]map[string]int{
				"lod0": {"533945": 5},
			},
		},
		{
			name:        "handle nil LOD counts",
			level:       3,
			featureType: "all",
			cityFile: DatasetFilesResponse{
				"bldg": []DatasetFilesResponseItem{
					{
						Code:   "53394547",
						MaxLod: 2,
						LOD0:   nil,
						LOD1:   lo.ToPtr(20),
						LOD2:   nil,
					},
				},
			},
			wantFeatures: map[string]bool{
				"53394547": true,
			},
			wantLodStat: map[string]int{
				"53394547": 0b00111,
			},
			wantMaxlod: map[string]int{
				"53394547": 2,
			},
			wantCounts: map[string]map[string]int{
				"lod1": {"53394547": 20},
			},
		},
		{
			name:        "handle zero LOD counts",
			level:       3,
			featureType: "all",
			cityFile: DatasetFilesResponse{
				"bldg": []DatasetFilesResponseItem{
					{
						Code:   "53394547",
						MaxLod: 2,
						LOD0:   lo.ToPtr(0),
						LOD1:   lo.ToPtr(20),
						LOD2:   lo.ToPtr(0),
					},
				},
			},
			wantFeatures: map[string]bool{
				"53394547": true,
			},
			wantLodStat: map[string]int{
				"53394547": 0b00111,
			},
			wantMaxlod: map[string]int{
				"53394547": 2,
			},
			wantCounts: map[string]map[string]int{
				"lod1": {"53394547": 20},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx := newLodstatContext()
			ctx.Collect(tt.level, tt.featureType, tt.cityFile)

			// Check features
			assert.Equal(t, len(tt.wantFeatures), len(ctx.Codes))
			for code := range tt.wantFeatures {
				_, ok := ctx.Codes[code]
				assert.True(t, ok, "Expected feature %s to be present", code)
			}

			// Check LodStat
			assert.Equal(t, tt.wantLodStat, ctx.LodStat)

			// Check Maxlod
			assert.Equal(t, tt.wantMaxlod, ctx.Maxlod)

			// Check LOD counts
			if counts, ok := tt.wantCounts["lod0"]; ok {
				for code, count := range counts {
					assert.Equal(t, count, ctx.Lod0Count[code], "LOD0 count mismatch for %s", code)
				}
			}
			if counts, ok := tt.wantCounts["lod1"]; ok {
				for code, count := range counts {
					assert.Equal(t, count, ctx.Lod1Count[code], "LOD1 count mismatch for %s", code)
				}
			}
			if counts, ok := tt.wantCounts["lod2"]; ok {
				for code, count := range counts {
					assert.Equal(t, count, ctx.Lod2Count[code], "LOD2 count mismatch for %s", code)
				}
			}
			if counts, ok := tt.wantCounts["lod3"]; ok {
				for code, count := range counts {
					assert.Equal(t, count, ctx.Lod3Count[code], "LOD3 count mismatch for %s", code)
				}
			}
			if counts, ok := tt.wantCounts["lod4"]; ok {
				for code, count := range counts {
					assert.Equal(t, count, ctx.Lod4Count[code], "LOD4 count mismatch for %s", code)
				}
			}
		})
	}
}

func TestLodstatContext_CollectAll(t *testing.T) {
	cityFiles := []DatasetFilesResponse{
		{
			"bldg": []DatasetFilesResponseItem{
				{
					Code:   "53394547",
					MaxLod: 2,
					LOD0:   lo.ToPtr(10),
					LOD1:   lo.ToPtr(20),
					LOD2:   lo.ToPtr(30),
				},
			},
		},
		{
			"bldg": []DatasetFilesResponseItem{
				{
					Code:   "53394548",
					MaxLod: 1,
					LOD0:   lo.ToPtr(5),
					LOD1:   lo.ToPtr(15),
				},
			},
			"tran": []DatasetFilesResponseItem{
				{
					Code:   "53394549",
					MaxLod: 0,
					LOD0:   lo.ToPtr(8),
				},
			},
		},
	}

	ctx := newLodstatContext()
	ctx.CollectAll(3, "all", cityFiles)

	// Should have collected from all files
	assert.Len(t, ctx.Codes, 3)
	assert.Contains(t, ctx.Codes, "53394547")
	assert.Contains(t, ctx.Codes, "53394548")
	assert.Contains(t, ctx.Codes, "53394549")

	// Check LOD stats
	assert.Equal(t, 0b00111, ctx.LodStat["53394547"])
	assert.Equal(t, 0b00011, ctx.LodStat["53394548"])
	assert.Equal(t, 0b00001, ctx.LodStat["53394549"])

	// Check counts
	assert.Equal(t, 10, ctx.Lod0Count["53394547"])
	assert.Equal(t, 5, ctx.Lod0Count["53394548"])
	assert.Equal(t, 8, ctx.Lod0Count["53394549"])
}

func TestLodstatContext_Properties(t *testing.T) {
	tests := []struct {
		name        string
		code        string
		featureType string
		setup       func(*lodstatContext)
		want        map[string]any
	}{
		{
			name:        "properties for all feature types",
			code:        "53394547",
			featureType: "all",
			setup: func(ctx *lodstatContext) {
				mesh, _ := jisx0410.Parse("53394547")
				ctx.Codes["53394547"] = mesh
				ctx.LodStat["53394547"] = 0b00111 // LOD 0,1,2
				ctx.Maxlod["53394547"] = 2
				ctx.Lod0Count["53394547"] = 10
				ctx.Lod1Count["53394547"] = 20
				ctx.Lod2Count["53394547"] = 30
			},
			want: map[string]any{
				"meshCode":  "53394547",
				"level":     3,
				"fileSize":  int64(0),
				"features":  0,
				"maxLod":    2,
				"lod0":      true,
				"lod1":      true,
				"lod2":      true,
				"lod3":      false,
				"lod4":      false,
				"lod0Count": 10,
				"lod1Count": 20,
				"lod2Count": 30,
				"lod3Count": 0,
				"lod4Count": 0,
			},
		},
		{
			name:        "properties for specific feature type",
			code:        "53394547",
			featureType: "bldg",
			setup: func(ctx *lodstatContext) {
				mesh, _ := jisx0410.Parse("53394547")
				ctx.Codes["53394547"] = mesh
				ctx.LodStat["53394547"] = 0b11111 // All LODs
				ctx.Maxlod["53394547"] = 4
				ctx.Lod0Count["53394547"] = 10
				ctx.Lod1Count["53394547"] = 20
				ctx.Lod2Count["53394547"] = 30
				ctx.Lod3Count["53394547"] = 40
				ctx.Lod4Count["53394547"] = 50
			},
			want: map[string]any{
				"featureType": "bldg",
				"meshCode":    "53394547",
				"level":       3,
				"fileSize":    int64(0),
				"features":    0,
				"maxLod":      4,
				"lod0":        true,
				"lod1":        true,
				"lod2":        true,
				"lod3":        true,
				"lod4":        true,
				"lod0Count":   10,
				"lod1Count":   20,
				"lod2Count":   30,
				"lod3Count":   40,
				"lod4Count":   50,
			},
		},
		{
			name:        "properties for code without LOD stats",
			code:        "53394547",
			featureType: "all",
			setup: func(ctx *lodstatContext) {
				mesh, _ := jisx0410.Parse("53394547")
				ctx.Codes["53394547"] = mesh
				ctx.Maxlod["53394547"] = 1
			},
			want: map[string]any{
				"meshCode":  "53394547",
				"level":     3,
				"fileSize":  int64(0),
				"features":  0,
				"maxLod":    1,
				"lod0Count": 0,
				"lod1Count": 0,
				"lod2Count": 0,
				"lod3Count": 0,
				"lod4Count": 0,
			},
		},
		{
			name:        "properties for code not in context",
			code:        "unknown",
			featureType: "all",
			setup:       func(ctx *lodstatContext) {},
			want:        nil,
		},
		{
			name:        "properties with partial LOD coverage",
			code:        "53394547",
			featureType: "all",
			setup: func(ctx *lodstatContext) {
				mesh, _ := jisx0410.Parse("53394547")
				ctx.Codes["53394547"] = mesh
				ctx.LodStat["53394547"] = 0b00101 // LOD 0,2 only
				ctx.Maxlod["53394547"] = 2
				ctx.Lod0Count["53394547"] = 10
				ctx.Lod2Count["53394547"] = 30
			},
			want: map[string]any{
				"meshCode":  "53394547",
				"level":     3,
				"fileSize":  int64(0),
				"features":  0,
				"maxLod":    2,
				"lod0":      true,
				"lod1":      false,
				"lod2":      true,
				"lod3":      false,
				"lod4":      false,
				"lod0Count": 10,
				"lod1Count": 0,
				"lod2Count": 30,
				"lod3Count": 0,
				"lod4Count": 0,
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ctx := newLodstatContext()
			tt.setup(ctx)
			got := ctx.Properties(tt.code, tt.featureType)
			assert.Equal(t, tt.want, got)
		})
	}
}

func TestLodstatContext_EdgeCases(t *testing.T) {
	t.Run("multiple updates to same mesh code keep highest maxlod", func(t *testing.T) {
		ctx := newLodstatContext()
		cityFile1 := DatasetFilesResponse{
			"bldg": []DatasetFilesResponseItem{
				{Code: "53394547", MaxLod: 1, LOD0: lo.ToPtr(10)},
			},
		}
		cityFile2 := DatasetFilesResponse{
			"bldg": []DatasetFilesResponseItem{
				{Code: "53394547", MaxLod: 3, LOD0: lo.ToPtr(20)},
			},
		}
		cityFile3 := DatasetFilesResponse{
			"bldg": []DatasetFilesResponseItem{
				{Code: "53394547", MaxLod: 2, LOD0: lo.ToPtr(30)},
			},
		}

		ctx.Collect(3, "all", cityFile1)
		ctx.Collect(3, "all", cityFile2)
		ctx.Collect(3, "all", cityFile3)

		assert.Equal(t, 3, ctx.Maxlod["53394547"])
		assert.Equal(t, 60, ctx.Lod0Count["53394547"])    // 10+20+30
		assert.Equal(t, 0b01111, ctx.LodStat["53394547"]) // Combined all LODs up to 3
	})

	t.Run("empty city files", func(t *testing.T) {
		ctx := newLodstatContext()
		cityFiles := []DatasetFilesResponse{}
		ctx.CollectAll(3, "all", cityFiles)

		assert.Empty(t, ctx.Codes)
		assert.Empty(t, ctx.LodStat)
		assert.Empty(t, ctx.Maxlod)
	})

	t.Run("city file with empty feature type map", func(t *testing.T) {
		ctx := newLodstatContext()
		cityFile := DatasetFilesResponse{}
		ctx.Collect(3, "all", cityFile)

		assert.Empty(t, ctx.Codes)
		assert.Empty(t, ctx.LodStat)
		assert.Empty(t, ctx.Maxlod)
	})

	t.Run("feature type with empty items list", func(t *testing.T) {
		ctx := newLodstatContext()
		cityFile := DatasetFilesResponse{
			"bldg": []DatasetFilesResponseItem{},
		}
		ctx.Collect(3, "all", cityFile)

		assert.Empty(t, ctx.Codes)
		assert.Empty(t, ctx.LodStat)
		assert.Empty(t, ctx.Maxlod)
	})
}
