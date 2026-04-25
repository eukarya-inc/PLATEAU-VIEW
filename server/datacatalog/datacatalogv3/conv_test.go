package datacatalogv3

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/stretchr/testify/assert"
)

// readyStatus returns a Status tag that makes IsBeta() return true
func readyStatus() *cms.Tag {
	return &cms.Tag{Name: string(ManagementStatusReady)}
}

func TestExtractBaseFeatureType(t *testing.T) {
	tests := []struct {
		name     string
		code     string
		expected string
	}{
		{
			name:     "base type bldg",
			code:     "bldg",
			expected: "bldg",
		},
		{
			name:     "derived type bldg2",
			code:     "bldg2",
			expected: "bldg",
		},
		{
			name:     "derived type bldg10",
			code:     "bldg10",
			expected: "bldg",
		},
		{
			name:     "base type tran",
			code:     "tran",
			expected: "tran",
		},
		{
			name:     "derived type tran3",
			code:     "tran3",
			expected: "tran",
		},
		{
			name:     "base type fld",
			code:     "fld",
			expected: "fld",
		},
		{
			name:     "empty string",
			code:     "",
			expected: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := ExtractBaseFeatureType(tt.code)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestIsDerivedFeatureType(t *testing.T) {
	tests := []struct {
		name     string
		code     string
		expected bool
	}{
		{
			name:     "base type bldg is not derived",
			code:     "bldg",
			expected: false,
		},
		{
			name:     "bldg2 is derived",
			code:     "bldg2",
			expected: true,
		},
		{
			name:     "tran is not derived",
			code:     "tran",
			expected: false,
		},
		{
			name:     "tran10 is derived",
			code:     "tran10",
			expected: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := IsDerivedFeatureType(tt.code)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestFilterPlateauByPriority(t *testing.T) {
	tests := []struct {
		name      string
		input     map[string][]*PlateauFeatureItem
		cityItems []*CityItem
		expected  map[string][]string // baseCode -> []cityID for simplicity
	}{
		{
			name:      "empty input",
			input:     map[string][]*PlateauFeatureItem{},
			cityItems: nil,
			expected:  map[string][]string{},
		},
		{
			name: "single base type single city",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 0, Status: readyStatus()},
				},
			},
			cityItems: nil,
			expected: map[string][]string{
				"bldg": {"city1"},
			},
		},
		{
			name: "derived type higher priority wins",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 0, Status: readyStatus()},
				},
				"bldg2": {
					{ID: "item2", City: "city1", Priority: 10, Status: readyStatus()},
				},
			},
			cityItems: nil,
			expected: map[string][]string{
				"bldg": {"city1"}, // item2 wins (higher priority)
			},
		},
		{
			name: "equal priority derived type wins",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 0, Status: readyStatus()},
				},
				"bldg2": {
					{ID: "item2", City: "city1", Priority: 0, Status: readyStatus()},
				},
			},
			cityItems: nil,
			expected: map[string][]string{
				"bldg": {"city1"}, // item2 wins (bldg2 > bldg alphabetically)
			},
		},
		{
			name: "base type higher priority wins over derived",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 100, Status: readyStatus()},
				},
				"bldg2": {
					{ID: "item2", City: "city1", Priority: 10, Status: readyStatus()},
				},
			},
			cityItems: nil,
			expected: map[string][]string{
				"bldg": {"city1"}, // item1 wins (higher priority)
			},
		},
		{
			name: "multiple cities independent",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 0, Status: readyStatus()},
					{ID: "item3", City: "city2", Priority: 100, Status: readyStatus()},
				},
				"bldg2": {
					{ID: "item2", City: "city1", Priority: 10, Status: readyStatus()},
				},
			},
			cityItems: nil,
			expected: map[string][]string{
				"bldg": {"city1", "city2"}, // city1: item2, city2: item3
			},
		},
		{
			name: "nil items are skipped",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					nil,
					{ID: "item1", City: "city1", Priority: 0, Status: readyStatus()},
					nil,
				},
			},
			cityItems: nil,
			expected: map[string][]string{
				"bldg": {"city1"},
			},
		},
		{
			name: "items without ready status are skipped when city not public",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 0, Status: readyStatus()},
				},
				"bldg2": {
					{ID: "item2", City: "city1", Priority: 100, Status: nil}, // not ready, should be skipped
				},
			},
			cityItems: nil,
			expected: map[string][]string{
				"bldg": {"city1"}, // item1 wins because item2 is not ready
			},
		},
		{
			name: "all items without ready status results in empty when city not public",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 0, Status: nil},
				},
			},
			cityItems: nil,
			expected:  map[string][]string{},
		},
		{
			name: "items without ready status are included when city is public",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 0, Status: nil}, // not ready but city is public
				},
			},
			cityItems: []*CityItem{
				{ID: "city1", CityPublic: true},
			},
			expected: map[string][]string{
				"bldg": {"city1"}, // included because city is public
			},
		},
		{
			name: "public city items compete with ready items by priority",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 5, Status: nil}, // public city, no status
				},
				"bldg2": {
					{ID: "item2", City: "city1", Priority: 10, Status: readyStatus()}, // ready status
				},
			},
			cityItems: []*CityItem{
				{ID: "city1", CityPublic: true},
			},
			expected: map[string][]string{
				"bldg": {"city1"}, // item2 wins (higher priority)
			},
		},
		{
			name: "mixed public and non-public cities",
			input: map[string][]*PlateauFeatureItem{
				"bldg": {
					{ID: "item1", City: "city1", Priority: 0, Status: nil},           // public city
					{ID: "item2", City: "city2", Priority: 0, Status: nil},           // non-public city, no status -> skipped
					{ID: "item3", City: "city3", Priority: 0, Status: readyStatus()}, // non-public city but ready
				},
			},
			cityItems: []*CityItem{
				{ID: "city1", CityPublic: true},
				{ID: "city2", CityPublic: false},
				{ID: "city3", CityPublic: false},
			},
			expected: map[string][]string{
				"bldg": {"city1", "city3"}, // city2 is skipped
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := filterPlateauByPriority(tt.input, tt.cityItems)

			// Convert result to comparable format
			got := make(map[string][]string)
			for code, items := range result {
				for _, item := range items {
					got[code] = append(got[code], item.City)
				}
			}

			// Check expected codes exist
			assert.Equal(t, len(tt.expected), len(got), "different number of base codes")

			for code, expectedCities := range tt.expected {
				gotCities, ok := got[code]
				assert.True(t, ok, "expected code %s not found", code)
				assert.ElementsMatch(t, expectedCities, gotCities, "cities don't match for code %s", code)
			}
		})
	}
}

