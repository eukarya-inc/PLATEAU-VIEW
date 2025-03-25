package datacatalogv3

import (
	"context"
	"fmt"
	"slices"
	"sort"

	"github.com/eukarya-inc/reearth-plateauview/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
)

func getPlateauSpecs(ctx context.Context, pcms plateaucms.SpecStore, maxYear int) ([]plateauapi.PlateauSpecSimple, error) {
	res, err := pcms.PlateauSpecs(ctx)
	if err != nil {
		return nil, err
	}

	res2 := make([]plateauapi.PlateauSpecSimple, 0, len(res))
	for _, r := range res {
		if r.Year > maxYear {
			continue
		}
		res2 = append(res2, toSimpleSpec(r))
	}

	return res2, nil
}

func toSimpleSpec(s plateaucms.PlateauSpec) plateauapi.PlateauSpecSimple {
	return plateauapi.PlateauSpecSimple{
		MajorVersion:  s.MajorVersion,
		Year:          s.Year,
		MinorVersions: s.MinorVersions(),
	}
}

func getFeatureTypes(ctx context.Context, pcms plateaucms.FeatureTypeStore) (ft FeatureTypes, _ error) {
	res, err := pcms.PlateauFeatureTypes(ctx)
	if err != nil {
		return FeatureTypes{}, fmt.Errorf("failed to get feature types: %w", err)
	}

	res2, err := pcms.DatasetTypes(ctx)
	if err != nil {
		return FeatureTypes{}, fmt.Errorf("failed to get dataset types: %w", err)
	}

	sort.SliceStable(res, func(i, j int) bool {
		return res[i].Order < res[j].Order
	})

	ft.Plateau = make([]FeatureType, 0, len(res))
	for i, r := range res {
		f := FeatureType{
			Code:                  r.Code,
			Name:                  r.Name,
			Order:                 i + 1,
			GroupName:             r.GroupName,
			MinSpecMajor:          r.MinSpecMajor,
			MinYear:               r.MinYear,
			Flood:                 r.Flood,
			MVTLayerName:          slices.Clone(r.MVTLayers),
			MVTLayerNamePrefix:    r.MVTLayerNamePrefix,
			UseCategoryAsMVTLayer: r.UseCategoryNameAsMVTLayer,
			HideTexture:           r.HideTexture,
		}

		if r.MinSpecMajor == 0 {
			f.MinSpecMajor = 3
		}

		if r.MVTLayersLOD0 != nil || r.MVTLayersLOD1 != nil || r.MVTLayersLOD2 != nil {
			f.MVTLayerNamesForLOD = map[int][]string{}
			if r.MVTLayersLOD0 != nil {
				f.MVTLayerNamesForLOD[0] = slices.Clone(r.MVTLayersLOD0)
			}
			if r.MVTLayersLOD1 != nil {
				f.MVTLayerNamesForLOD[1] = slices.Clone(r.MVTLayersLOD1)
			}
			if r.MVTLayersLOD2 != nil {
				f.MVTLayerNamesForLOD[2] = slices.Clone(r.MVTLayersLOD2)
			}
		}

		ft.Plateau = append(ft.Plateau, f)
	}

	for _, r := range res2 {
		f := FeatureType{
			Code: r.Code,
			Name: r.Name,
		}
		if r.Category == plateaucms.DatasetCategoryRelated {
			ft.Related = append(ft.Related, f)
		} else if r.Category == plateaucms.DatasetCategoryGeneric {
			ft.Generic = append(ft.Generic, f)
		}
	}

	for i, f := range ft.Related {
		f.Order = len(ft.Plateau) + i + 1
		ft.Related[i] = f
	}

	for i, f := range ft.Generic {
		f.Order = len(ft.Plateau) + len(ft.Related) + i + 1
		ft.Generic[i] = f
	}

	return
}
