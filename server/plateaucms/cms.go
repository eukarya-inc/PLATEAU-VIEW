package plateaucms

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"slices"
	"strings"
	"sync"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/putil"
	"github.com/labstack/echo/v4"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/rerror"
	"golang.org/x/sync/singleflight"
)

// defaultMetadataCacheTTL is the time-to-live for the in-memory metadata cache.
// Metadata rarely changes at request cadence (adding a project / rotating a token
// happens on human timescales), so a short TTL absorbs bursts of concurrent
// requests without noticeably delaying propagation of legitimate edits.
const defaultMetadataCacheTTL = 60 * time.Second

const (
	ProjectNameParam             = "pid"
	tokenProject                 = "system"
	metadataModel                = "workspaces"
	plateauSpecModel             = "plateau-spec"
	plateauFeatureTypesModel     = "plateau-features"
	plateauProjectModel          = "plateau-projects"
	datasetTypesModel            = "plateau-dataset-types"
	projectAliasField            = "project_alias"
	datacatalogProjectAliasField = "datacatalog_project_alias"
	plateauPrefix                = "plateau-"
)

var HTTPMethodsAll = []string{
	http.MethodGet,
	http.MethodPost,
	http.MethodPatch,
	http.MethodPut,
	http.MethodDelete,
}

var HTTPMethodsExceptGET = []string{
	http.MethodPost,
	http.MethodPatch,
	http.MethodPut,
	http.MethodDelete,
}

type Config struct {
	CMSBaseURL       string
	CMSMainToken     string
	CMSSystemProject string
	// compat
	CMSMainProject string
	AdminToken     string
}

type CMS struct {
	cmsbase       string
	cmsSysProject string
	cmsMain       cms.Interface
	// comapt
	cmsMainProject string
	cmsToken       string
	adminToken     string

	// metadata cache
	metadataCacheTTL time.Duration
	metadataSF       singleflight.Group
	metadataMu       sync.RWMutex
	metadataCache    MetadataList
	metadataFetched  time.Time

	// plateau feature type / dataset type cache
	featureTypeCacheTTL time.Duration
	featureTypeSF       singleflight.Group
	featureTypeMu       sync.RWMutex
	featureTypeCache    PlateauFeatureTypeList
	featureTypeFetched  time.Time
	datasetTypeCache    DatasetTypeList
	datasetTypeFetched  time.Time
}

func New(c Config) (*CMS, error) {
	cmsMain, err := cms.New(c.CMSBaseURL, c.CMSMainToken)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize cms: %w", err)
	}

	if c.CMSSystemProject == "" {
		c.CMSSystemProject = tokenProject
	}

	return &CMS{
		cmsbase:       c.CMSBaseURL,
		cmsSysProject: c.CMSSystemProject,
		cmsMain:       cmsMain,
		// compat
		cmsMainProject:      c.CMSMainProject,
		cmsToken:            c.CMSMainToken,
		adminToken:          c.AdminToken,
		metadataCacheTTL:    defaultMetadataCacheTTL,
		featureTypeCacheTTL: defaultMetadataCacheTTL,
	}, nil
}

// MainCMS returns the CMS client authenticated with the main token, used to
// access the system project.
func (h *CMS) MainCMS() cms.Interface {
	return h.cmsMain
}

// SystemProject returns the alias of the CMS system project.
func (h *CMS) SystemProject() string {
	return h.cmsSysProject
}

func (h *CMS) Clone() *CMS {
	return &CMS{
		cmsbase:       h.cmsbase,
		cmsSysProject: h.cmsSysProject,
		cmsMain:       h.cmsMain,
		// compat
		cmsMainProject:      h.cmsMainProject,
		cmsToken:            h.cmsToken,
		adminToken:          h.adminToken,
		metadataCacheTTL:    h.metadataCacheTTL,
		featureTypeCacheTTL: h.featureTypeCacheTTL,
		// Note: singleflight.Group and cache state intentionally not copied —
		// a clone is a fresh instance that maintains its own cache.
	}
}

