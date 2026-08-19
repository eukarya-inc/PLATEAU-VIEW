package plateauapi

import (
	"context"
	"testing"

	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestFilterDataset(t *testing.T) {
	t.Run("emergency_route", func(t *testing.T) {
		assert.True(t, filterDataset(RelatedDataset{
			TypeCode: "emergency_route",
		}, DatasetsInput{
			IncludeTypes: []string{"emergency_route"},
		}, nil))
	})

	t.Run("search tokens case insensitive", func(t *testing.T) {
		// uppercase search token matches lowercase name
		assert.True(t, filterDataset(PlateauDataset{
			Name: "tokyo building",
		}, DatasetsInput{
			SearchTokens: []string{"TOKYO"},
		}, nil))

		// lowercase search token matches uppercase name
		assert.True(t, filterDataset(PlateauDataset{
			Name: "TOKYO BUILDING",
		}, DatasetsInput{
			SearchTokens: []string{"tokyo"},
		}, nil))

		// mixed case
		assert.True(t, filterDataset(PlateauDataset{
			Name: "Tokyo Building",
		}, DatasetsInput{
			SearchTokens: []string{"tOkYo"},
		}, nil))
	})

	t.Run("default stage", func(t *testing.T) {
		assert.False(t, filterDataset(RelatedDataset{
			TypeCode: "emergency_route",
			Admin: &Admin{
				Stage: "beta",
			},
		}, DatasetsInput{}, nil))
	})

	t.Run("ar", func(t *testing.T) {
		assert.True(t, filterDataset(RelatedDataset{
			Ar: false,
		}, DatasetsInput{
			Ar: nil,
		}, nil))
		assert.True(t, filterDataset(RelatedDataset{
			Ar: true,
		}, DatasetsInput{
			Ar: nil,
		}, nil))
		assert.True(t, filterDataset(RelatedDataset{
			Ar: true,
		}, DatasetsInput{
			Ar: lo.ToPtr(true),
		}, nil))
		assert.False(t, filterDataset(RelatedDataset{
			Ar: false,
		}, DatasetsInput{
			Ar: lo.ToPtr(true),
		}, nil))
		assert.True(t, filterDataset(RelatedDataset{
			Ar: false,
		}, DatasetsInput{
			Ar: lo.ToPtr(false),
		}, nil))
		assert.False(t, filterDataset(RelatedDataset{
			Ar: false,
		}, DatasetsInput{
			Ar: lo.ToPtr(true),
		}, nil))
	})

	t.Run("beta stage", func(t *testing.T) {
		assert.True(t, filterDataset(RelatedDataset{
			TypeCode: "emergency_route",
			Admin: map[string]any{
				"stage": "beta",
			},
		}, DatasetsInput{
			IncludeTypes: []string{"emergency_route"},
		}, []string{"beta"}))
	})

	t.Run("beta stage 2", func(t *testing.T) {
		assert.True(t, filterDataset(RelatedDataset{
			TypeCode: "emergency_route",
		}, DatasetsInput{
			IncludeTypes: []string{"emergency_route"},
		}, nil))
	})

	t.Run("groupedOnly", func(t *testing.T) {
		assert.False(t, filterDataset(PlateauDataset{}, DatasetsInput{
			GroupedOnly: lo.ToPtr(true),
		}, nil))
	})
}

func TestFilterArea(t *testing.T) {
	testCases := []struct {
		name                string
		area                Area
		input               AreasInput
		areasWithoutDataset map[ID]struct{}
		expected            bool
	}{
		{
			name: "Prefecture with search tokens",
			area: Prefecture{Name: "Tokyo"},
			input: AreasInput{
				SearchTokens: []string{"Tokyo"},
			},
			expected: true,
		},
		{
			name: "Prefecture without search tokens",
			area: Prefecture{Name: "Tokyo"},
			input: AreasInput{
				SearchTokens: []string{},
			},
			expected: true,
		},
		{
			name: "Prefecture without non-matching search tokens",
			area: Prefecture{Name: "Tokyo"},
			input: AreasInput{
				SearchTokens: []string{"Kanagawa"},
			},
			expected: false,
		},
		{
			name: "Prefecture with case insensitive search tokens",
			area: Prefecture{Name: "Tokyo"},
			input: AreasInput{
				SearchTokens: []string{"TOKYO"},
			},
			expected: true,
		},
		{
			name: "Prefecture with lowercase search tokens",
			area: Prefecture{Name: "TOKYO"},
			input: AreasInput{
				SearchTokens: []string{"tokyo"},
			},
			expected: true,
		},
		{
			name: "City with search tokens and matching parent code",
			area: City{Name: "Shinjuku", PrefectureCode: "13"},
			input: AreasInput{
				SearchTokens: []string{"Shinjuku"},
				ParentCode:   lo.ToPtr(AreaCode("13")),
			},
			expected: true,
		},
		{
			name: "City with search tokens and non-matching parent code",
			area: City{Name: "Shinjuku", PrefectureCode: "13"},
			input: AreasInput{
				SearchTokens: []string{"Shinjuku"},
				ParentCode:   lo.ToPtr(AreaCode("14")),
			},
			expected: false,
		},
		{
			name: "Ward with search tokens and matching parent code",
			area: Ward{Name: "Shinjuku", PrefectureCode: "13", CityCode: "13104"},
			input: AreasInput{
				SearchTokens: []string{"Shinjuku"},
				ParentCode:   lo.ToPtr(AreaCode("13104")),
			},
			expected: true,
		},
		{
			name: "Ward with search tokens and non-matching parent code",
			area: Ward{Name: "Shinjuku", PrefectureCode: "13", CityCode: "13104"},
			input: AreasInput{
				SearchTokens: []string{"Shinjuku"},
				ParentCode:   lo.ToPtr(AreaCode("13105")),
			},
			expected: false,
		},
		{
			name: "Ward without search tokens",
			area: Ward{Name: "Shinjuku", PrefectureCode: "13", CityCode: "13104"},
			input: AreasInput{
				SearchTokens: []string{},
			},
			expected: true,
		},
		{
			name: "shallow",
			area: Ward{Name: "Shinjuku", PrefectureCode: "13", CityCode: "13104"},
			input: AreasInput{
				ParentCode: lo.ToPtr(AreaCode("13")),
			},
			expected: false,
		},
		{
			name: "deep",
			area: Ward{Name: "Shinjuku", PrefectureCode: "13", CityCode: "13104"},
			input: AreasInput{
				ParentCode: lo.ToPtr(AreaCode("13")),
				Deep:       lo.ToPtr(true),
			},
			expected: true,
		},
		{
			name: "includeEmpty",
			area: Ward{ID: "aaa"},
			input: AreasInput{
				IncludeEmpty: lo.ToPtr(true),
			},
			areasWithoutDataset: map[ID]struct{}{"aaa": {}},
			expected:            true,
		},
		{
			name:                "without includeEmpty",
			area:                Ward{ID: "aaa"},
			input:               AreasInput{},
			areasWithoutDataset: map[ID]struct{}{"aaa": {}},
			expected:            false,
		},
	}

	for _, tc := range testCases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			actual := filterArea(tc.area, tc.input, tc.areasWithoutDataset)
			assert.Equal(t, tc.expected, actual)
		})
	}
}

