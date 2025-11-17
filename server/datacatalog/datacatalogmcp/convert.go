package datacatalogmcp

import (
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

// convertToAreasInput converts MCP input to GraphQL AreasInput
func convertToAreasInput(input *SearchAreasInput) *plateauapi.AreasInput {
	if input == nil {
		return nil
	}

	var parentCode *plateauapi.AreaCode
	if input.ParentCode != nil {
		pc := plateauapi.AreaCode(*input.ParentCode)
		parentCode = &pc
	}

	var categories []plateauapi.DatasetTypeCategory
	if len(input.Categories) > 0 {
		categories = lo.Map(input.Categories, func(c string, _ int) plateauapi.DatasetTypeCategory {
			return plateauapi.DatasetTypeCategory(c)
		})
	}

	var areaTypes []plateauapi.AreaType
	if len(input.AreaTypes) > 0 {
		areaTypes = lo.Map(input.AreaTypes, func(at string, _ int) plateauapi.AreaType {
			return plateauapi.AreaType(at)
		})
	}

	var searchTokens []string
	if input.SearchText != nil {
		searchTokens = []string{*input.SearchText}
	}

	return &plateauapi.AreasInput{
		ParentCode:     parentCode,
		DatasetTypes:   input.DatasetTypes,
		Categories:     categories,
		AreaTypes:      areaTypes,
		SearchTokens:   searchTokens,
		IncludeParents: input.IncludeParents,
		IncludeEmpty:   input.IncludeEmpty,
		Deep:           input.Deep,
	}
}

// convertToDatasetsInput converts MCP input to GraphQL DatasetsInput
func convertToDatasetsInput(input *SearchDatasetsInput) *plateauapi.DatasetsInput {
	if input == nil {
		return nil
	}

	var areaCodes []plateauapi.AreaCode
	if len(input.AreaCodes) > 0 {
		areaCodes = lo.Map(input.AreaCodes, func(c string, _ int) plateauapi.AreaCode {
			return plateauapi.AreaCode(c)
		})
	}

	var includeTypes []string
	if len(input.DatasetTypes) > 0 {
		includeTypes = input.DatasetTypes
	} else if len(input.Categories) > 0 {
		// Convert categories to type codes
		includeTypes = lo.Map(input.Categories, func(c string, _ int) string {
			return string(plateauapi.DatasetTypeCategory(c))
		})
	}

	var searchTokens []string
	if input.SearchText != nil {
		searchTokens = []string{*input.SearchText}
	}

	return &plateauapi.DatasetsInput{
		AreaCodes:        areaCodes,
		IncludeTypes:     includeTypes,
		PlateauSpec:      input.PlateauSpec,
		Year:             input.Year,
		RegistrationYear: input.RegistrationYear,
		SearchTokens:     searchTokens,
		Shallow:          input.Shallow,
	}
}

// convertToDatasetTypesInput converts MCP input to GraphQL DatasetTypesInput
func convertToDatasetTypesInput(input *ListDatasetTypesInput) *plateauapi.DatasetTypesInput {
	if input == nil {
		return nil
	}

	var category *plateauapi.DatasetTypeCategory
	if input.Category != nil {
		c := plateauapi.DatasetTypeCategory(*input.Category)
		category = &c
	}

	return &plateauapi.DatasetTypesInput{
		Category:    category,
		PlateauSpec: input.PlateauSpec,
		Year:        input.Year,
	}
}
