package plateaucms

import (
	"context"
	"errors"
	"fmt"
	"slices"
	"strconv"
	"strings"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/rerror"
	"github.com/samber/lo"
)

type MetadataStore interface {
	AllMetadata(ctx context.Context, findDataCatalog bool) (MetadataList, error)
	Metadata(ctx context.Context, prj string, findDataCatalog, useDefault bool) (Metadata, MetadataList, error)
}

var _ MetadataStore = &CMS{}

const (
	ConverterFME     = "fme"
	ConverterFlow    = "flow"
	ConverterFMEFlow = "fme_flow"
)

type Metadata struct {
	Name                     string `json:"name" cms:"name,text"`
	ProjectAlias             string `json:"project_alias" cms:"project_alias,text"`
	DataCatalogProjectAlias  string `json:"datacatalog_project_alias" cms:"datacatalog_project_alias,text"`
	DataCatalogSchemaVersion string `json:"datacatalog_schema_version" cms:"datacatalog_schema_version,select"`
	CMSAPIKey                string `json:"cms_apikey" cms:"cms_apikey,text"`
	SidebarAccessToken       string `json:"sidebar_access_token" cms:"sidebar_access_token,text"`
	SubPorjectAlias          string `json:"subproject_alias" cms:"subproject_alias,text"`
	CMSURL                   string `json:"cms_url" cms:"cms_url,text"`
	WorkspaceID              string `json:"workspace_id" cms:"workspace_id,text"`
	ProjectID                string `json:"project_id" cms:"project_id,text"`
	MergePlateau             bool   `json:"merge_plateau" cms:"merge_plateau,boolean"`
	Converter                string `json:"converter" cms:"converter,select"`
	// whether the request is authenticated with sidebar access token
	Auth       bool   `json:"-" cms:"-"`
	CMSBaseURL string `json:"-" cms:"-"`
}

func (m Metadata) IsFMEEnabled() bool {
	return m.Converter == ConverterFME || m.Converter == ConverterFMEFlow
}

func (m Metadata) IsFlowEnabled() bool {
	return m.Converter == ConverterFlow || m.Converter == ConverterFMEFlow
}

func (m Metadata) CMS() (*cms.CMS, error) {
	return cms.New(m.CMSBaseURL, m.CMSAPIKey)
}

func (m Metadata) IsPlateau() bool {
	return strings.HasPrefix(m.DataCatalogProjectAlias, plateauPrefix)
}

func (m Metadata) PlateauYear() int {
	if !strings.HasPrefix(m.DataCatalogProjectAlias, plateauPrefix) {
		return 0
	}

	name := strings.TrimPrefix(m.DataCatalogProjectAlias, plateauPrefix)
	if len(name) < 4 {
		return 0
	}

	year, err := strconv.Atoi(name[:4])
	if err != nil {
		return 0
	}

	return year
}

func (m Metadata) IsValidToken(token string) bool {
	return m.SidebarAccessToken == token
}

type MetadataList []Metadata

func (l MetadataList) PlateauProjects() MetadataList {
	m := lo.FilterMap(l, func(m Metadata, _ int) (lo.Tuple2[Metadata, int], bool) {
		y := m.PlateauYear()
		return lo.Tuple2[Metadata, int]{A: m, B: y}, y > 0
	})

	slices.SortFunc(m, func(a, b lo.Tuple2[Metadata, int]) int {
		return b.B - a.B
	})

	return lo.Map(m, func(t lo.Tuple2[Metadata, int], _ int) Metadata {
		return t.A
	})
}

func (l MetadataList) FindSys(project string) (Metadata, bool) {
	return lo.Find(l, func(m Metadata) bool {
		return m.ProjectAlias == project
	})
}

func (l MetadataList) FindDataCatalog(project string) (Metadata, bool) {
	return lo.Find(l, func(m Metadata) bool {
		return m.DataCatalogProjectAlias == project
	})
}

func (l MetadataList) FindMetadata(prj string, findDataCatalog, useDefault bool) (Metadata, bool) {
	if prj == "" && useDefault {
		m := l.Default()
		if m == nil {
			return Metadata{}, false
		}
		return *m, true
	}

	md, ok := lo.Find(l, func(i Metadata) bool {
		return findDataCatalog && i.DataCatalogProjectAlias == prj || i.ProjectAlias == prj || i.ProjectID == prj
	})

	if !ok {
		return Metadata{}, false
	}

	return md, true
}

func (l MetadataList) FindDataCatalogAndSub(project string) (res MetadataList) {
	m, ok := l.FindDataCatalog(project)
	if !ok {
		return nil
	}

	res = MetadataList{m}
	if m.SubPorjectAlias == "" && !m.MergePlateau {
		return
	}

	if m.MergePlateau {
		return append(res, l.PlateauProjects()...)
	}

	sub, ok := l.FindDataCatalog(m.SubPorjectAlias)
	if !ok {
		return
	}

	return append(res, sub)
}

func (metadata MetadataList) Default() *Metadata {
	p := metadata.PlateauProjects()
	if len(p) == 0 {
		return nil
	}
	return &p[0]
}

func (metadata MetadataList) FindByYear(year int) *Metadata {
	if year <= 0 {
		return nil
	}

	for _, m := range metadata {
		if y := m.PlateauYear(); y > 0 && y == year {
			return &m
		}
	}
	return nil
}

func (h *CMS) Metadata(ctx context.Context, prj string, findDataCatalog, useDefault bool) (Metadata, MetadataList, error) {
	// compat
	if h.cmsMainProject != "" && prj == h.cmsMainProject {
		return Metadata{
			ProjectAlias:       h.cmsMainProject,
			CMSAPIKey:          h.cmsToken,
			SidebarAccessToken: h.adminToken,
			CMSBaseURL:         h.cmsbase,
		}, nil, nil
	}

	all, err := h.AllMetadata(ctx, findDataCatalog)
	if err != nil {
		return Metadata{}, nil, err
	}

	md, ok := all.FindMetadata(prj, findDataCatalog, useDefault)
	if !ok {
		return Metadata{}, all, rerror.ErrNotFound
	}

	return md, all, nil
}

func (h *CMS) AllMetadata(ctx context.Context, findDataCatalog bool) (MetadataList, error) {
	if h.cmsSysProject == "" {
		return nil, rerror.ErrNotFound
	}

	items, err := h.cmsMain.GetItemsByKeyInParallel(ctx, h.cmsSysProject, metadataModel, false, 100)
	if err != nil || items == nil {
		if errors.Is(err, cms.ErrNotFound) || items == nil {
			return nil, rerror.ErrNotFound
		}
		return nil, rerror.ErrInternalBy(fmt.Errorf("plateaucms: failed to get metadata: %w", err))
	}

	all := make([]Metadata, 0, len(items.Items))
	for _, item := range items.Items {
		m := Metadata{}
		item.Unmarshal(&m)
		if m.CMSAPIKey == "" {
			continue
		}
		if m.DataCatalogProjectAlias == "" {
			m.DataCatalogProjectAlias = m.ProjectAlias
		}
		m.CMSBaseURL = h.cmsbase
		all = append(all, m)
	}

	return all, nil
}