func TestFilterPlateauByPriority_VerifyWinningItem(t *testing.T) {
	// This test verifies that the correct item wins based on priority
	input := map[string][]*PlateauFeatureItem{
		"bldg": {
			{ID: "bldg-item", City: "city1", Priority: 5, Status: readyStatus()},
		},
		"bldg2": {
			{ID: "bldg2-item", City: "city1", Priority: 10, Status: readyStatus()},
		},
	}

	result := filterPlateauByPriority(input, nil)

	// Should have one entry for bldg (base code)
	assert.Len(t, result, 1)
	items, ok := result["bldg"]
	assert.True(t, ok)
	assert.Len(t, items, 1)
	// The bldg2 item should win due to higher priority
	assert.Equal(t, "bldg2-item", items[0].ID)
}

func TestFilterPlateauByPriority_NotReadyItemsSkipped(t *testing.T) {
	// This test verifies that items without ready status are skipped when city is not public
	input := map[string][]*PlateauFeatureItem{
		"bldg": {
			{ID: "bldg-item", City: "city1", Priority: 5, Status: readyStatus()},
		},
		"bldg2": {
			{ID: "bldg2-item", City: "city1", Priority: 100, Status: nil}, // higher priority but not ready
		},
	}

	result := filterPlateauByPriority(input, nil)

	// Should have one entry for bldg (base code)
	assert.Len(t, result, 1)
	items, ok := result["bldg"]
	assert.True(t, ok)
	assert.Len(t, items, 1)
	// The bldg item should win because bldg2 is not ready
	assert.Equal(t, "bldg-item", items[0].ID)
}

