package plateaucms

import (
	"context"
	"net/http"
	"net/http/httptest"
	"net/url"
	"sync"
	"testing"
	"time"

	"github.com/jarcoal/httpmock"
	"github.com/labstack/echo/v4"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearthx/rerror"
	"github.com/samber/lo"
	"github.com/stretchr/testify/assert"
)

const (
	testCMSHost            = "https://example.com"
	testCMSToken           = "token"
	testCMSProject         = "prj"
	testSidebarAccessToken = "access_token"
	testModelKey           = "model1"
	testModelKey2          = "model2"
)

func TestHandler_AuthMiddleware(t *testing.T) {
	httpmock.Activate()
	defer httpmock.Deactivate()
	mockCMS(t)
	h := newHandler()
	e := echo.New()

	t.Run("normal project", func(t *testing.T) {
		m := h.AuthMiddleware(AuthMiddlewareConfig{
			AuthMethods: HTTPMethodsAll,
		})
		handler := m(func(c echo.Context) error {
			return nil
		})
		r := httptest.NewRequest("POST", "/", nil)
		r.Header.Set("Authorization", "Bearer ac")
		w := httptest.NewRecorder()
		c := e.NewContext(r, w)
		c.SetPath("/:pid")
		c.SetParamNames("pid")
		c.SetParamValues("prjprj")

		assert.NoError(t, handler(c))
		assert.Equal(t, http.StatusOK, w.Result().StatusCode)
		assert.Equal(t, "prjprj", c.Param("pid"))
		assert.NotNil(t, GetCMSFromContext(c.Request().Context()))
		assert.Equal(t, Metadata{
			ProjectAlias:             "prjprj",
			DataCatalogProjectAlias:  "prjprjprj",
			DataCatalogSchemaVersion: "v3",
			CMSAPIKey:                "token!",
			SidebarAccessToken:       "ac",
			Auth:                     true,
			CMSBaseURL:               testCMSHost,
		}, GetCMSMetadataFromContext(c.Request().Context()))
	})

	t.Run("default project", func(t *testing.T) {
		m := h.AuthMiddleware(AuthMiddlewareConfig{
			AuthMethods:    HTTPMethodsAll,
			DefaultProject: "prjprj",
		})
		handler := m(func(c echo.Context) error {
			return nil
		})
		r := httptest.NewRequest("POST", "/", nil)
		r.Header.Set("Authorization", "Bearer ac")
		w := httptest.NewRecorder()
		c := e.NewContext(r, w)

		assert.NoError(t, handler(c))
		assert.Equal(t, http.StatusOK, w.Result().StatusCode)
		assert.Empty(t, c.Param("pid"))
		assert.NotNil(t, GetCMSFromContext(c.Request().Context()))
		assert.Equal(t, Metadata{
			ProjectAlias:             "prjprj",
			DataCatalogProjectAlias:  "prjprjprj",
			DataCatalogSchemaVersion: "v3",
			CMSAPIKey:                "token!",
			SidebarAccessToken:       "ac",
			Auth:                     true,
			CMSBaseURL:               testCMSHost,
		}, GetCMSMetadataFromContext(c.Request().Context()))
	})

	t.Run("normal project with invalid token", func(t *testing.T) {
		m := h.AuthMiddleware(AuthMiddlewareConfig{
			AuthMethods: HTTPMethodsAll,
		})
		handler := m(func(c echo.Context) error {
			return nil
		})
		r := httptest.NewRequest("POST", "/", nil)
		r.Header.Set("Autorization", "Bearer invalid")
		w := httptest.NewRecorder()
		c := e.NewContext(r, w)
		c.SetPath("/:pid")
		c.SetParamNames("pid")
		c.SetParamValues("prjprj")

		assert.NoError(t, handler(c))
		assert.Equal(t, http.StatusUnauthorized, w.Result().StatusCode)
		assert.Equal(t, "prjprj", c.Param("pid"))
		assert.Nil(t, GetCMSFromContext(c.Request().Context()))
		assert.Empty(t, GetCMSMetadataFromContext(c.Request().Context()))
	})

	t.Run("normal project with invalid token and skipAuth", func(t *testing.T) {
		m := h.AuthMiddleware(AuthMiddlewareConfig{})
		handler := m(func(c echo.Context) error {
			return nil
		})
		r := httptest.NewRequest("POST", "/", nil)
		r.Header.Set("Autorization", "Bearer invalid")
		w := httptest.NewRecorder()
		c := e.NewContext(r, w)
		c.SetPath("/:pid")
		c.SetParamNames("pid")
		c.SetParamValues("prjprj")

		assert.NoError(t, handler(c))
		assert.Equal(t, http.StatusOK, w.Result().StatusCode)
		assert.Equal(t, "prjprj", c.Param("pid"))
		assert.NotNil(t, GetCMSFromContext(c.Request().Context()))
		assert.Equal(t, Metadata{
			ProjectAlias:             "prjprj",
			DataCatalogProjectAlias:  "prjprjprj",
			DataCatalogSchemaVersion: "v3",
			CMSAPIKey:                "token!",
			SidebarAccessToken:       "ac",
			Auth:                     false,
			CMSBaseURL:               testCMSHost,
		}, GetCMSMetadataFromContext(c.Request().Context()))
	})

	t.Run("invalid project", func(t *testing.T) {
		m := h.AuthMiddleware(AuthMiddlewareConfig{
			AuthMethods: HTTPMethodsAll,
		})
		handler := m(func(c echo.Context) error {
			return nil
		})
		r := httptest.NewRequest("POST", "/", nil)
		w := httptest.NewRecorder()
		c := e.NewContext(r, w)
		c.SetPath("/:pid")
		c.SetParamNames("pid")
		c.SetParamValues("prjprj!")

		assert.NoError(t, handler(c))
		assert.Equal(t, http.StatusOK, w.Result().StatusCode)
		assert.Nil(t, GetCMSFromContext(c.Request().Context()))
		assert.Empty(t, GetCMSMetadataFromContext(c.Request().Context()))
	})
}

