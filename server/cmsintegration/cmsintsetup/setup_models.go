package cmsintsetup

import (
	"context"
	"errors"
	"fmt"

	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"github.com/reearth/reearthx/util"
	"github.com/samber/lo"
)

type SetupConfig struct {
	Type    string `json:"type"`
	Project string `json:"project"`
}

func SetupModels(ctx context.Context, s *Services, i SetupConfig) error {
	switch i.Type {
	case "system":
		return SetupSystemModel(ctx, s, i)
	case "plateau":
		return SetupPlateauModels(ctx, s, i)
	case "city-empty":
		return SetupEmptyCityModel(ctx, s, i)
	case "city":
		return SetupCityModel(ctx, s, i)
	case "feature":
		return SetupFeatureModel(ctx, s, i)
	case "geospatialjp-index":
		return SetupGeospatialjpIndexModel(ctx, s, i)
	case "geospatialjp-data":
		return SetupGeospatialjpDataModel(ctx, s, i)
	case "related":
		return SetupRelatedModel(ctx, s, i)
	case "generic":
		return SetupGenericModel(ctx, s, i)
	case "tiles":
		return SetupTilesModel(ctx, s, i)
	}

	return nil
}

func SetupPlateauModels(ctx context.Context, s *Services, i SetupConfig) error {
	return util.Try(
		func() error { return SetupEmptyCityModel(ctx, s, i) },
		func() error { return SetupFeatureModel(ctx, s, i) },
		func() error { return SetupGeospatialjpIndexModel(ctx, s, i) },
		func() error { return SetupGeospatialjpDataModel(ctx, s, i) },
		func() error { return SetupRelatedModel(ctx, s, i) },
		func() error { return SetupGenericModel(ctx, s, i) },
		func() error { return SetupCityModel(ctx, s, i) },
		func() error { return SetupTilesModel(ctx, s, i) },
	)
}

func SetupSystemModel(ctx context.Context, s *Services, i SetupConfig) error {
	models, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
		{Key: "workspaces", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
		{Key: "plateau-spec", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
		{Key: "plateau-features", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
		{Key: "plateau-dataset-types", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
	})
	if err != nil || len(models) == 0 {
		return fmt.Errorf("failed to create model sys: %w", err)
	}

	for _, ft := range defaultFeatureTypes {
		item := &cms.Item{}
		cms.Marshal(ft, item)
		if _, err := s.CMS.CreateItemByKey(ctx, i.Project, "plateau-features", item.Fields, item.MetadataFields); err != nil {
			return fmt.Errorf("failed to create item %s: %w", ft.Code, err)
		}
	}

	for _, dt := range defaultDataTypes {
		item := &cms.Item{}
		cms.Marshal(dt, item)
		if _, err := s.CMS.CreateItemByKey(ctx, i.Project, "plateau-dataset-types", item.Fields, item.MetadataFields); err != nil {
			return fmt.Errorf("failed to create item %s: %w", dt.Code, err)
		}
	}

	return nil
}

func SetupEmptyCityModel(ctx context.Context, s *Services, i SetupConfig) error {
	_, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
		{Key: "city", Value: nil},
	})
	return err
}

func SetupCityModel(ctx context.Context, s *Services, i SetupConfig) error {
	_, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
		{Key: "city", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
	})
	return err
}

func SetupFeatureModel(ctx context.Context, s *Services, i SetupConfig) error {
	features, err := s.PCMS.PlateauFeatureTypes(ctx)
	if err != nil {
		return fmt.Errorf("failed to get feature types: %w", err)
	}

	// TODO: create a group

	errs := make([]error, 0, len(features))
	for _, f := range features {
		_, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
			{Key: "plateau-" + f.Code, Value: &cms.Schema{
				Fields: []cms.SchemaField{
					// TODO
				},
			}},
		})
		if err != nil {
			errs = append(errs, fmt.Errorf("failed to create model %s: %w", f.Code, err))
		}
	}
	return errors.Join(errs...)
}

func SetupGeospatialjpIndexModel(ctx context.Context, s *Services, i SetupConfig) error {
	// TODO: create a group

	_, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
		{Key: "geospatialjp-index", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
	})
	return err
}

func SetupGeospatialjpDataModel(ctx context.Context, s *Services, i SetupConfig) error {
	_, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
		{Key: "plateau-geospatialjp-data", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
	})
	return err
}

func SetupRelatedModel(ctx context.Context, s *Services, i SetupConfig) error {
	// TODO: create a group

	_, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
		{Key: "plateau-related", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
	})
	return err
}

func SetupGenericModel(ctx context.Context, s *Services, i SetupConfig) error {
	// TODO: create a group

	_, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
		{Key: "plateau-generic", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
	})
	return err
}

func SetupTilesModel(ctx context.Context, s *Services, i SetupConfig) error {
	_, err := setupModels(ctx, s.CMS, i.Project, []lo.Entry[string, *cms.Schema]{
		{Key: "tiles", Value: &cms.Schema{
			Fields: []cms.SchemaField{
				// TODO
			},
		}},
	})
	return err
}

func setupModels(ctx context.Context, _ cms.Interface, _ string, models []lo.Entry[string, *cms.Schema]) ([]*cms.Model, error) {
	res := make([]*cms.Model, 0, len(models))
	errs := make([]error, 0, len(models))
	for _, m := range models {
		// TODO
		log.Debugfc(ctx, "creating model %s", m.Key)
	}

	var err error
	if len(errs) > 0 {
		err = errors.Join(errs...)
	}
	return res, err
}

var defaultFeatureTypes = plateaucms.PlateauFeatureTypeList{
	{
		Code: "bldg",
		Name: "建築物モデル",
		QC:   true,
		Conv: true,
	},
	// TODO
}

var defaultDataTypes = plateaucms.DatasetTypeList{
	{
		Code:     "usecase",
		Name:     "ユースケース",
		Category: plateaucms.DatasetCategoryGeneric,
	},
	{
		Code:     "global",
		Name:     "全球ケース",
		Category: plateaucms.DatasetCategoryGeneric,
	},
	{
		Code:     "sample",
		Name:     "サンプルデータ",
		Category: plateaucms.DatasetCategoryGeneric,
	},
	{
		Code:     "shelter",
		Name:     "避難施設情報",
		Category: plateaucms.DatasetCategoryRelated,
	},
	{
		Code:     "park",
		Name:     "公園情報",
		Category: plateaucms.DatasetCategoryRelated,
	},
	{
		Code:     "landmark",
		Name:     "ランドマーク情報",
		Category: plateaucms.DatasetCategoryRelated,
	},
	{
		Code:     "station",
		Name:     "鉄道駅情報",
		Category: plateaucms.DatasetCategoryRelated,
	},
	{
		Code:     "railway",
		Name:     "鉄道情報",
		Category: plateaucms.DatasetCategoryRelated,
	},
	{
		Code:     "emergency_route",
		Name:     "緊急輸送道路情報",
		Category: plateaucms.DatasetCategoryRelated,
	},
	{
		Code:     "border",
		Name:     "行政界情報",
		Category: plateaucms.DatasetCategoryRelated,
	},
	{
		Code:     "city",
		Name:     "自治体データ",
		Category: plateaucms.DatasetCategoryGeneric,
	},
}
