package datacatalogv3

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/eukarya-inc/reearth-plateauview/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/log"
	"github.com/samber/lo"
)

const cacheDir = "cache"
const cachePrefix = "cache-datacatalogv3-"
const gespatialjpDatasetURL = "https://www.geospatial.jp/ckan/dataset/"
const datasetTypeNameUsecase = "ユースケース"
const datasetTypeNameCity = "自治体データ"

type CMS struct {
	cms      cms.Interface
	pcms     *plateaucms.CMS
	project  string
	year     int
	plateau  bool
	cache    bool
	cacheDir string
}

type CMSOpts struct {
	CMS     cms.Interface
	PCMS    *plateaucms.CMS
	Year    int
	Plateau bool
	Project string
	Cache   bool
}

func NewCMS(opts CMSOpts) *CMS {
	return &CMS{
		cms:      opts.CMS,
		pcms:     opts.PCMS,
		project:  opts.Project,
		year:     opts.Year,
		plateau:  opts.Plateau,
		cache:    opts.Cache,
		cacheDir: filepath.Join(cacheDir, cachePrefix+opts.Project),
	}
}

func (c *CMS) GetAll(ctx context.Context) (*AllData, error) {
	cmsinfo, err := c.GetCMSInfo(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get CMS info: %w", err)
	}
	if cmsinfo == nil {
		log.Debugfc(ctx, "datacatalogv3: metadata not found: %s", c.project)
		return nil, nil
	}

	all := AllData{
		Name:    c.project,
		Year:    c.year,
		CMSInfo: *cmsinfo,
	}

	specs, err := c.GetPlateauSpecs(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get plateau specs: %w", err)
	}

	featureTypes, err := c.GetFeatureTypes(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get feature types: %w", err)
	}

	all.PlateauSpecs = specs
	all.FeatureTypes = featureTypes

	cityItemsChan := lo.Async2(func() ([]*CityItem, error) {
		return c.GetCityItems(ctx, c.project, featureTypes.Plateau)
	})

	relatedItemsChan := lo.Async2(func() ([]*RelatedItem, error) {
		return c.GetRelatedItems(ctx, c.project, featureTypes.Related)
	})

	genericItemsChan := lo.Async2(func() ([]*GenericItem, error) {
		return c.GetGenericItems(ctx, c.project)
	})

	sampleItemsChan := lo.Async2(func() ([]*PlateauFeatureItem, error) {
		return c.GetSampleItems(ctx, c.project)
	})

	geospatialjpDataItemsChan := lo.Async2(func() ([]*GeospatialjpDataItem, error) {
		return c.GetGeospatialjpDataItems(ctx, c.project)
	})

	featureItemsChans := make([]<-chan lo.Tuple3[string, []*PlateauFeatureItem, error], 0, len(all.FeatureTypes.Plateau))
	for _, featureType := range all.FeatureTypes.Plateau {
		featureType := featureType

		if featureType.MinYear > 0 && c.year < featureType.MinYear {
			continue
		}

		featureItemsChan := lo.Async3(func() (string, []*PlateauFeatureItem, error) {
			res, err := c.GetPlateauItems(ctx, c.project, featureType.Code)
			return featureType.Code, res, err
		})
		featureItemsChans = append(featureItemsChans, featureItemsChan)
	}

	if res := <-cityItemsChan; res.B != nil {
		return nil, fmt.Errorf("failed to get city items: %w", res.B)
	} else {
		all.City = res.A
	}

	if res := <-relatedItemsChan; c.plateau && res.B != nil {
		return nil, fmt.Errorf("failed to get related items: %w", res.B)
	} else {
		all.Related = res.A
	}

	if res := <-genericItemsChan; res.B != nil {
		return nil, fmt.Errorf("failed to get generic items: %w", res.B)
	} else {
		all.Generic = res.A
	}

	if res := <-sampleItemsChan; c.plateau && res.B != nil {
		return nil, fmt.Errorf("failed to get sample items: %w", res.B)
	} else {
		all.Sample = res.A
	}

	if res := <-geospatialjpDataItemsChan; c.plateau && res.B != nil {
		return nil, fmt.Errorf("failed to get geospatialjp data items: %w", res.B)
	} else {
		all.GeospatialjpDataItems = res.A
	}

	all.Plateau = make(map[string][]*PlateauFeatureItem)
	for _, featureItemsChan := range featureItemsChans {
		if res := <-featureItemsChan; c.plateau && res.C != nil {
			return nil, fmt.Errorf("failed to get feature items (%s): %w", res.A, res.C)
		} else {
			for _, d := range res.B {
				if d.Sample {
					all.Sample = append(all.Sample, d)
				} else {
					all.Plateau[res.A] = append(all.Plateau[res.A], d)
				}
			}
		}
	}

	return &all, nil
}

