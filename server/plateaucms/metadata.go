package plateaucms

import (
	"context"
	"errors"
	"fmt"
	"slices"
	"strconv"
	"strings"
	"time"

	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/rerror"
	"github.com/samber/lo"
)

type MetadataStore interface {
	AllMetadata(ctx context.Context, findDataCatalog bool) (MetadataList, error)
	Metadata(ctx context.Context, prj string, findDataCatalog, useDefault bool) (Metadata, MetadataList, error)
}

var _ MetadataStore = &CMS{}

type Metadata struct {
	Name                     string `json:"name" cms:"name,text"`
	ProjectAlias             string `json:"project_alias" cms:"project_alias,text"`
	DataCatalogProjectAlias  string `json:"datacatalog_project_alias" cms:"datacatalog_project_alias,text"`
	DataCatalogSchemaVersion string `json:"datacatalog_schema_version" cms:"datacatalog_schema_version,select"`
	DataCatalogFlowEnabled   bool   `json:"datacatalog_flow_enabled" cms:"datacatalog_flow_enabled,boolean"`
	CMSAPIKey                string `json:"cms_apikey" cms:"cms_apikey,text"`
	SidebarAccessToken       string `json:"sidebar_access_token" cms:"sidebar_access_token,text"`
	SubPorjectAlias          string `json:"subproject_alias" cms:"subproject_alias,text"`
	CMSURL                   string `json:"cms_url" cms:"cms_url,text"`
	WorkspaceID              string `json:"workspace_id" cms:"workspace_id,text"`
	ProjectID                string `json:"project_id" cms:"project_id,text"`
	MergePlateau             bool   `json:"merge_plateau" cms:"merge_plateau,boolean"`
	Disabled                 bool   `json:"disabled" cms:"disabled,boolean"`
	// whether the request is authenticated with sidebar access token
	Auth       bool   `json:"-" cms:"-"`
	CMSBaseURL string `json:"-" cms:"-"`
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

func (m Metadata) IsV2() bool {
	return m.DataCatalogSchemaVersion == "" || m.DataCatalogSchemaVersion == "v2"
}

func (m Metadata) IsV3() bool {
	return m.DataCatalogSchemaVersion == "v3"
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

// V3Projects returns all v3 projects sorted by year (newest first)
// Note: Disabled projects and invalid entries are already filtered out in AllMetadata
func (l MetadataList) V3Projects() MetadataList {
	m := lo.FilterMap(l, func(m Metadata, _ int) (lo.Tuple2[Metadata, int], bool) {
		if !m.IsV3() {
			return lo.Tuple2[Metadata, int]{}, false
		}
		y := m.PlateauYear()
		return lo.Tuple2[Metadata, int]{A: m, B: y}, true
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

// AllMetadata returns the full metadata list for the system project.
//
// This is on the hot path of AuthMiddleware — invoked on every authenticated
// request through the sidebar, share, and datacatalog services — so the result
// is cached in memory with a short TTL. Concurrent misses are deduplicated via
// singleflight so a burst of cache-miss requests results in a single upstream
// fan-out to the CMS rather than one per request. The `findDataCatalog`
// parameter does not affect the fetched set (filtering happens downstream in
// MetadataList methods), so a single cache entry serves both cases.
func (h *CMS) AllMetadata(ctx context.Context, findDataCatalog bool) (MetadataList, error) {
	if h.cmsSysProject == "" {
		return nil, rerror.ErrNotFound
	}

	if cached, ok := h.cachedAllMetadata(); ok {
		return cached, nil
	}

	v, err, _ := h.metadataSF.Do("all", func() (any, error) {
		// Recheck under singleflight: an earlier waiter may have populated the
		// cache while this goroutine was queued.
		if cached, ok := h.cachedAllMetadata(); ok {
			return cached, nil
		}

		// Detach cancellation for the shared fetch: singleflight has multiple
		// waiters and the leader is a coincidence — if the leader's request is
		// cancelled (client disconnect / handler deadline), the upstream fetch
		// must still complete so the other waiters aren't poisoned by
		// context.Canceled. A bounded timeout keeps a stuck upstream from
		// hanging the group forever.
		fetchCtx, cancel := context.WithTimeout(context.WithoutCancel(ctx), metadataFetchTimeout)
		defer cancel()

		list, err := h.fetchAllMetadata(fetchCtx)
		if err != nil {
			return nil, err
		}

		h.storeAllMetadata(list)
		return list, nil
	})
	if err != nil {
		return nil, err
	}
	return v.(MetadataList), nil
}

// metadataFetchTimeout bounds the shared upstream fetch inside singleflight so
// a stuck CMS API can't keep followers waiting indefinitely. This is separate
// from any per-request deadline because the leader's context is detached (see
// AllMetadata for the rationale).
const metadataFetchTimeout = 30 * time.Second

func (h *CMS) cachedAllMetadata() (MetadataList, bool) {
	ttl := h.metadataCacheTTL
	if ttl <= 0 {
		return nil, false
	}

	h.metadataMu.RLock()
	defer h.metadataMu.RUnlock()

	if h.metadataCache == nil || time.Since(h.metadataFetched) >= ttl {
		return nil, false
	}
	return h.metadataCache, true
}

func (h *CMS) storeAllMetadata(list MetadataList) {
	if h.metadataCacheTTL <= 0 {
		return
	}

	h.metadataMu.Lock()
	defer h.metadataMu.Unlock()
	h.metadataCache = list
	h.metadataFetched = time.Now()
}

func (h *CMS) fetchAllMetadata(ctx context.Context) (MetadataList, error) {
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
		if m.CMSAPIKey == "" || m.Disabled {
			continue
		}
		if m.DataCatalogProjectAlias == "" {
			m.DataCatalogProjectAlias = m.ProjectAlias
		}
		if m.DataCatalogProjectAlias == "" {
			continue
		}
		m.CMSBaseURL = h.cmsbase
		all = append(all, m)
	}

	return all, nil
}