type AuthMiddlewareConfig struct {
	Key             string
	AuthMethods     []string
	FindDataCatalog bool
	DefaultProject  string
	UseDefault      bool
}

func (h *CMS) LastModified(c echo.Context, prj string, models ...string) (bool, error) {
	ctx := c.Request().Context()
	cmsh := GetCMSFromContext(ctx)

	mlastModified := time.Time{}
	for _, m := range models {
		model, err := cmsh.GetModelByKey(ctx, prj, m)
		if err != nil {
			if errors.Is(err, cms.ErrNotFound) {
				continue
			}
			return false, err
		}

		if model != nil && mlastModified.Before(model.LastModified) {
			mlastModified = model.LastModified
		}
	}

	return putil.LastModified(c, mlastModified)
}

func (h *CMS) AuthMiddleware(conf AuthMiddlewareConfig) echo.MiddlewareFunc {
	key := conf.Key
	if key == "" {
		key = ProjectNameParam
	}

	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			req := c.Request()
			ctx := req.Context()

			prj := c.Param(key)
			token := ""
			if t := req.Header.Get("Authorization"); t != "" && strings.HasPrefix(t, "Bearer ") {
				token = strings.TrimPrefix(t, "Bearer ")
			}

			ctx, err := h.InitContext(ctx, conf, prj, token, req.Method)
			if err != nil {
				if errors.Is(err, echo.ErrUnauthorized) {
					return c.JSON(http.StatusUnauthorized, "unauthorized")
				}
				return err
			}

			c.SetRequest(req.WithContext(ctx))
			return next(c)
		}
	}
}

func (h *CMS) InitContext(ctx context.Context, conf AuthMiddlewareConfig, prj, token, method string) (context.Context, error) {
	authMethods := conf.AuthMethods
	findDataCatalog := conf.FindDataCatalog
	defaultProject := conf.DefaultProject
	useDefault := conf.UseDefault

	if prj == "" {
		prj = defaultProject
	}

	md, all, err := h.Metadata(ctx, prj, findDataCatalog, useDefault)
	if len(all) > 0 {
		ctx = SetAllCMSMetadataFromContext(ctx, all)
	}

	if err != nil {
		if errors.Is(err, rerror.ErrNotFound) {
			ctx = context.WithValue(ctx, cmsMetadataContextKey{}, md)
			return ctx, nil
		}
		return nil, err
	}

	cmsh, err := cms.New(h.cmsbase, md.CMSAPIKey)
	if err != nil {
		return nil, rerror.ErrInternalBy(fmt.Errorf("plateaucms: failed to create cms for %s: %w", prj, err))
	}

	// auth
	if md.SidebarAccessToken == "" || token != md.SidebarAccessToken {
		if len(authMethods) > 0 && slices.Contains(authMethods, method) {
			return nil, echo.ErrUnauthorized
		}
	} else {
		md.Auth = true
	}

	// attach
	ctx = context.WithValue(ctx, plateauCMSContextKey{}, h)
	ctx = context.WithValue(ctx, cmsMetadataContextKey{}, md)
	ctx = context.WithValue(ctx, cmsContextKey{}, cmsh)
	return ctx, nil
}

func valueToAssetURL(v *cms.Value) string {
	return anyToAssetURL(v.Interface())
}

func anyToAssetURL(v any) string {
	if v == nil {
		return ""
	}

	m, ok := v.(map[string]any)
	if !ok {
		m2, ok := v.(map[any]any)
		if !ok {
			return ""
		}

		m = map[string]interface{}{}
		for k, v := range m2 {
			if s, ok := k.(string); ok {
				m[s] = v
			}
		}
	}

	url, ok := m["url"].(string)
	if !ok {
		return ""
	}

	return url
}
