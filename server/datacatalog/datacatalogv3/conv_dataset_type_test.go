package datacatalogv3

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/stretchr/testify/assert"
)

func TestFeatureTypes_ToDatasetTypes(t *testing.T) {
	tests := []struct {
		name  string
		ft    FeatureTypes
		specs []plateauapi.PlateauSpec
		want  plateauapi.DatasetTypes
	}{
		{
			name: "test",
			ft: FeatureTypes{
				Plateau: []FeatureType{
					{
						Code:         "bldg",
						Name:         "建築物",
						Flood:        true,
						Order:        1,
						MinSpecMajor: 1,
					},
					{
						Code:         "tran",
						Name:         "交通",
						MinSpecMajor: 2,
					},
				},
				Generic: []FeatureType{
					{
						Code:  "usecase",
						Name:  "ユースケース",
						Order: 2,
					},
				},
			},
			specs: []plateauapi.PlateauSpec{
				{
					ID:           "id",
					MajorVersion: 1,
					Year:         2021,
				},
				{
					ID:           "id2",
					MajorVersion: 2,
					Year:         2023,
				},
			},
			want: plateauapi.DatasetTypes{
				plateauapi.DatasetTypeCategoryPlateau: []plateauapi.DatasetType{
					&plateauapi.PlateauDatasetType{
						Category:      plateauapi.DatasetTypeCategoryPlateau,
						ID:            "dt_bldg_1",
						Name:          "建築物",
						Code:          "bldg",
						Flood:         true,
						PlateauSpecID: "id",
						Year:          2021,
						Order:         1,
					},
					&plateauapi.PlateauDatasetType{
						Category:      plateauapi.DatasetTypeCategoryPlateau,
						ID:            "dt_bldg_2",
						Name:          "建築物",
						Code:          "bldg",
						Flood:         true,
						PlateauSpecID: "id2",
						Year:          2023,
						Order:         1,
					},
					&plateauapi.PlateauDatasetType{
						Category:      plateauapi.DatasetTypeCategoryPlateau,
						ID:            "dt_tran_2",
						Name:          "交通",
						Code:          "tran",
						Flood:         false,
						PlateauSpecID: "id2",
						Year:          2023,
						Order:         0,
					},
				},
				plateauapi.DatasetTypeCategoryRelated: []plateauapi.DatasetType{},
				plateauapi.DatasetTypeCategoryGeneric: []plateauapi.DatasetType{
					&plateauapi.GenericDatasetType{
						Category: plateauapi.DatasetTypeCategoryGeneric,
						ID:       "dt_usecase",
						Name:     "ユースケース",
						Code:     "usecase",
						Order:    2,
					},
				},
			},
		},
		{
			name: "derived types are skipped",
			ft: FeatureTypes{
				Plateau: []FeatureType{
					{
						Code:         "bldg",
						Name:         "建築物",
						MinSpecMajor: 1,
					},
					{
						Code:         "bldg2", // derived type should be skipped
						Name:         "建築物2",
						MinSpecMajor: 1,
					},
					{
						Code:         "tran",
						Name:         "交通",
						MinSpecMajor: 1,
					},
				},
			},
			specs: []plateauapi.PlateauSpec{
				{
					ID:           "id",
					MajorVersion: 1,
					Year:         2021,
				},
			},
			want: plateauapi.DatasetTypes{
				plateauapi.DatasetTypeCategoryPlateau: []plateauapi.DatasetType{
					&plateauapi.PlateauDatasetType{
						Category:      plateauapi.DatasetTypeCategoryPlateau,
						ID:            "dt_bldg_1",
						Name:          "建築物",
						Code:          "bldg",
						PlateauSpecID: "id",
						Year:          2021,
					},
					// bldg2 is not included (derived type)
					&plateauapi.PlateauDatasetType{
						Category:      plateauapi.DatasetTypeCategoryPlateau,
						ID:            "dt_tran_1",
						Name:          "交通",
						Code:          "tran",
						PlateauSpecID: "id",
						Year:          2021,
					},
				},
				plateauapi.DatasetTypeCategoryRelated: []plateauapi.DatasetType{},
				plateauapi.DatasetTypeCategoryGeneric: []plateauapi.DatasetType{},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tt.ft.ToDatasetTypes(tt.specs)
			assert.Equal(t, tt.want, got)
		})
	}
}
