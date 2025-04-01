package cmsintsetup

import (
	"context"

	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
)

type plateauCMSMock struct {
	plateaucms.FeatureTypeStore
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
	return defaultDataTypes, nil
}