func TestFilterByCode(t *testing.T) {
	assert.True(t, filterByCode("a_a", DatasetTypeCategoryGeneric, []string{"a_a"}, nil))
	assert.True(t, filterByCode("a", DatasetTypeCategoryGeneric, []string{"a", "b"}, nil))
	assert.True(t, filterByCode("b", DatasetTypeCategoryGeneric, []string{"a", "b"}, nil))
	assert.True(t, filterByCode("a", DatasetTypeCategoryGeneric, []string{"generic"}, nil))
	assert.False(t, filterByCode("b", DatasetTypeCategoryGeneric, []string{"a"}, nil))
	assert.False(t, filterByCode("a", DatasetTypeCategoryGeneric, nil, []string{"a"}))
	assert.False(t, filterByCode("a", DatasetTypeCategoryGeneric, nil, []string{"a", "b"}))
	assert.False(t, filterByCode("a", DatasetTypeCategoryGeneric, []string{"a"}, []string{"a", "b"}))
	assert.False(t, filterByCode("a", DatasetTypeCategoryGeneric, nil, []string{"generic"}))
}

func TestFilterByPlateauSpec(t *testing.T) {
	testCases := []struct {
		name        string
		querySpec   *string
		datasetSpec string
		expected    bool
	}{
		{
			name:        "Nil query spec and empty dataset spec",
			querySpec:   nil,
			datasetSpec: "",
			expected:    true,
		},
		{
			name:        "Empty query spec and empty dataset spec",
			datasetSpec: "",
			querySpec:   lo.ToPtr(""),
			expected:    true,
		},
		{
			name:        "Nil query spec and non-empty dataset spec",
			querySpec:   nil,
			datasetSpec: "1.0",
			expected:    true,
		},
		{
			name:        "Empty query spec and non-empty dataset spec",
			querySpec:   lo.ToPtr(""),
			datasetSpec: "1.0",
			expected:    true,
		},
		{
			name:        "Non-empty query spec and non-empty dataset spec with matching major version",
			querySpec:   lo.ToPtr("ps_1"),
			datasetSpec: "1.2",
			expected:    true,
		},
		{
			name:        "Non-empty query spec and non-empty dataset spec with non-matching major version",
			querySpec:   lo.ToPtr("1"),
			datasetSpec: "2.0",
			expected:    false,
		},
		{
			name:        "Non-empty query spec and non-empty dataset spec with matching major and minor version",
			querySpec:   lo.ToPtr("1.2"),
			datasetSpec: "1.2",
			expected:    true,
		},
		{
			name:        "Non-empty query spec and non-empty dataset spec with non-matching minor version",
			querySpec:   lo.ToPtr("1.2"),
			datasetSpec: "1.3",
			expected:    false,
		},
		{
			name:        "Non-empty query spec and non-empty dataset spec with non-matching major version and matching minor version",
			querySpec:   lo.ToPtr("1.2"),
			datasetSpec: "2.2",
			expected:    false,
		},
	}

	for _, tc := range testCases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			actual := filterByPlateauSpec(tc.querySpec, tc.datasetSpec)
			assert.Equal(t, tc.expected, actual)
		})
	}
}

