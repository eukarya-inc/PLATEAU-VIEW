package cmsintrelated

import (
	"context"

	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
)

type plateauCMSMock struct {
	PCMS
	plateauSpecs func(ctx context.Context) ([]plateaucms.PlateauSpec, error)
}

func (p *plateauCMSMock) PlateauSpecs(ctx context.Context) ([]plateaucms.PlateauSpec, error) {
	return p.plateauSpecs(ctx)
}

func (p *plateauCMSMock) PlateauFeatureTypes(ctx context.Context) (plateaucms.PlateauFeatureTypeList, error) {
	return []plateaucms.PlateauFeatureType{
		{
			Code: "bldg",
			Name: "建築物モデル",
			QC:   true,
			Conv: true,
		},
		{
			Code: "tran",
			Name: "交通モデル（道路）",
			QC:   true,
			Conv: true,
		},
		{
			Code: "luse",
			Name: "土地利用モデル",
			QC:   true,
			Conv: true,
		},
		{
			Code:      "fld",
			Name:      "洪水浸水想定区域モデル",
			QC:        true,
			Conv:      true,
			UseGroups: true,
		},
	}, nil
}

func (p *plateauCMSMock) DatasetTypes(ctx context.Context) (plateaucms.DatasetTypeList, error) {
	return plateaucms.DatasetTypeList{
		{
			Code:     "shelter",
			Name:     "SHELTER",
			Category: plateaucms.DatasetCategoryRelated,
		},
		{
			Code:     "park",
			Name:     "PARK",
			Category: plateaucms.DatasetCategoryRelated,
		},
		{
			Code:     "landmark",
			Name:     "LANDMARK",
			Category: plateaucms.DatasetCategoryRelated,
		},
		{
			Code:     "station",
			Name:     "STATION",
			Category: plateaucms.DatasetCategoryRelated,
		},
		{
			Code:     "railway",
			Name:     "RAILWAY",
			Category: plateaucms.DatasetCategoryRelated,
		},
		{
			Code:     "emergency_route",
			Name:     "EMERGENCY_ROUTE",
			Category: plateaucms.DatasetCategoryRelated,
		},
		{
			Code:     "border",
			Name:     "BORDER",
			Category: plateaucms.DatasetCategoryRelated,
		},
	}, nil
}

func (p *plateauCMSMock) Metadata(ctx context.Context, prj string, findDataCatalog, useDefault bool) (plateaucms.Metadata, plateaucms.MetadataList, error) {
	return plateaucms.Metadata{
		Converter: "fme",
	}, nil, nil
}