func (c *CMS) GetCityItems(ctx context.Context, project string, featureTypes []FeatureType) ([]*CityItem, error) {
	items, err := getItemsAndConv(
		c.cms, ctx, project, modelPrefix+cityModel,
		func(i cms.Item) *CityItem {
			return CityItemFrom(&i, featureTypes)
		},
	)

	// TODO: dynamic year
	for _, item := range items {
		if item.Year == "" {
			item.Year = "令和5年度"
		}
	}

	return items, err
}

func (c *CMS) GetPlateauItems(ctx context.Context, project, feature string) ([]*PlateauFeatureItem, error) {
	cacheKey := fmt.Sprintf("plateau_%s_%s", project, feature)
	if c.cache {
		if items, err := loadCache[[]*PlateauFeatureItem](
			c.cacheDir, cacheKey,
		); err != nil {
			return nil, err
		} else if items != nil {
			return items, nil
		}
	}

	items, err := getItemsAndConv(
		c.cms, ctx, project, modelPrefix+feature,
		func(i cms.Item) *PlateauFeatureItem {
			return PlateauFeatureItemFrom(&i, feature)
		},
	)

	if err == nil && c.cache {
		if err := saveCache(c.cacheDir, cacheKey, items); err != nil {
			return nil, err
		}
	}

	return items, err
}

func (c *CMS) GetRelatedItems(ctx context.Context, project string, featureTypes []FeatureType) ([]*RelatedItem, error) {
	cacheKey := fmt.Sprintf(
		"related_%s_%s",
		project,
		strings.Join(lo.Map(
			featureTypes,
			func(b FeatureType, _ int) string {
				return b.Code
			},
		), "-"),
	)

	if c.cache {
		if items, err := loadCache[[]*RelatedItem](
			c.cacheDir, cacheKey,
		); err != nil {
			return nil, err
		} else if items != nil {
			return items, nil
		}
	}

	items, err := getItemsAndConv(
		c.cms, ctx, project, modelPrefix+relatedModel,
		func(i cms.Item) *RelatedItem {
			return RelatedItemFrom(&i, featureTypes)
		},
	)

	if err == nil && c.cache {
		if err := saveCache(c.cacheDir, cacheKey, items); err != nil {
			return nil, err
		}
	}

	return items, err
}

func (c *CMS) GetGenericItems(ctx context.Context, project string) ([]*GenericItem, error) {
	cacheKey := fmt.Sprintf("generic_%s", project)
	if c.cache {
		if items, err := loadCache[[]*GenericItem](
			c.cacheDir, cacheKey,
		); err != nil {
			return nil, err
		} else if items != nil {
			return items, nil
		}
	}

	items, err := getItemsAndConv(
		c.cms, ctx, project, modelPrefix+genericModel,
		func(i cms.Item) *GenericItem {
			return GenericItemFrom(&i)
		},
	)

	for _, item := range items {
		if item.Category == "" {
			if c.plateau {
				item.Category = datasetTypeNameUsecase
			} else {
				item.Category = datasetTypeNameCity
			}
		}
	}

	if err == nil && c.cache {
		if err := saveCache(c.cacheDir, cacheKey, items); err != nil {
			return nil, err
		}
	}

	return items, err
}

func (c *CMS) GetSampleItems(ctx context.Context, project string) ([]*PlateauFeatureItem, error) {
	cacheKey := fmt.Sprintf("sample_%s", project)
	if c.cache {
		if items, err := loadCache[[]*PlateauFeatureItem](
			c.cacheDir, cacheKey,
		); err != nil {
			return nil, err
		} else if items != nil {
			return items, nil
		}
	}

	items, err := getItemsAndConv(
		c.cms, ctx, project, modelPrefix+sampleModel,
		func(i cms.Item) *PlateauFeatureItem {
			return PlateauFeatureItemFrom(&i, "")
		},
	)

	if err == nil && c.cache {
		if err := saveCache(c.cacheDir, cacheKey, items); err != nil {
			return nil, err
		}
	}

	return items, err
}

