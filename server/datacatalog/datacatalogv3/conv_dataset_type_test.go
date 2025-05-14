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
						Code:         "code",
						Name:         "name",
						Flood:        true,
						Order:        1,
						MinSpecMajor: 1,
					},
					{
						Code:         "code2",
						Name:         "name2",
						MinSpecMajor: 2,
					},
				},
				Generic: []FeatureType{
					{
						Code:  "code3",
						Name:  "name3",
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
						ID:            "dt_code_1",
						Name:          "name",
						Code:          "code",
						Flood:         true,
						PlateauSpecID: "id",
						Year:          2021,
						Order:         1,
					},
					&plateauapi.PlateauDatasetType{
						Category:      plateauapi.DatasetTypeCategoryPlateau,
						ID:            "dt_code_2",
						Name:          "name",
						Code:          "code",
						Flood:         true,
						PlateauSpecID: "id2",
						Year:          2023,
						Order:         1,
					},
					&plateauapi.PlateauDatasetType{
						Category:      plateauapi.DatasetTypeCategoryPlateau,
						ID:            "dt_code2_2",
						Name:          "name2",
						Code:          "code2",
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
						ID:       "dt_code3",
						Name:     "name3",
						Code:     "code3",
						Order:    2,
					},
				},
			},
		},
	}

	for _, tt := range tests {
		tt := tt
		t.Run(tt.name, func(t *testing.T) {
			got := tt.ft.ToDatasetTypes(tt.specs)
			assert.Equal(t, tt.want, got)
		})
	}
}
