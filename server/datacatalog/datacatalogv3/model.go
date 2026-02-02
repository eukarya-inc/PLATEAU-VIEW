package datacatalogv3

import (
	"fmt"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
)

type AllData struct {
	Name                  string
	Year                  int
	Host                  string
	PlateauSpecs          []plateauapi.PlateauSpecSimple
	FeatureTypes          FeatureTypes
	City                  []*CityItem
	Related               []*RelatedItem
	Generic               []*GenericItem
	Sample                []*PlateauFeatureItem
	Plateau               map[string][]*PlateauFeatureItem
	Flow                  map[string][]*PlateauFeatureItem // plateau-flow model items grouped by feature type
	FlowEnabled           bool                             // whether Flow datasets should be included in the catalog
	GeospatialjpDataItems []*GeospatialjpDataItem
	CMSInfo               CMSInfo
}

func (d *AllData) FindPlateauFeatureItemByCityID(ft, cityID string) *PlateauFeatureItem {
	for _, f := range d.Plateau[ft] {
		if f != nil && f.City == cityID {
			return f
		}
	}
	return nil
}

func (d *AllData) FindPlateauFeatureItemsByCityID(cityID string) (res []*PlateauFeatureItem) {
	for _, ft := range d.FeatureTypes.Plateau {
		for _, f := range d.Plateau[ft.Code] {
			if f != nil && f.City == cityID {
				res = append(res, f)
			}
		}
	}
	return
}

func (all *AllData) FeatureTypesOf(cityID string) (res []string) {
	for _, ft := range all.FeatureTypes.Plateau {
		if p := all.FindPlateauFeatureItemByCityID(ft.Code, cityID); p != nil && p.CityGML != "" {
			res = append(res, ft.Code)
		}
	}

	return res
}

func (d *AllData) FindGspatialjpDataItemByCityID(cityID string) *GeospatialjpDataItem {
	for _, i := range d.GeospatialjpDataItems {
		if i != nil && i.City == cityID {
			return i
		}
	}
	return nil
}

type FeatureTypes struct {
	Plateau []FeatureType
	Related []FeatureType
	Generic []FeatureType
}

func (ft FeatureTypes) PlateauMap() map[string]*FeatureType {
	res := make(map[string]*FeatureType)
	for _, f := range ft.Plateau {
		f := f
		res[f.Code] = &f
	}
	return res
}

func (ft FeatureTypes) FindPlateauByCode(code string) *FeatureType {
	for _, f := range ft.Plateau {
		if f.Code == code {
			return &f
		}
	}
	return nil
}

type CMSInfo struct {
	CMSURL      string
	WorkspaceID string
	ProjectID   string
	ModelIDMap  ModelIDMap
	FlowEnabled bool // whether Flow datasets should be included in the catalog
}

func (c CMSInfo) ItemBaseURL(modelKey string) string {
	return c.ModelIDMap.ItemBaseURL(c.CMSURL, c.WorkspaceID, c.ProjectID, modelKey)
}

type ModelIDMap map[string]string

func (m ModelIDMap) ItemBaseURL(cmsURL, workspaceID, projectID, modelKey string) string {
	if cmsURL == "" || workspaceID == "" || projectID == "" || m == nil || modelKey == "" {
		return ""
	}

	v, ok := m[modelKey]
	if !ok {
		return ""
	}

	return fmt.Sprintf("%s/workspace/%s/project/%s/content/%s/details/", cmsURL, workspaceID, projectID, v)
}