func (c *CMS) GetGeospatialjpDataItems(ctx context.Context, project string) ([]*GeospatialjpDataItem, error) {
	cacheKey := fmt.Sprintf("geospatialjp_%s", project)
	if c.cache {
		if items, err := loadCache[[]*GeospatialjpDataItem](
			c.cacheDir, cacheKey,
		); err != nil {
			return nil, err
		} else if items != nil {
			return items, nil
		}
	}

	items, err := getItemsAndConv(
		c.cms, ctx, project, modelPrefix+geospatialjpDataModel,
		func(i cms.Item) *GeospatialjpDataItem {
			return GeospatialjpDataItemFrom(&i)
		},
	)

	if err == nil && c.cache {
		if err := saveCache(c.cacheDir, cacheKey, items); err != nil {
			return nil, err
		}
	}

	return items, err
}

func (c *CMS) GetCMSInfo(ctx context.Context) (*CMSInfo, error) {
	metadata := plateaucms.GetAllCMSMetadataFromContext(ctx)
	if len(metadata) == 0 {
		return nil, nil
	}

	md, ok := metadata.FindMetadata(c.project, true, false)
	if !ok {
		return nil, fmt.Errorf("metadata not found")
	}

	modelIDs, err := c.GetModelIDs(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get model IDs: %w", err)
	}

	return &CMSInfo{
		CMSURL:      md.CMSURL,
		WorkspaceID: md.WorkspaceID,
		ProjectID:   md.ProjectID,
		ModelIDMap:  modelIDs,
	}, nil
}

func (c *CMS) GetModelIDs(ctx context.Context) (ModelIDMap, error) {
	models, err := c.cms.GetModels(ctx, c.project)
	if err != nil {
		return nil, fmt.Errorf("failed to get models: %w", err)
	}

	res := make(ModelIDMap, len(models.Models))
	for _, model := range models.Models {
		if strings.HasPrefix(model.Key, modelPrefix) {
			res[strings.TrimPrefix(model.Key, modelPrefix)] = model.ID
		}
	}

	return res, nil
}

func (c *CMS) GetPlateauSpecs(ctx context.Context) ([]plateauapi.PlateauSpecSimple, error) {
	return getPlateauSpecs(ctx, c.pcms, c.year)
}

func (c *CMS) GetFeatureTypes(ctx context.Context) (FeatureTypes, error) {
	return getFeatureTypes(ctx, c.pcms)
}

func getItemsAndConv[T any](cms cms.Interface, ctx context.Context, project, model string, conv func(cms.Item) *T) ([]*T, error) {
	items, err := cms.GetItemsByKeyInParallel(ctx, project, model, true, 100)
	if err != nil && model != modelPrefix+sampleModel { // sample is optional
		log.Warnfc(ctx, "datacatalogv3: failed to get items (%s/%s): %v", project, model, err)
	}
	if items == nil {
		return nil, nil
	}

	res := make([]*T, 0, len(items.Items))
	for _, item := range items.Items {
		if c := conv(item); c != nil {
			res = append(res, c)
		}
	}

	return res, nil
}

func loadCache[T any](cachePath, key string) (t T, _ error) {
	_ = os.MkdirAll(cachePath, 0755)

	f, err := os.Open(filepath.Join(cachePath, key+".json"))
	if err != nil {
		if os.IsNotExist(err) {
			return
		}
		return t, fmt.Errorf("failed to open cache file: %w", err)
	}

	defer f.Close()

	var v T
	if err = json.NewDecoder(f).Decode(&v); err != nil {
		return
	}

	return v, nil
}

func saveCache(cachePath, key string, content any) error {
	_ = os.MkdirAll(cachePath, 0755)

	f, err := os.Create(filepath.Join(cachePath, key+".json"))
	if err != nil {
		return fmt.Errorf("failed to create cache file: %w", err)
	}

	defer f.Close()

	if err = json.NewEncoder(f).Encode(content); err != nil {
		return fmt.Errorf("failed to encode cache content: %w", err)
	}

	return nil
}
