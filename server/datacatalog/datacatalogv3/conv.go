package datacatalogv3

import (
	"fmt"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

// ExtractBaseFeatureType extracts the base feature type from a code.
// Examples: "bldg2" -> "bldg", "tran10" -> "tran", "bldg" -> "bldg"
func ExtractBaseFeatureType(code string) string {
	return strings.TrimRight(code, "0123456789")
}

// IsDerivedFeatureType returns true if the feature type code is a derived type
// (e.g., "bldg2" is derived from "bldg").
func IsDerivedFeatureType(code string) bool {
	return ExtractBaseFeatureType(code) != code
}

// filterPlateauByPriority filters plateau items by priority.
// For each city + base feature type combination, only the item with highest priority is kept.
// Items are included if:
//   - The city is public (CityPublic == true), OR
//   - The item has IsBeta() == true (Status == "確認可能")
//
// When priorities are equal, derived types (e.g., "bldg2") are preferred over base types.
// Returns: filtered items mapped to their BASE feature type code.
func filterPlateauByPriority(
	plateau map[string][]*PlateauFeatureItem,
	cityItems []*CityItem,
) map[string][]*PlateauFeatureItem {
	// Build cityID -> CityItem map for quick lookup
	cityMap := make(map[string]*CityItem)
	for _, c := range cityItems {
		if c != nil {
			cityMap[c.ID] = c
		}
	}

	// cityID -> baseFeatureType -> best entry
	type priorityEntry struct {
		priority        int
		featureTypeCode string // original code (e.g., "bldg2")
		item            *PlateauFeatureItem
	}

	bestByCity := make(map[string]map[string]priorityEntry)

	for featureTypeCode, items := range plateau {
		baseFeatureType := ExtractBaseFeatureType(featureTypeCode)

		for _, item := range items {
			if item == nil {
				continue
			}

			// Check if this item should be included in the catalog:
			// - If the city is public (CityPublic == true), include regardless of Status
			// - Otherwise, only include if Status == "確認可能" (IsBeta() == true)
			cityItem := cityMap[item.City]
			isPublicCity := cityItem != nil && cityItem.CityPublic
			if !isPublicCity && !item.IsBeta() {
				continue
			}

			cityID := item.City
			if bestByCity[cityID] == nil {
				bestByCity[cityID] = make(map[string]priorityEntry)
			}

			current, exists := bestByCity[cityID][baseFeatureType]
			shouldReplace := !exists ||
				item.Priority > current.priority ||
				(item.Priority == current.priority && featureTypeCode > current.featureTypeCode)

			if shouldReplace {
				bestByCity[cityID][baseFeatureType] = priorityEntry{
					priority:        item.Priority,
					featureTypeCode: featureTypeCode,
					item:            item,
				}
			}
		}
	}

	// Build result with base feature type code as key
	result := make(map[string][]*PlateauFeatureItem)
	for _, cityBest := range bestByCity {
		for baseCode, entry := range cityBest {
			result[baseCode] = append(result[baseCode], entry.item)
		}
	}

	return result
}

const sampleCode = "sample"

func (all *AllData) Into() (res *plateauapi.InMemoryRepoContext, warning []string) {
	if all == nil {
		warning = append(warning, "data is nil")
		return
	}

	res = &plateauapi.InMemoryRepoContext{
		Name:     all.Name,
		Areas:    plateauapi.Areas{},
		Datasets: plateauapi.Datasets{},
	}
	res.PlateauSpecs = plateauapi.PlateauSpecsFrom(all.PlateauSpecs)
	res.DatasetTypes = all.FeatureTypes.ToDatasetTypes(res.PlateauSpecs)

	ic := newInternalContext()
	ic.cmsinfo = all.CMSInfo
	ic.regYear = all.Year

	// layer names
	ic.layerNamesForType = all.FeatureTypes.LayerNames()

	// pref and city
	for _, cityItem := range all.City {
		pref, city := cityItem.ToPrefecture(), cityItem.ToCity()
		if pref == nil || city == nil {
			continue
		}

		ic.Add(cityItem, pref, city)

		if res.Areas.FindByCodeAndType(pref.Code, plateauapi.AreaTypePrefecture) == nil {
			res.Areas.Append(plateauapi.AreaTypePrefecture, []plateauapi.Area{pref})
		}

		if res.Areas.FindByCodeAndType(city.Code, plateauapi.AreaTypeCity) == nil {
			res.Areas.Append(plateauapi.AreaTypeCity, []plateauapi.Area{city})
		}
	}

	res.Years = ic.Years()

	// wards - use all plateau items (before priority filtering) to get all wards
	for _, ft := range res.DatasetTypes[plateauapi.DatasetTypeCategoryPlateau] {
		wards, w := getWards(all.Plateau[ft.GetCode()], ic)
		warning = append(warning, w...)
		ic.AddWards(wards)
		res.Areas.Append(
			plateauapi.AreaTypeWard,
			lo.Map(wards, func(w *plateauapi.Ward, _ int) plateauapi.Area { return w }),
		)
	}

	// plateau - filter by priority and map to base feature types
	filteredPlateau := filterPlateauByPriority(all.Plateau, all.City)
	plateauDatasetTypes := res.DatasetTypes.CodeMap(plateauapi.DatasetTypeCategoryPlateau)
	plateauFeatureTypes := all.FeatureTypes.PlateauMap()

	// Track processed base codes to avoid duplicates
	processedBaseCodes := make(map[string]bool)
	for _, dt := range res.DatasetTypes[plateauapi.DatasetTypeCategoryPlateau] {
		code := dt.GetCode()
		baseCode := ExtractBaseFeatureType(code)

		// Skip if base code already processed
		if processedBaseCodes[baseCode] {
			continue
		}
		// Skip derived types (process only base types)
		if baseCode != code {
			continue
		}
		processedBaseCodes[baseCode] = true

		items := filteredPlateau[baseCode]
		if items == nil {
			continue
		}

		datasets, w := convertPlateau(
			items,
			baseCode, // Use base feature type code
			false,    // not flow
			res.PlateauSpecs,
			plateauDatasetTypes,
			plateauFeatureTypes,
			ic,
		)
		warning = append(warning, w...)
		res.Datasets.Append(plateauapi.DatasetTypeCategoryPlateau, datasets)
	}

	// flow - convert Flow model data (always beta, no priority filtering)
	// Only process if FlowEnabled is true in metadata
	if all.CMSInfo.FlowEnabled && len(all.Flow) > 0 {
		// Register Flow-referenced cities that might be in alpha stage
		// Flow items can reference alpha-stage cities for testing purposes
		flowCityItemIDs := make(map[string]bool)
		for _, items := range all.Flow {
			for _, item := range items {
				if item.City != "" {
					flowCityItemIDs[item.City] = true
				}
			}
		}

		for _, cityItem := range all.City {
			if !flowCityItemIDs[cityItem.ID] {
				continue
			}
			// Skip if already registered
			if ic.CityItem(cityItem.ID) != nil {
				continue
			}
			// Register even if alpha stage (for Flow)
			city := cityItem.ToCity()
			if city == nil {
				continue
			}

			// Create prefecture manually (ToPrefecture checks IsPublicOrBeta, but we want to bypass that for Flow)
			prefCode := cityItem.CityCode[:2]
			pref := &plateauapi.Prefecture{
				ID:   plateauapi.NewID(prefCode, plateauapi.TypePrefecture),
				Name: cityItem.Prefecture,
				Code: plateauapi.AreaCode(prefCode),
				Type: plateauapi.AreaTypePrefecture,
			}

			ic.Add(cityItem, pref, city)

			if res.Areas.FindByCodeAndType(pref.Code, plateauapi.AreaTypePrefecture) == nil {
				res.Areas.Append(plateauapi.AreaTypePrefecture, []plateauapi.Area{pref})
			}
			if res.Areas.FindByCodeAndType(city.Code, plateauapi.AreaTypeCity) == nil {
				res.Areas.Append(plateauapi.AreaTypeCity, []plateauapi.Area{city})
			}
		}

		for code, items := range all.Flow {
			if len(items) == 0 {
				continue
			}

			baseCode := ExtractBaseFeatureType(code)
			datasets, w := convertPlateau(
				items,
				baseCode, // Use base feature type code
				true,     // is flow
				res.PlateauSpecs,
				plateauDatasetTypes,
				plateauFeatureTypes,
				ic,
			)
			warning = append(warning, w...)
			res.Datasets.Append(plateauapi.DatasetTypeCategoryPlateau, datasets)
		}
	}

	// sample
	sample, _ := res.DatasetTypes.FindByCode(sampleCode, plateauapi.DatasetTypeCategoryGeneric).(*plateauapi.GenericDatasetType)
	if sample != nil {
		sample, w := convertSample(
			sample,
			all,
			res.PlateauSpecs,
			plateauDatasetTypes,
			plateauFeatureTypes,
			ic,
		)
		warning = append(warning, w...)
		res.Datasets.Append(
			plateauapi.DatasetTypeCategoryGeneric,
			plateauapi.ToDatasets(sample),
		)
	}

	// related
	{
		datasets, w := convertRelated(all.Related, res.DatasetTypes[plateauapi.DatasetTypeCategoryRelated], ic)
		warning = append(warning, w...)
		res.Datasets.Append(plateauapi.DatasetTypeCategoryRelated, datasets)
	}

	// generic
	{
		datasets, w := convertGeneric(all.Generic, res.DatasetTypes[plateauapi.DatasetTypeCategoryGeneric], ic)
		warning = append(warning, w...)
		res.Datasets.Append(plateauapi.DatasetTypeCategoryGeneric, datasets)
	}

	// global lodstat (hardcoded)
	{
		datasets, w := createGlobalLodstatDatasets(res.DatasetTypes[plateauapi.DatasetTypeCategoryGeneric], all.Host)
		warning = append(warning, w...)
		res.Datasets.Append(plateauapi.DatasetTypeCategoryGeneric, datasets)
	}

	// citygml
	{
		var w []string
		res.CityGML, w = toCityGMLs(all, ic.regYear)
		warning = append(warning, w...)
	}

	// Filter out DatasetTypes without any Datasets
	{
		var w []string
		res.DatasetTypes, w = filterDatasetTypesWithoutDatasets(res.DatasetTypes, res.Datasets)
		warning = append(warning, w...)
	}

	return
}

func getWards(items []*PlateauFeatureItem, ic *internalContext) (res []*plateauapi.Ward, warning []string) {
	for _, ds := range items {
		area := ic.AreaContext(ds.City)
		if area == nil {
			warning = append(warning, fmt.Sprintf("plateau %s: city not found: %s", ds.ID, ds.City))
			continue
		}

		wards := ds.toWards(area.Pref, area.City)
		res = append(res, wards...)
	}

	return
}

func convertSample(
	sample plateauapi.DatasetType,
	all *AllData,
	specs []plateauapi.PlateauSpec,
	dts map[string]plateauapi.DatasetType,
	fts map[string]*FeatureType,
	ic *internalContext,
) (res []*plateauapi.GenericDataset, warning []string) {
	sampleID := sample.GetID()
	sampleCode := sample.GetCode()
	sampleName := sample.GetName()
	const idSuffix = "_sample"

	raw := make([]*plateauapi.PlateauDataset, 0, len(all.Sample)+len(all.City))

	// common sample datasets
	{
		targets := all.Sample
		datasets, w := convertPlateauRaw(
			targets,
			false,
			"",
			false, // not flow
			specs,
			dts,
			fts,
			ic,
		)
		warning = append(warning, w...)
		raw = append(raw, datasets...)
	}

	// city sample datasets
	for _, c := range all.City {
		if !c.Sample || c.CityCode == "" {
			continue
		}

		targets := all.FindPlateauFeatureItemsByCityID(c.ID)
		datasets, w := convertPlateauRaw(
			targets,
			false,
			"",
			false, // not flow
			specs,
			dts,
			fts,
			ic,
		)
		setGroupsToDatasets(datasets, []string{sampleName, c.CityName})
		warning = append(warning, w...)
		raw = append(raw, datasets...)
	}

	res = append(res, plateauapi.PlateauDatasetsToGenericDatasets(raw, sampleID, sampleCode, idSuffix)...)
	return
}

func convertPlateau(
	items []*PlateauFeatureItem,
	code string,
	isFlow bool,
	specs []plateauapi.PlateauSpec,
	dts map[string]plateauapi.DatasetType,
	fts map[string]*FeatureType,
	ic *internalContext,
) ([]plateauapi.Dataset, []string) {
	res, w := convertPlateauRaw(
		items, true, code, isFlow, specs, dts, fts, ic,
	)
	return plateauapi.ToDatasets(res), w
}

func convertPlateauRaw(
	items []*PlateauFeatureItem,
	ignoreSample bool,
	code string,
	isFlow bool,
	specs []plateauapi.PlateauSpec,
	dts map[string]plateauapi.DatasetType,
	fts map[string]*FeatureType,
	ic *internalContext,
) (res []*plateauapi.PlateauDataset, warning []string) {
	for _, ds := range items {
		if ds == nil {
			continue
		}

		ftcode := code
		if ds.FeatureType != "" {
			ftcode = ds.FeatureType
		}

		// For derived types (e.g., bldg2), use the base type's DatasetType and FeatureType
		// since derived types are merged into base types in the catalog
		baseFtcode := ExtractBaseFeatureType(ftcode)
		dt := dts[baseFtcode]
		ft := fts[baseFtcode]

		if dt == nil || ft == nil {
			warning = append(warning, fmt.Sprintf("plateau %s: invalid feature type: %s", ds.ID, ftcode))
			continue
		}

		pdt, ok := dt.(*plateauapi.PlateauDatasetType)
		if !ok {
			warning = append(warning, fmt.Sprintf("plateau %s: invalid dataset type: %s", dt.GetCode(), dt.GetName()))
			return
		}

		layerNames := ic.layerNamesForType[pdt.Code]
		cityItem := ic.CityItem(ds.City)
		if cityItem == nil {
			warning = append(warning, fmt.Sprintf("plateau %s %s: invalid city: %s", ds.ID, pdt.Code, ds.City))
			continue
		}

		if ignoreSample && cityItem.Sample {
			continue
		}

		area := ic.AreaContext(ds.City)
		if area == nil {
			warning = append(warning, fmt.Sprintf("plateau %s %s: invalid city: %s", ds.ID, pdt.Code, ds.City))
			continue
		}

		cityCode := lo.FromPtr(area.CityCode).String()
		spec := plateauapi.FindSpecMinorByName(specs, area.CityItem.Spec)
		if spec == nil {
			warning = append(warning, fmt.Sprintf("plateau %s %s: invalid spec: %s", cityCode, pdt.Code, area.CityItem.Spec))
			continue
		}

		opts := ToPlateauDatasetsOptions{
			ID:          ds.ID,
			CreatedAt:   ds.CreatedAt,
			UpdatedAt:   ds.UpdatedAt,
			Area:        area,
			Spec:        spec,
			DatasetType: pdt,
			LayerNames:  layerNames,
			FeatureType: ft,
			Year:        ic.regYear,
			CMSInfo:     ic.cmsinfo,
			IsFlow:      isFlow,
		}
		ds, w := ds.toDatasets(opts)
		warning = append(warning, w...)
		if ds != nil {
			res = append(res, ds...)
		}
	}

	return
}

func setGroupsToDatasets(datasets []*plateauapi.PlateauDataset, groups []string) {
	for _, ds := range datasets {
		if ds == nil {
			continue
		}

		ds.Groups = make([]string, len(groups))
		copy(ds.Groups, groups)
	}
}

func convertRelated(items []*RelatedItem, datasetTypes []plateauapi.DatasetType, ic *internalContext) (res []plateauapi.Dataset, warning []string) {
	for _, ds := range items {
		area := ic.AreaContext(ds.City)
		if area == nil {
			warning = append(warning, fmt.Sprintf("related %s: invalid city: %s", ds.ID, ds.City))
			continue
		}

		ds, w := ds.toDatasets(area, datasetTypes, ic.regYear, ic.cmsinfo)
		warning = append(warning, w...)
		if ds != nil {
			res = append(res, ds...)
		}
	}

	return
}

func convertGeneric(items []*GenericItem, datasetTypes []plateauapi.DatasetType, ic *internalContext) (res []plateauapi.Dataset, warning []string) {
	for _, ds := range items {
		area := ic.AreaContext(ds.City)
		if area == nil {
			warning = append(warning, fmt.Sprintf("generic %s: invalid city: %s", ds.ID, ds.City))
			continue
		}

		ds, w := ds.toDatasets(area, datasetTypes, ic.regYear, ic.cmsinfo)
		warning = append(warning, w...)
		if ds != nil {
			res = append(res, ds...)
		}
	}

	return
}

func createGlobalLodstatDatasets(datasetTypes []plateauapi.DatasetType, host string) (res []plateauapi.Dataset, warning []string) {
	const globalCode = "global"
	const lodstatCode = "lodstat"
	const lodstatName = "PLATEAU 地域メッシュ別LOD統計情報（建築物モデル）"
	const desc = "PLATEAU 3D都市モデルの建築物モデルにおける、地域メッシュ単位のLOD統計情報を提供するベクタータイル"

	// Find global dataset type
	var globalType plateauapi.DatasetType
	for _, dt := range datasetTypes {
		if dt.GetCode() == globalCode {
			globalType = dt
			break
		}
	}

	if globalType == nil {
		warning = append(warning, "global dataset type not found")
		return nil, warning
	}

	// Build URL with host prefix (using auto mode)
	url := "/lodstat/mvt/bldg/auto/{z}/{x}/{y}.mvt"
	if host != "" {
		// Host may already contain https://, so just concatenate
		url = host + url
	}

	// Create global lodstat dataset
	dataset := &plateauapi.GenericDataset{
		ID:                plateauapi.NewID(lodstatCode, plateauapi.TypeDataset),
		Name:              lodstatName,
		Description:       lo.ToPtr(desc),
		Year:              2025,
		RegisterationYear: 2025,
		TypeID:            globalType.GetID(),
		TypeCode:          globalCode,
		Items: []*plateauapi.GenericDatasetItem{
			{
				ID:       plateauapi.NewID(lodstatCode, plateauapi.TypeDatasetItem),
				Format:   plateauapi.DatasetFormatMvt,
				Name:     lodstatName,
				URL:      url,
				Layers:   []string{"lodstat"},
				ParentID: plateauapi.NewID(lodstatCode, plateauapi.TypeDataset),
			},
		},
	}

	res = append(res, dataset)
	return
}

// filterDatasetTypesWithoutDatasets filters out DatasetTypes that have no corresponding Datasets
func filterDatasetTypesWithoutDatasets(datasetTypes plateauapi.DatasetTypes, datasets plateauapi.Datasets) (filtered plateauapi.DatasetTypes, warning []string) {
	filtered = make(plateauapi.DatasetTypes)

	for category, types := range datasetTypes {
		var filteredTypes []plateauapi.DatasetType
		categoryDatasets := datasets[category]

		for _, dt := range types {
			// Check if there are any datasets for this dataset type
			hasDataset := false
			for _, ds := range categoryDatasets {
				if ds.GetTypeID() == dt.GetID() {
					hasDataset = true
					break
				}
			}

			if hasDataset {
				filteredTypes = append(filteredTypes, dt)
			} else {
				warning = append(warning, fmt.Sprintf("dataset type %s (%s) has no datasets, filtering out", dt.GetCode(), dt.GetName()))
			}
		}

		if len(filteredTypes) > 0 {
			filtered[category] = filteredTypes
		}
	}

	return
}
