package datacatalogmcp

import (
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

func TestConvertToAreasInput_Nil(t *testing.T) {
	result := convertToAreasInput(nil)
	assert.Nil(t, result)
}

func TestConvertToAreasInput_Empty(t *testing.T) {
	input := &SearchAreasInput{}
	result := convertToAreasInput(input)

	assert.NotNil(t, result)
	assert.Nil(t, result.ParentCode)
	assert.Nil(t, result.DatasetTypes)
	assert.Nil(t, result.Categories)
	assert.Nil(t, result.AreaTypes)
	assert.Nil(t, result.SearchTokens)
	assert.Nil(t, result.IncludeParents)
	assert.Nil(t, result.IncludeEmpty)
	assert.Nil(t, result.Deep)
}

func TestConvertToAreasInput_Full(t *testing.T) {
	input := &SearchAreasInput{
		ParentCode:     lo.ToPtr("13"),
		DatasetTypes:   []string{"bldg", "tran"},
		Categories:     []string{"PLATEAU", "RELATED"},
		AreaTypes:      []string{"PREFECTURE", "CITY"},
		SearchText:     lo.ToPtr("東京"),
		IncludeParents: lo.ToPtr(true),
		IncludeEmpty:   lo.ToPtr(false),
		Deep:           lo.ToPtr(true),
	}

	result := convertToAreasInput(input)

	assert.NotNil(t, result)
	assert.NotNil(t, result.ParentCode)
	assert.Equal(t, plateauapi.AreaCode("13"), *result.ParentCode)
	assert.Equal(t, []string{"bldg", "tran"}, result.DatasetTypes)
	assert.Equal(t, 2, len(result.Categories))
	assert.Equal(t, plateauapi.DatasetTypeCategoryPlateau, result.Categories[0])
	assert.Equal(t, plateauapi.DatasetTypeCategoryRelated, result.Categories[1])
	assert.Equal(t, 2, len(result.AreaTypes))
	assert.Equal(t, plateauapi.AreaTypePrefecture, result.AreaTypes[0])
	assert.Equal(t, plateauapi.AreaTypeCity, result.AreaTypes[1])
	assert.Equal(t, []string{"東京"}, result.SearchTokens)
	assert.NotNil(t, result.IncludeParents)
	assert.True(t, *result.IncludeParents)
	assert.NotNil(t, result.IncludeEmpty)
	assert.False(t, *result.IncludeEmpty)
	assert.NotNil(t, result.Deep)
	assert.True(t, *result.Deep)
}

func TestConvertToDatasetsInput_Nil(t *testing.T) {
	result := convertToDatasetsInput(nil)
	assert.Nil(t, result)
}

func TestConvertToDatasetsInput_Empty(t *testing.T) {
	input := &SearchDatasetsInput{}
	result := convertToDatasetsInput(input)

	assert.NotNil(t, result)
	assert.Nil(t, result.AreaCodes)
	assert.Nil(t, result.IncludeTypes)
	assert.Nil(t, result.PlateauSpec)
	assert.Nil(t, result.Year)
	assert.Nil(t, result.RegistrationYear)
	assert.Nil(t, result.SearchTokens)
	assert.Nil(t, result.Shallow)
}

func TestConvertToDatasetsInput_Full(t *testing.T) {
	input := &SearchDatasetsInput{
		AreaCodes:        []string{"13", "14"},
		DatasetTypes:     []string{"bldg", "tran"},
		PlateauSpec:      lo.ToPtr("3.0"),
		Year:             lo.ToPtr(2023),
		RegistrationYear: lo.ToPtr(2023),
		SearchText:       lo.ToPtr("建築物"),
		Shallow:          lo.ToPtr(true),
	}

	result := convertToDatasetsInput(input)

	assert.NotNil(t, result)
	assert.Equal(t, 2, len(result.AreaCodes))
	assert.Equal(t, plateauapi.AreaCode("13"), result.AreaCodes[0])
	assert.Equal(t, plateauapi.AreaCode("14"), result.AreaCodes[1])
	assert.Equal(t, []string{"bldg", "tran"}, result.IncludeTypes)
	assert.NotNil(t, result.PlateauSpec)
	assert.Equal(t, "3.0", *result.PlateauSpec)
	assert.NotNil(t, result.Year)
	assert.Equal(t, 2023, *result.Year)
	assert.NotNil(t, result.RegistrationYear)
	assert.Equal(t, 2023, *result.RegistrationYear)
	assert.Equal(t, []string{"建築物"}, result.SearchTokens)
	assert.NotNil(t, result.Shallow)
	assert.True(t, *result.Shallow)
}

func TestConvertToDatasetsInput_CategoriesOnly(t *testing.T) {
	input := &SearchDatasetsInput{
		Categories: []string{"PLATEAU", "RELATED"},
	}

	result := convertToDatasetsInput(input)

	assert.NotNil(t, result)
	// Categories are converted to IncludeTypes when DatasetTypes is empty
	assert.Equal(t, 2, len(result.IncludeTypes))
	assert.Equal(t, "PLATEAU", result.IncludeTypes[0])
	assert.Equal(t, "RELATED", result.IncludeTypes[1])
}

func TestConvertToDatasetsInput_DatasetTypesPriority(t *testing.T) {
	input := &SearchDatasetsInput{
		DatasetTypes: []string{"bldg"},
		Categories:   []string{"PLATEAU"}, // これは無視される
	}

	result := convertToDatasetsInput(input)

	assert.NotNil(t, result)
	// DatasetTypes takes priority over Categories
	assert.Equal(t, []string{"bldg"}, result.IncludeTypes)
}

func TestConvertToDatasetTypesInput_Nil(t *testing.T) {
	result := convertToDatasetTypesInput(nil)
	assert.Nil(t, result)
}

func TestConvertToDatasetTypesInput_Empty(t *testing.T) {
	input := &ListDatasetTypesInput{}
	result := convertToDatasetTypesInput(input)

	assert.NotNil(t, result)
	assert.Nil(t, result.Category)
	assert.Nil(t, result.PlateauSpec)
	assert.Nil(t, result.Year)
}

func TestConvertToDatasetTypesInput_Full(t *testing.T) {
	input := &ListDatasetTypesInput{
		Category:    lo.ToPtr("PLATEAU"),
		PlateauSpec: lo.ToPtr("3.0"),
		Year:        lo.ToPtr(2023),
	}

	result := convertToDatasetTypesInput(input)

	assert.NotNil(t, result)
	assert.NotNil(t, result.Category)
	assert.Equal(t, plateauapi.DatasetTypeCategoryPlateau, *result.Category)
	assert.NotNil(t, result.PlateauSpec)
	assert.Equal(t, "3.0", *result.PlateauSpec)
	assert.NotNil(t, result.Year)
	assert.Equal(t, 2023, *result.Year)
}

func TestConvertToDatasetTypesInput_RelatedCategory(t *testing.T) {
	input := &ListDatasetTypesInput{
		Category: lo.ToPtr("RELATED"),
	}

	result := convertToDatasetTypesInput(input)

	assert.NotNil(t, result)
	assert.NotNil(t, result.Category)
	assert.Equal(t, plateauapi.DatasetTypeCategoryRelated, *result.Category)
}

func TestConvertToDatasetTypesInput_GenericCategory(t *testing.T) {
	input := &ListDatasetTypesInput{
		Category: lo.ToPtr("GENERIC"),
	}

	result := convertToDatasetTypesInput(input)

	assert.NotNil(t, result)
	assert.NotNil(t, result.Category)
	assert.Equal(t, plateauapi.DatasetTypeCategoryGeneric, *result.Category)
}
