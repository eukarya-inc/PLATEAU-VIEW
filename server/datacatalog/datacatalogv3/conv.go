package datacatalogv3

import (
	"fmt"

	"github.com/eukarya-inc/reearth-plateauview/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

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

	// wards
	for _, ft := range res.DatasetTypes[plateauapi.DatasetTypeCategoryPlateau] {
		wards, w := getWards(all.Plateau[ft.GetCode()], ic)
		warning = append(warning, w...)
		ic.AddWards(wards)
		res.Areas.Append(
			plateauapi.AreaTypeWard,
			lo.Map(wards, func(w *plateauapi.Ward, _ int) plateauapi.Area { return w }),
		)
	}

	// plateau
	plateauDatasetTypes := res.DatasetTypes.CodeMap(plateauapi.DatasetTypeCategoryPlateau)
	plateauFeatureTypes := all.FeatureTypes.PlateauMap()
	for _, dt := range res.DatasetTypes[plateauapi.DatasetTypeCategoryPlateau] {
		datasets, w := convertPlateau(
			all.Plateau[dt.GetCode()],
			dt.GetCode(),
			res.PlateauSpecs,
			plateauDatasetTypes,
			plateauFeatureTypes,
			ic,
		)
		warning = append(warning, w...)
		res.Datasets.Append(plateauapi.DatasetTypeCategoryPlateau, datasets)
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

	// citygml
	{
		var w []string
		res.CityGML, w = toCityGMLs(all, ic.regYear)
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
	specs []plateauapi.PlateauSpec,
	dts map[string]plateauapi.DatasetType,
	fts map[string]*FeatureType,
	ic *internalContext,
) ([]plateauapi.Dataset, []string) {
	res, w := convertPlateauRaw(
		items, true, code, specs, dts, fts, ic,
	)
	return plateauapi.ToDatasets(res), w
}

func convertPlateauRaw(
	items []*PlateauFeatureItem,
	ignoreSample bool,
	code string,
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

		dt := dts[ftcode]
		ft := fts[ftcode]

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