func TestHandler_Metadata(t *testing.T) {
	httpmock.Activate()
	defer httpmock.Deactivate()
	mockCMS(t)
	h := newHandler()

	expectedAll := MetadataList{
		{
			ProjectAlias:            testCMSProject,
			DataCatalogProjectAlias: testCMSProject,
			CMSAPIKey:               testCMSToken,
			SidebarAccessToken:      testSidebarAccessToken,
			CMSBaseURL:              testCMSHost,
		},
		{
			ProjectAlias:             "prjprj",
			DataCatalogProjectAlias:  "prjprjprj",
			DataCatalogSchemaVersion: "v3",
			CMSAPIKey:                "token!",
			SidebarAccessToken:       "ac",
			CMSBaseURL:               testCMSHost,
		},
	}

	md, all, err := h.Metadata(context.Background(), "prjprj", false, false)
	assert.NoError(t, err)
	assert.Equal(t, Metadata{
		ProjectAlias:             "prjprj",
		DataCatalogProjectAlias:  "prjprjprj",
		DataCatalogSchemaVersion: "v3",
		CMSAPIKey:                "token!",
		SidebarAccessToken:       "ac",
		CMSBaseURL:               testCMSHost,
	}, md)
	assert.Equal(t, expectedAll, all)

	md, all, err = h.Metadata(context.Background(), "prjprjprj", false, false)
	assert.Equal(t, rerror.ErrNotFound, err)
	assert.Equal(t, expectedAll, all)
	assert.Empty(t, md)

	md, all, err = h.Metadata(context.Background(), "prjprjprj", true, false)
	assert.NoError(t, err)
	assert.Equal(t, expectedAll, all)
	assert.Equal(t, Metadata{
		ProjectAlias:             "prjprj",
		DataCatalogProjectAlias:  "prjprjprj",
		DataCatalogSchemaVersion: "v3",
		CMSAPIKey:                "token!",
		SidebarAccessToken:       "ac",
		CMSBaseURL:               testCMSHost,
	}, md)
}

func TestHandler_LastModified(t *testing.T) {
	httpmock.Activate()
	defer httpmock.Deactivate()
	lastModified := time.Date(2022, time.April, 1, 0, 0, 0, 0, time.Local)
	lastModified2 := time.Date(2022, time.April, 2, 0, 0, 0, 0, time.Local)

	httpmock.RegisterResponder(
		"GET",
		lo.Must(url.JoinPath(testCMSHost, "api", "projects", testCMSProject, "models", testModelKey)),
		httpmock.NewJsonResponderOrPanic(http.StatusOK, &cms.Model{LastModified: lastModified}),
	)
	httpmock.RegisterResponder(
		"GET",
		lo.Must(url.JoinPath(testCMSHost, "api", "projects", testCMSProject, "models", testModelKey2)),
		httpmock.NewJsonResponderOrPanic(http.StatusOK, &cms.Model{LastModified: lastModified2}),
	)
	h := newHandler()
	cms := lo.Must(cms.New(testCMSHost, testCMSToken))

	e := echo.New()

	// no If-Modified-Since
	r := httptest.NewRequest("GET", "/", nil)
	r = r.WithContext(context.WithValue(r.Context(), cmsContextKey{}, cms))
	w := httptest.NewRecorder()
	hit, err := h.LastModified(e.NewContext(r, w), testCMSProject, testModelKey, testModelKey2)
	assert.NoError(t, err)
	assert.False(t, hit)
	assert.Equal(t, lastModified2.Format(time.RFC1123), w.Header().Get(echo.HeaderLastModified))

	// If-Modified-Since
	r = httptest.NewRequest("GET", "/", nil)
	r = r.WithContext(context.WithValue(r.Context(), cmsContextKey{}, cms))
	r.Header.Set(echo.HeaderIfModifiedSince, lastModified2.Format(time.RFC1123))
	w = httptest.NewRecorder()
	hit, err = newHandler().LastModified(e.NewContext(r, w), testCMSProject, testModelKey, testModelKey2)
	assert.NoError(t, err)
	assert.True(t, hit)
	assert.Equal(t, http.StatusNotModified, w.Result().StatusCode)
	assert.Equal(t, lastModified2.Format(time.RFC1123), w.Header().Get(echo.HeaderLastModified))

	// expired If-Modified-Since
	r = httptest.NewRequest("GET", "/", nil)
	r = r.WithContext(context.WithValue(r.Context(), cmsContextKey{}, cms))
	r.Header.Set(echo.HeaderIfModifiedSince, lastModified.Format(time.RFC1123))
	w = httptest.NewRecorder()
	hit, err = newHandler().LastModified(e.NewContext(r, w), testCMSProject, testModelKey, testModelKey2)
	assert.NoError(t, err)
	assert.False(t, hit)
	assert.Equal(t, lastModified2.Format(time.RFC1123), w.Header().Get(echo.HeaderLastModified))
}