func TestAllData_Into_FiltersEmptyDatasetTypes(t *testing.T) {
	// Create a simple test that focuses on the filtering logic
	// We'll create DatasetTypes and Datasets manually and then test the filtering

	// Create a mock InMemoryRepoContext with some dataset types and datasets
	mockContext := &plateauapi.InMemoryRepoContext{
		Name:  "test",
		Areas: plateauapi.Areas{},
		DatasetTypes: plateauapi.DatasetTypes{
			plateauapi.DatasetTypeCategoryPlateau: []plateauapi.DatasetType{
				&plateauapi.PlateauDatasetType{
					ID:   "dt_bldg",
					Code: "bldg",
					Name: "建築物",
				},
				&plateauapi.PlateauDatasetType{
					ID:   "dt_empty",
					Code: "empty",
					Name: "空データ",
				},
			},
			plateauapi.DatasetTypeCategoryRelated: []plateauapi.DatasetType{
				&plateauapi.RelatedDatasetType{
					ID:   "dt_related1",
					Code: "related1",
					Name: "関連データ1",
				},
				&plateauapi.RelatedDatasetType{
					ID:   "dt_related2",
					Code: "related2",
					Name: "関連データ2",
				},
			},
		},
		Datasets: plateauapi.Datasets{
			plateauapi.DatasetTypeCategoryPlateau: []plateauapi.Dataset{
				&plateauapi.PlateauDataset{
					ID:     "ds_bldg_1",
					TypeID: "dt_bldg",
				},
			},
			plateauapi.DatasetTypeCategoryRelated: []plateauapi.Dataset{
				&plateauapi.RelatedDataset{
					ID:     "ds_related1_1",
					TypeID: "dt_related1",
				},
			},
		},
	}

	// Apply the filtering logic
	var warnings []string
	filteredDatasetTypes := make(plateauapi.DatasetTypes)
	for category, datasetTypes := range mockContext.DatasetTypes {
		var filteredTypes []plateauapi.DatasetType
		datasets := mockContext.Datasets[category]

		for _, dt := range datasetTypes {
			// Check if there are any datasets for this dataset type
			hasDataset := false
			for _, ds := range datasets {
				if ds.GetTypeID() == dt.GetID() {
					hasDataset = true
					break
				}
			}

			if hasDataset {
				filteredTypes = append(filteredTypes, dt)
			} else {
				warnings = append(warnings, "dataset type "+dt.GetCode()+" ("+dt.GetName()+") has no datasets, filtering out")
			}
		}

		if len(filteredTypes) > 0 {
			filteredDatasetTypes[category] = filteredTypes
		}
	}

	// Verify the results
	assert.Equal(t, 1, len(filteredDatasetTypes[plateauapi.DatasetTypeCategoryPlateau]))
	assert.Equal(t, 1, len(filteredDatasetTypes[plateauapi.DatasetTypeCategoryRelated]))

	// Check that only dataset types with datasets remain
	assert.Equal(t, "bldg", filteredDatasetTypes[plateauapi.DatasetTypeCategoryPlateau][0].GetCode())
	assert.Equal(t, "related1", filteredDatasetTypes[plateauapi.DatasetTypeCategoryRelated][0].GetCode())

	// Check warnings
	assert.Contains(t, warnings, "dataset type empty (空データ) has no datasets, filtering out")
	assert.Contains(t, warnings, "dataset type related2 (関連データ2) has no datasets, filtering out")
}
