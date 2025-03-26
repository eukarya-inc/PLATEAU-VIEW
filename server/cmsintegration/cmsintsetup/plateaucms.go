package cmsintsetup

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	"github.com/eukarya-inc/reearth-plateauview/server/cmsintegration/cmsintegrationcommon"
	cms "github.com/reearth/reearth-cms-api/go"
)

type PlateauCMS struct {
	CMS           cms.Interface
	cacheBasePath string
}

func NewPlateauCMS(cms cms.Interface, cacheBasePath string) *PlateauCMS {
	return &PlateauCMS{CMS: cms, cacheBasePath: cacheBasePath}
}

func (c *PlateauCMS) GetAllCities(ctx context.Context, prj string, featureTypes []string) ([]*cmsintegrationcommon.CityItem, error) {
	return getAllItems(ctx, c.CMS, prj, cmsintegrationcommon.ModelPrefix+cmsintegrationcommon.CityModel, c.cacheBasePath, func(item *cms.Item) (*cmsintegrationcommon.CityItem, bool, error) {
		city := cmsintegrationcommon.CityItemFrom(item, featureTypes)
		return city, city != nil, nil
	})
}

func (c *PlateauCMS) GetAllRelated(ctx context.Context, prj string, relatedDataTypes []string) ([]*cmsintegrationcommon.RelatedItem, error) {
	return getAllItems(ctx, c.CMS, prj, cmsintegrationcommon.ModelPrefix+cmsintegrationcommon.RelatedModel, c.cacheBasePath, func(item *cms.Item) (*cmsintegrationcommon.RelatedItem, bool, error) {
		city := cmsintegrationcommon.RelatedItemFrom(item, relatedDataTypes)
		return city, city != nil, nil
	})
}

func getAllItems[T any](ctx context.Context, c cms.Interface, prj, model, cacheBase string, mapper func(*cms.Item) (T, bool, error)) ([]T, error) {
	key := fmt.Sprintf("cms_allitems_%s_%s", prj, model)
	items := &cms.Items{}

	if found, err := findCache(cacheBase, key, items); err != nil {
		return nil, fmt.Errorf("failed to find cache: %w", err)
	} else if !found {
		items2, err := c.GetItemsByKeyInParallel(ctx, prj, model, true, 0)
		if err != nil {
			return nil, err
		}

		if err := saveCache(cacheBase, key, items2); err != nil {
			return nil, fmt.Errorf("failed to save cache: %w", err)
		}

		items = items2
	}

	res := make([]T, 0, len(items.Items))
	for _, item := range items.Items {
		t, ok, err := mapper(&item)
		if err != nil {
			return nil, err
		}
		if !ok {
			continue
		}
		res = append(res, t)
	}

	return res, nil
}

func findCache(base, key string, t any) (bool, error) {
	if base == "" || key == "" {
		return false, nil
	}
	f, err := os.Open(filepath.Join(base, key))
	if err != nil {
		if os.IsNotExist(err) {
			return false, nil
		}
		return false, err
	}

	defer f.Close()
	if err := json.NewDecoder(f).Decode(t); err != nil {
		return false, err
	}

	return true, nil
}

func saveCache(base, key string, content any) error {
	if base == "" || key == "" {
		return nil
	}
	_ = os.MkdirAll(base, 0755)
	f, err := os.Create(filepath.Join(base, key))
	if err != nil {
		return err
	}

	defer f.Close()
	if err := json.NewEncoder(f).Encode(content); err != nil {
		return err
	}

	return nil
}
