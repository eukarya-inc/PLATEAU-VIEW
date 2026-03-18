package datacatalogv3

import (
	"fmt"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/samber/lo"
)

func (ft FeatureTypes) ToDatasetTypes(specs []plateauapi.PlateauSpec) plateauapi.DatasetTypes {
	res := make(plateauapi.DatasetTypes)
	res[plateauapi.DatasetTypeCategoryPlateau] = lo.FlatMap(ft.Plateau, func(f FeatureType, _ int) []plateauapi.DatasetType {
		// Skip derived feature types (e.g., bldg2) - they are merged into base types
		if IsDerivedFeatureType(f.Code) {
			return nil
		}
		return lo.FilterMap(specs, func(s plateauapi.PlateauSpec, _ int) (plateauapi.DatasetType, bool) {
			if s.MajorVersion < f.MinSpecMajor {
				return nil, false
			}
			return f.ToPlateauDatasetType(s), true
		})
	})
	res[plateauapi.DatasetTypeCategoryRelated] = lo.Map(ft.Related, func(f FeatureType, _ int) plateauapi.DatasetType {
		return f.ToRelatedDatasetType()
	})
	res[plateauapi.DatasetTypeCategoryGeneric] = lo.Map(ft.Generic, func(f FeatureType, _ int) plateauapi.DatasetType {
		return f.ToGenericDatasetType()
	})
	return res
}

func (f *FeatureType) ToPlateauDatasetType(spec plateauapi.PlateauSpec) *plateauapi.PlateauDatasetType {
	return &plateauapi.PlateauDatasetType{
		Category:      plateauapi.DatasetTypeCategoryPlateau,
		ID:            plateauDatasetTypeID(f.Code, spec.MajorVersion),
		Name:          f.Name,
		Code:          f.Code,
		Flood:         f.Flood,
		PlateauSpecID: spec.ID,
		Year:          spec.Year,
		Order:         f.Order,
	}
}

func (f *FeatureType) ToRelatedDatasetType() *plateauapi.RelatedDatasetType {
	return &plateauapi.RelatedDatasetType{
		Category: plateauapi.DatasetTypeCategoryRelated,
		ID:       plateauapi.NewID(f.Code, plateauapi.TypeDatasetType),
		Name:     f.Name,
		Code:     f.Code,
		Order:    f.Order,
	}
}

func (f *FeatureType) ToGenericDatasetType() *plateauapi.GenericDatasetType {
	return &plateauapi.GenericDatasetType{
		Category: plateauapi.DatasetTypeCategoryGeneric,
		ID:       plateauapi.NewID(f.Code, plateauapi.TypeDatasetType),
		Name:     f.Name,
		Code:     f.Code,
		Order:    f.Order,
	}
}

func (ft FeatureTypes) LayerNames() map[string]LayerNames {
	res := map[string]LayerNames{}
	for _, ft := range ft.Plateau {
		namesForLOD := map[int][]string{}
		for lod, layers := range ft.MVTLayerNamesForLOD {
			namesForLOD[lod] = layers
		}

		res[ft.Code] = LayerNames{
			Name:        ft.MVTLayerName,
			NamesForLOD: namesForLOD,
			Prefix:      ft.MVTLayerNamePrefix,
			PreferDef:   ft.UseCategoryAsMVTLayer,
		}
	}
	return res
}

func plateauDatasetTypeID(code string, specMajorVersion int) plateauapi.ID {
	return plateauapi.NewID(fmt.Sprintf("%s_%d", code, specMajorVersion), plateauapi.TypeDatasetType)
}