func TestFilterDatasetType(t *testing.T) {
	testCases := []struct {
		name     string
		ty       DatasetType
		input    DatasetTypesInput
		expected bool
	}{
		{
			name: "PlateauDatasetType with matching category, year and plateau spec",
			ty: PlateauDatasetType{
				Category:      DatasetTypeCategoryPlateau,
				Year:          2021,
				PlateauSpecID: "ps_1.0",
			},
			input: DatasetTypesInput{
				Category:    lo.ToPtr(DatasetTypeCategoryPlateau),
				Year:        lo.ToPtr(2021),
				PlateauSpec: lo.ToPtr("1.0"),
			},
			expected: true,
		},
		{
			name: "PlateauDatasetType with matching plateau spec major version",
			ty: PlateauDatasetType{
				Category:      DatasetTypeCategoryPlateau,
				PlateauSpecID: "ps_1",
			},
			input: DatasetTypesInput{
				PlateauSpec: lo.ToPtr("1"),
			},
			expected: true,
		},
		{
			name: "PlateauDatasetType with non-matching category",
			ty: PlateauDatasetType{
				Category:      DatasetTypeCategoryPlateau,
				Year:          2021,
				PlateauSpecID: "ps_1.0",
			},
			input: DatasetTypesInput{
				Category: lo.ToPtr(DatasetTypeCategoryRelated),
			},
			expected: false,
		},
		{
			name: "PlateauDatasetType with non-matching year",
			ty: PlateauDatasetType{
				Category:      DatasetTypeCategoryPlateau,
				Year:          2021,
				PlateauSpecID: "ps_1.0",
			},
			input: DatasetTypesInput{
				Year: lo.ToPtr(2022),
			},
			expected: false,
		},
		{
			name: "PlateauDatasetType with non-matching plateau spec",
			ty: PlateauDatasetType{
				Category:      DatasetTypeCategoryPlateau,
				Year:          2021,
				PlateauSpecID: "ps_1.0",
			},
			input: DatasetTypesInput{
				PlateauSpec: lo.ToPtr("2.0"),
			},
			expected: false,
		},
		{
			name: "RelatedDatasetType with matching category",
			ty: RelatedDatasetType{
				Category: DatasetTypeCategoryRelated,
			},
			input: DatasetTypesInput{
				Category: lo.ToPtr(DatasetTypeCategoryRelated),
			},
			expected: true,
		},
		{
			name: "RelatedDatasetType with non-matching category",
			ty: RelatedDatasetType{
				Category: DatasetTypeCategoryRelated,
			},
			input: DatasetTypesInput{
				Category: lo.ToPtr(DatasetTypeCategoryPlateau),
			},
			expected: false,
		},
		{
			name: "GenericDatasetType with matching category",
			ty: GenericDatasetType{
				Category: DatasetTypeCategoryGeneric,
			},
			input: DatasetTypesInput{
				Category: lo.ToPtr(DatasetTypeCategoryGeneric),
			},
			expected: true,
		},
		{
			name: "GenericDatasetType with non-matching category",
			ty: GenericDatasetType{
				Category: DatasetTypeCategoryGeneric,
			},
			input: DatasetTypesInput{
				Category: lo.ToPtr(DatasetTypeCategoryPlateau),
			},
			expected: false,
		},
	}

	for _, tc := range testCases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			actual := filterDatasetType(tc.ty, tc.input)
			assert.Equal(t, tc.expected, actual)
		})
	}
}

func TestVisibilityKey(t *testing.T) {
	ctx := context.Background()
	assert.Equal(t, "public|", VisibilityKey(ctx))
	assert.Equal(t, "admin|", VisibilityKey(BypassAdminRemoval(ctx, true)))
	assert.Equal(t, "public|", VisibilityKey(BypassAdminRemoval(ctx, false)))

	// Stage order must not change the key, but the stage set must.
	assert.Equal(t,
		VisibilityKey(AllowAdminStages(ctx, []string{"alpha", "beta"})),
		VisibilityKey(AllowAdminStages(ctx, []string{"beta", "alpha"})),
	)
	assert.NotEqual(t,
		VisibilityKey(AllowAdminStages(ctx, []string{"beta"})),
		VisibilityKey(AllowAdminStages(ctx, []string{"alpha", "beta"})),
	)
}
