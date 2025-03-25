package plateaucms

import (
	"context"
	"errors"
	"fmt"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/rerror"
)

type SpecStore interface {
	PlateauSpecs(context.Context) ([]PlateauSpec, error)
}

var _ SpecStore = &CMS{}

type PlateauSpec struct {
	ID              string `json:"id" cms:"id,text"`
	MajorVersion    int    `json:"major_version" cms:"major_version,integer"`
	Year            int    `json:"year" cms:"year,integer"`
	MaxMinorVersion int    `json:"max_minor_version" cms:"max_minor_version,integer"`
	FMEURL          string `json:"fme_url" cms:"fme_url,text"`
	AttrList        string `json:"attr_list" cms:"-"`
}

func (s PlateauSpec) MinorVersions() []string {
	return minorVersionsFromMax(s.MajorVersion, s.MaxMinorVersion)
}

func minorVersionsFromMax(major, max int) []string {
	res := make([]string, 0, max)
	for i := 0; i <= max; i++ {
		res = append(res, fmt.Sprintf("%d.%d", major, i))
	}
	return res
}

func (h *CMS) PlateauSpecs(ctx context.Context) ([]PlateauSpec, error) {
	if h.cmsSysProject == "" {
		return nil, rerror.ErrNotFound
	}

	items, err := h.cmsMain.GetItemsByKeyInParallel(ctx, h.cmsSysProject, plateauSpecModel, true, 100)
	if err != nil || items == nil {
		if errors.Is(err, cms.ErrNotFound) || items == nil {
			return nil, rerror.ErrNotFound
		}
		return nil, rerror.ErrInternalBy(fmt.Errorf("plateaucms: failed to get plateau-spec: %w", err))
	}

	all := make([]PlateauSpec, 0, len(items.Items))
	for _, item := range items.Items {
		m := PlateauSpec{}
		item.Unmarshal(&m)

		m.AttrList = valueToAssetURL(item.FieldByKey("attr_list").GetValue())

		all = append(all, m)
	}

	return all, nil
}
