package plateaucms

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"strings"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/rerror"
)

type FeatureTypeStore interface {
	PlateauFeatureTypes(context.Context) (PlateauFeatureTypeList, error)
	DatasetTypes(context.Context) (DatasetTypeList, error)
}

var _ FeatureTypeStore = &CMS{}

type PlateauFeatureType struct {
	Code                      string         `json:"code" cms:"code,text"`
	Name                      string         `json:"name" cms:"name,text"`
	GroupName                 string         `json:"groupName" cms:"group_name,text"`
	MinSpecMajor              int            `json:"minSpecMajor" cms:"min_spec_major,integer"`
	MinYear                   int            `json:"minYear" cms:"min_year,integer"`
	Order                     int            `json:"order" cms:"order,integer"`
	QC                        bool           `json:"qc" cms:"qc,boolean"`
	Conv                      bool           `json:"conv" cms:"conv,boolean"`
	MVTLayers                 []string       `json:"layers" cms:"mvt_layers,text"`
	MVTLayersLOD0             []string       `json:"layersLod0" cms:"mvt_layers_lod0,text"`
	MVTLayersLOD1             []string       `json:"layersLod1" cms:"mvt_layers_lod1,text"`
	MVTLayersLOD2             []string       `json:"layersLod2" cms:"mvt_layers_lod2,text"`
	MVTLayerNamePrefix        string         `json:"mvtLayerNamePrefix" cms:"mvt_layer_name_prefix,text"`
	UseCategoryNameAsMVTLayer bool           `json:"useCategoryAsMVTLayer" cms:"use_category_as_mvt_layer,boolean"`
	Flood                     bool           `json:"flood" cms:"flood,boolean"`
	HideTexture               bool           `json:"hideTexture" cms:"hide_texture,boolean"`
	UseGroups                 bool           `json:"useGroups" cms:"use_groups,boolean"`
	FlowQCV                   map[int]string `json:"flowQcV" cms:"-"`
	FlowConvV                 map[int]string `json:"flowConvV" cms:"-"`
	FlowQC                    string         `json:"flowQc" cms:"flow_qc,text"`
	FlowConv                  string         `json:"flowConv" cms:"flow_conv,text"`
}

func PlateauFeatureTypeFrom(item *cms.Item) *PlateauFeatureType {
	if item == nil {
		return nil
	}

	res := &PlateauFeatureType{}
	item.Unmarshal(res)

	for _, f := range item.Fields {
		const qck = "flow_qc_v"
		const convk = "flow_conv_v"

		if strings.HasPrefix(f.Key, qck) {
			if res.FlowQCV == nil {
				res.FlowQCV = map[int]string{}
			}
			if v, _ := strconv.Atoi(strings.TrimPrefix(f.Key, qck)); v > 0 {
				if t, _ := f.Value.(string); t != "" {
					res.FlowQCV[v] = t
				}
			}
		}

		if strings.HasPrefix(f.Key, convk) {
			if res.FlowConvV == nil {
				res.FlowConvV = map[int]string{}
			}
			if v, _ := strconv.Atoi(strings.TrimPrefix(f.Key, convk)); v > 0 {
				if t, _ := f.Value.(string); t != "" {
					res.FlowConvV[v] = t
				}
			}
		}
	}

	return res
}

func (f PlateauFeatureType) FlowQCTriggerID(v int) string {
	if t := f.FlowQCV[v]; t != "" {
		return t
	}
	return f.FlowQC
}

func (f PlateauFeatureType) FlowConvTriggerID(v int) string {
	if t := f.FlowConvV[v]; t != "" {
		return t
	}
	return f.FlowConv
}

type PlateauFeatureTypeList []PlateauFeatureType

func (f PlateauFeatureTypeList) GetByCode(code string) (PlateauFeatureType, bool) {
	for _, ft := range f {
		if ft.Code == code {
			return ft, true
		}
	}
	return PlateauFeatureType{}, false
}

func (f PlateauFeatureTypeList) Codes() []string {
	codes := make([]string, 0, len(f))
	for _, ft := range f {
		codes = append(codes, ft.Code)
	}
	return codes
}

const (
	DatasetCategoryRelated = "関連データセット"
	DatasetCategoryGeneric = "その他のデータセット"
)

type DatasetType struct {
	Code     string `json:"code" cms:"code,text"`
	Name     string `json:"name" cms:"name,text"`
	Category string `json:"category" cms:"category,select"`
}

func (f *PlateauFeatureType) MVTLayersOfLOD(lod int) []string {
	switch lod {
	case 0:
		return f.MVTLayersLOD0
	case 1:
		return f.MVTLayersLOD1
	case 2:
		return f.MVTLayersLOD2
	}
	return nil
}

type DatasetTypeList []DatasetType

func (f DatasetTypeList) Codes(cat string) []string {
	codes := make([]string, 0, len(f))
	for _, ft := range f {
		if ft.Category != cat {
			continue
		}
		codes = append(codes, ft.Code)
	}
	return codes
}

func (h *CMS) PlateauFeatureTypes(ctx context.Context) (PlateauFeatureTypeList, error) {
	if h.cmsSysProject == "" {
		return nil, rerror.ErrNotFound
	}

	items, err := h.cmsMain.GetItemsByKeyInParallel(ctx, h.cmsSysProject, plateauFeatureTypesModel, true, 100)
	if err != nil || items == nil {
		if errors.Is(err, cms.ErrNotFound) || items == nil {
			return nil, rerror.ErrNotFound
		}
		return nil, rerror.ErrInternalBy(fmt.Errorf("plateaucms: failed to get plateau-features: %w", err))
	}

	all := make([]PlateauFeatureType, 0, len(items.Items))
	for _, item := range items.Items {
		if m := PlateauFeatureTypeFrom(&item); m != nil {
			all = append(all, *m)
		}
	}

	return all, nil
}

func (h *CMS) DatasetTypes(ctx context.Context) (DatasetTypeList, error) {
	if h.cmsSysProject == "" {
		return nil, rerror.ErrNotFound
	}

	items, err := h.cmsMain.GetItemsByKeyInParallel(ctx, h.cmsSysProject, datasetTypesModel, true, 100)
	if err != nil || items == nil {
		if errors.Is(err, cms.ErrNotFound) || items == nil {
			return nil, rerror.ErrNotFound
		}
		return nil, rerror.ErrInternalBy(fmt.Errorf("plateaucms: failed to get plateau-features: %w", err))
	}

	all := make([]DatasetType, 0, len(items.Items))
	for _, item := range items.Items {
		m := DatasetType{}
		item.Unmarshal(&m)
		all = append(all, m)
	}

	return all, nil
}
