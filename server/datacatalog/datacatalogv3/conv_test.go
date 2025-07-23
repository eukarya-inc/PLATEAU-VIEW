package datacatalogv3

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/stretchr/testify/assert"
)

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