func newHandler() *CMS {
	return &CMS{
		cmsbase:       testCMSHost,
		cmsSysProject: tokenProject,
		cmsMain:       lo.Must(cms.New(testCMSHost, testCMSToken)),
	}
}

func mockCMS(t *testing.T) {
	t.Helper()

	httpmock.RegisterResponder(
		"GET",
		lo.Must(url.JoinPath(testCMSHost, "api", "projects", tokenProject, "models", metadataModel, "items")),
		httpmock.NewJsonResponderOrPanic(http.StatusOK, cms.Items{
			PerPage:    1,
			Page:       1,
			TotalCount: 1,
			Items: []cms.Item{
				{
					ID: "1",
					Fields: []*cms.Field{
						{Key: "project_alias", Value: testCMSProject},
						{Key: "cms_apikey", Value: testCMSToken},
						{Key: "sidebar_access_token", Value: testSidebarAccessToken},
					},
				},
				{
					ID: "2",
					Fields: []*cms.Field{
						{Key: "project_alias", Value: "prjprj"},
						{Key: "datacatalog_project_alias", Value: "prjprjprj"},
						{Key: "datacatalog_schema_version", Value: "v3"},
						{Key: "cms_apikey", Value: "token!"},
						{Key: "sidebar_access_token", Value: "ac"},
					},
				},
			},
		}),
	)
}

// TestHandler_AllMetadata_Cache guards SCA-01: repeated AllMetadata calls
// within the cache TTL must hit the upstream CMS at most once, and concurrent
// misses must be deduplicated via singleflight.
func TestHandler_AllMetadata_Cache(t *testing.T) {
	httpmock.Activate()
	defer httpmock.Deactivate()
	mockCMS(t)

	h := newHandler()
	h.metadataCacheTTL = time.Minute

	ctx := context.Background()

	// Prime the cache.
	_, err := h.AllMetadata(ctx, false)
	assert.NoError(t, err)
	baseline := httpmock.GetCallCountInfo()["GET "+lo.Must(url.JoinPath(testCMSHost, "api", "projects", tokenProject, "models", metadataModel, "items"))]
	assert.Positive(t, baseline)

	// Subsequent calls within the TTL must not hit upstream again.
	for range 5 {
		_, err := h.AllMetadata(ctx, false)
		assert.NoError(t, err)
	}
	assert.Equal(t, baseline, httpmock.GetCallCountInfo()["GET "+lo.Must(url.JoinPath(testCMSHost, "api", "projects", tokenProject, "models", metadataModel, "items"))])

	// Concurrent misses on a fresh handler should be deduplicated by singleflight.
	// Collect errors in per-goroutine slots and assert on the main goroutine
	// (testify's assertions are not safe to call concurrently, and doing so
	// races against -race under real contention).
	h2 := newHandler()
	h2.metadataCacheTTL = time.Minute
	const workers = 10
	errs := make([]error, workers)
	var wg sync.WaitGroup
	for i := range workers {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			_, err := h2.AllMetadata(ctx, false)
			errs[i] = err
		}(i)
	}
	wg.Wait()
	for _, err := range errs {
		assert.NoError(t, err)
	}
	// After the concurrent burst the counter should have advanced by only 1
	// (singleflight collapses the misses); allow ≤ baseline to tolerate that
	// GetItemsByKeyInParallel may issue more than one HTTP call per fetch.
	after := httpmock.GetCallCountInfo()["GET "+lo.Must(url.JoinPath(testCMSHost, "api", "projects", tokenProject, "models", metadataModel, "items"))]
	assert.LessOrEqual(t, after-baseline, baseline, "singleflight should collapse concurrent misses to a single upstream fetch")
}

func TestMetadataList_PlateauProjects(t *testing.T) {
	m := MetadataList{
		{
			DataCatalogProjectAlias: "plateau-2022",
		},
		{
			DataCatalogProjectAlias: "plateau-aaaa",
		},
		{
			DataCatalogProjectAlias: "plateau-2023",
		},
	}
	ps := m.PlateauProjects()
	assert.Equal(t, MetadataList{
		{
			DataCatalogProjectAlias: "plateau-2023",
		},
		{
			DataCatalogProjectAlias: "plateau-2022",
		},
	}, ps)
}
