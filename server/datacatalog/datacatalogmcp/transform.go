package datacatalogmcp

import (
	"context"
	"reflect"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

const maxResults = 100

// TransformMetadata converts GraphQL metadata to MCP response
func TransformMetadata(ctx context.Context, repo plateauapi.Repo) (*GetMetadataResponse, error) {
	// Get available years
	years, err := repo.Years(ctx)
	if err != nil {
		return nil, err
	}

	// Get PLATEAU specs
	specs, err := repo.PlateauSpecs(ctx)
	if err != nil {
		return nil, err
	}

	// Get all areas to count
	allAreas, err := repo.Areas(ctx, nil)
	if err != nil {
		return nil, err
	}

	// Get all datasets to count
	allDatasets, err := repo.Datasets(ctx, nil)
	if err != nil {
		return nil, err
	}

	return &GetMetadataResponse{
		AvailableYears: years,
		PlateauSpecs:   transformPlateauSpecs(specs),
		TotalAreas:     len(allAreas),
		TotalDatasets:  len(allDatasets),
	}, nil
}

func transformPlateauSpecs(specs []*plateauapi.PlateauSpec) []PlateauSpecMajorInfo {
	return lo.Map(specs, func(spec *plateauapi.PlateauSpec, _ int) PlateauSpecMajorInfo {
		return PlateauSpecMajorInfo{
			ID:            string(spec.ID),
			MajorVersion:  spec.MajorVersion,
			Year:          spec.Year,
			MinorVersions: transformPlateauSpecMinors(spec.MinorVersions),
		}
	})
}

func transformPlateauSpecMinors(minors []*plateauapi.PlateauSpecMinor) []PlateauSpecMinorInfo {
	return lo.Map(minors, func(minor *plateauapi.PlateauSpecMinor, _ int) PlateauSpecMinorInfo {
		return PlateauSpecMinorInfo{
			ID:      string(minor.ID),
			Name:    minor.Name,
			Version: minor.Version,
		}
	})
}

// CreateResponseMetadata creates metadata for search responses
func CreateResponseMetadata(totalCount int, suggestions []string) ResponseMetadata {
	returnedCount := totalCount
	hasMore := false

	if totalCount > maxResults {
		returnedCount = maxResults
		hasMore = true
	}

	return ResponseMetadata{
		TotalCount:            totalCount,
		ReturnedCount:         returnedCount,
		HasMore:               hasMore,
		RefinementSuggestions: suggestions,
	}
}

// TransformSearchAreas converts GraphQL areas to MCP response
func TransformSearchAreas(areas []plateauapi.Area, input *SearchAreasInput) *SearchAreasResponse {
	areaInfos := lo.Map(areas, func(area plateauapi.Area, _ int) AreaInfo {
		var parentID *string
		if area.GetParentID() != nil {
			pid := string(*area.GetParentID())
			parentID = &pid
		}

		return AreaInfo{
			ID:           string(area.GetID()),
			Type:         string(area.GetType()),
			Code:         string(area.GetCode()),
			Name:         area.GetName(),
			ParentID:     parentID,
			DatasetCount: len(area.GetDatasets()),
		}
	})

	// Limit results
	totalCount := len(areaInfos)
	if totalCount > maxResults {
		areaInfos = areaInfos[:maxResults]
	}

	// Generate suggestions
	suggestions := generateAreaSuggestions(totalCount, input)

	return &SearchAreasResponse{
		Areas:    areaInfos,
		Metadata: CreateResponseMetadata(totalCount, suggestions),
	}
}

func generateAreaSuggestions(totalCount int, input *SearchAreasInput) []string {
	if totalCount <= maxResults {
		return nil
	}

	suggestions := []string{}

	if input == nil || input.ParentCode == nil {
		suggestions = append(suggestions, "親地域コードで絞り込む (parent_code パラメータ)")
	}

	if input == nil || len(input.DatasetTypes) == 0 {
		suggestions = append(suggestions, "データセット種類で絞り込む (dataset_types パラメータ)")
	}

	if input == nil || len(input.AreaTypes) == 0 {
		suggestions = append(suggestions, "地域タイプで絞り込む (area_types パラメータ: PREFECTURE, CITY, WARD)")
	}

	if input == nil || input.SearchText == nil {
		suggestions = append(suggestions, "検索文字列で絞り込む (search_text パラメータ)")
	}

	return suggestions
}

// TransformGetArea converts GraphQL area to MCP response
func TransformGetArea(area plateauapi.Area) *GetAreaResponse {
	var parent *AreaParent
	// We need to check if Parent field exists before calling GetParent()
	// because GetParent() dereferences the parent pointer which causes panic when nil
	var hasParent bool
	switch a := area.(type) {
	case *plateauapi.City:
		hasParent = a.Parent != nil
	case *plateauapi.Ward:
		hasParent = a.Parent != nil
	default:
		hasParent = false // Prefecture has no parent
	}

	if hasParent {
		p := area.GetParent()
		parent = &AreaParent{
			ID:   string(p.GetID()),
			Code: string(p.GetCode()),
			Name: p.GetName(),
		}
	}

	children := lo.Map(area.GetChildren(), func(child plateauapi.Area, _ int) AreaChild {
		return AreaChild{
			ID:   string(child.GetID()),
			Code: string(child.GetCode()),
			Name: child.GetName(),
		}
	})

	var planarCrs *string
	if city, ok := area.(*plateauapi.City); ok {
		if city.PlanarCrsEpsgCode != nil {
			crs := *city.PlanarCrsEpsgCode
			planarCrs = &crs
		}
	}

	return &GetAreaResponse{
		ID:                string(area.GetID()),
		Type:              string(area.GetType()),
		Code:              string(area.GetCode()),
		Name:              area.GetName(),
		Parent:            parent,
		Children:          children,
		PlanarCrsEpsgCode: planarCrs,
	}
}

// TransformSearchDatasets converts GraphQL datasets to MCP response
func TransformSearchDatasets(datasets []plateauapi.Dataset, input *SearchDatasetsInput) *SearchDatasetsResponse {
	datasetInfos := lo.Map(datasets, func(dataset plateauapi.Dataset, _ int) DatasetInfo {
		return transformDatasetInfo(dataset)
	})

	// Limit results
	totalCount := len(datasetInfos)
	if totalCount > maxResults {
		datasetInfos = datasetInfos[:maxResults]
	}

	// Generate suggestions
	suggestions := generateDatasetSuggestions(totalCount, input)

	return &SearchDatasetsResponse{
		Datasets: datasetInfos,
		Metadata: CreateResponseMetadata(totalCount, suggestions),
	}
}

func transformDatasetInfo(dataset plateauapi.Dataset) DatasetInfo {
	var desc *string
	if dataset.GetDescription() != nil {
		d := *dataset.GetDescription()
		desc = &d
	}

	var plateauSpec *string
	if pd, ok := dataset.(*plateauapi.PlateauDataset); ok {
		if pd.PlateauSpecMinor != nil {
			ps := pd.PlateauSpecMinor.Name
			plateauSpec = &ps
		}
	}

	// Get dataset type safely
	// We need to check if Type field is nil before calling GetType()
	// because GetType() dereferences the pointer which causes panic when nil
	var typeInfo DatasetTypeInfo
	var hasType bool

	// Check for nil Type field based on dataset type
	switch d := dataset.(type) {
	case *plateauapi.PlateauDataset:
		hasType = d.Type != nil
	case *plateauapi.RelatedDataset:
		hasType = d.Type != nil
	case *plateauapi.GenericDataset:
		hasType = d.Type != nil
	default:
		hasType = true // Unknown type, assume it has type
	}

	if hasType {
		dt := dataset.GetType()
		typeInfo = DatasetTypeInfo{
			Code:     dataset.GetTypeCode(),
			Name:     dt.GetName(),
			Category: string(dt.GetCategory()),
		}
	} else {
		// Fallback when type is nil
		typeInfo = DatasetTypeInfo{
			Code:     dataset.GetTypeCode(),
			Name:     "",
			Category: "",
		}
	}

	return DatasetInfo{
		ID:          string(dataset.GetID()),
		Name:        dataset.GetName(),
		Description: desc,
		Type:        typeInfo,
		Area: DatasetAreaInfo{
			Prefecture: ptrString(dataset.GetPrefecture()),
			City:       ptrString(dataset.GetCity()),
			Ward:       ptrString(dataset.GetWard()),
		},
		Year:             dataset.GetYear(),
		RegistrationYear: dataset.GetRegisterationYear(),
		PlateauSpec:      plateauSpec,
		ItemCount:        len(dataset.GetItems()),
	}
}

func ptrString(area plateauapi.Area) *string {
	if area == nil {
		return nil
	}
	// Check if area interface contains a nil value
	// This can happen when the underlying concrete type is nil
	if reflect.ValueOf(area).IsNil() {
		return nil
	}
	name := area.GetName()
	return &name
}

func generateDatasetSuggestions(totalCount int, input *SearchDatasetsInput) []string {
	if totalCount <= maxResults {
		return nil
	}

	suggestions := []string{}

	if input == nil || len(input.AreaCodes) == 0 {
		suggestions = append(suggestions, "地域コードで絞り込む (area_codes パラメータ)")
	}

	if input == nil || len(input.DatasetTypes) == 0 {
		suggestions = append(suggestions, "データセット種類で絞り込む (dataset_types パラメータ)")
	}

	if input == nil || input.Year == nil {
		suggestions = append(suggestions, "年度で絞り込む (year パラメータ)")
	}

	if input == nil || input.SearchText == nil {
		suggestions = append(suggestions, "検索文字列で絞り込む (search_text パラメータ)")
	}

	return suggestions
}

// TransformGetDataset converts GraphQL dataset to MCP response.
// host is the externally reachable origin (scheme + host) of the API and is
// used to build composite/latest URLs. An empty host omits those URLs.
func TransformGetDataset(dataset plateauapi.Dataset, host string) *GetDatasetResponse {
	var desc *string
	if dataset.GetDescription() != nil {
		d := *dataset.GetDescription()
		desc = &d
	}

	var plateauSpec *PlateauSpecInfo
	if pd, ok := dataset.(*plateauapi.PlateauDataset); ok {
		if pd.PlateauSpecMinor != nil {
			plateauSpec = &PlateauSpecInfo{
				Name:    pd.PlateauSpecMinor.Name,
				Version: pd.PlateauSpecMinor.Version,
			}
		}
	}

	var openDataURL *string
	if dataset.GetOpenDataURL() != nil {
		url := *dataset.GetOpenDataURL()
		openDataURL = &url
	}

	items := lo.Map(dataset.GetItems(), func(item plateauapi.DatasetItem, _ int) DatasetItemInfo {
		return transformDatasetItem(item, dataset, host)
	})

	// Get dataset type safely (same logic as transformDatasetInfo)
	var typeInfo DatasetTypeInfo
	var hasType bool
	switch d := dataset.(type) {
	case *plateauapi.PlateauDataset:
		hasType = d.Type != nil
	case *plateauapi.RelatedDataset:
		hasType = d.Type != nil
	case *plateauapi.GenericDataset:
		hasType = d.Type != nil
	default:
		hasType = true
	}

	if hasType {
		dt := dataset.GetType()
		typeInfo = DatasetTypeInfo{
			Code:     dataset.GetTypeCode(),
			Name:     dt.GetName(),
			Category: string(dt.GetCategory()),
		}
	} else {
		typeInfo = DatasetTypeInfo{
			Code:     dataset.GetTypeCode(),
			Name:     "",
			Category: "",
		}
	}

	return &GetDatasetResponse{
		ID:          string(dataset.GetID()),
		Name:        dataset.GetName(),
		Description: desc,
		Type:        typeInfo,
		Area: DatasetAreaDetail{
			Prefecture: transformAreaParent(dataset.GetPrefecture()),
			City:       transformAreaParent(dataset.GetCity()),
			Ward:       transformAreaParent(dataset.GetWard()),
		},
		Year:             dataset.GetYear(),
		RegistrationYear: dataset.GetRegisterationYear(),
		PlateauSpec:      plateauSpec,
		Groups:           dataset.GetGroups(),
		OpenDataURL:      openDataURL,
		Items:            items,
	}
}

func transformAreaParent(area plateauapi.Area) *AreaParent {
	if area == nil {
		return nil
	}
	// Check if area interface contains a nil value
	if reflect.ValueOf(area).IsNil() {
		return nil
	}
	return &AreaParent{
		ID:   string(area.GetID()),
		Code: string(area.GetCode()),
		Name: area.GetName(),
	}
}

func transformDatasetItem(item plateauapi.DatasetItem, parent plateauapi.Dataset, host string) DatasetItemInfo {
	info := DatasetItemInfo{
		ID:     string(item.GetID()),
		Name:   item.GetName(),
		Format: string(item.GetFormat()),
		URL:    item.GetURL(),
		Layers: item.GetLayers(),
	}

	pitem, _ := item.(*plateauapi.PlateauDatasetItem)
	pparent, _ := parent.(*plateauapi.PlateauDataset)

	if pitem != nil {
		if pitem.Lod != nil {
			lod := *pitem.Lod
			info.Lod = &lod
		}
		if pitem.Texture != nil {
			texture := string(*pitem.Texture)
			info.Texture = &texture
		}
	}
	if pparent != nil && strings.Contains(string(pparent.ID), "_interior") {
		t := true
		info.Interior = &t
	}
	if pitem != nil && pparent != nil {
		if u := plateauapi.BuildPlateauItemDynamicURL(host, pitem, pparent, false); u != "" {
			info.CompositeURL = &u
		}
		if u := plateauapi.BuildPlateauItemDynamicURL(host, pitem, pparent, true); u != "" {
			info.LatestURL = &u
		}
	}

	return info
}

// TransformListDatasetTypes converts GraphQL dataset types to MCP response
func TransformListDatasetTypes(types []plateauapi.DatasetType) *ListDatasetTypesResponse {
	typeInfos := lo.Map(types, func(t plateauapi.DatasetType, _ int) DatasetTypeListInfo {
		var year *int
		if pt, ok := t.(*plateauapi.PlateauDatasetType); ok {
			y := pt.Year
			year = &y
		}

		return DatasetTypeListInfo{
			ID:           string(t.GetID()),
			Code:         t.GetCode(),
			Name:         t.GetName(),
			Category:     string(t.GetCategory()),
			Year:         year,
			DatasetCount: len(t.GetDatasets()),
		}
	})

	return &ListDatasetTypesResponse{
		DatasetTypes: typeInfos,
	}
}
