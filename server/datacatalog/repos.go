package datacatalog

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/JamesLMilner/quadtree-go"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogv2"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogv2/datacatalogv2adapter"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/datacatalogv3"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/geocoding"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/plateauapi"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/govpolygon"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
	"github.com/reearth/reearthx/rerror"
	"github.com/samber/lo"
	"golang.org/x/sync/errgroup"
)

var qt *govpolygon.Quadtree

func init() {
	qt = govpolygon.NewQuadtree(nil, 1.0/60.0)
}

// ReposHandler handles data catalog repositories
type ReposHandler struct {
	reposv3            *datacatalogv3.Repos
	reposv2            *datacatalogv2adapter.Repos
	pcms               *plateaucms.CMS
	gqlComplexityLimit int
	cacheUpdateKey     string
	geocodingAppID     string
	cityConcurrency    int
	cacheURL           string
	host               string

	qt *govpolygon.Quadtree
}

const pidParamName = "pid"
const conditionsParamName = "conditions"
const gqlComplexityLimit = 1000
const cmsSchemaVersion = "v3"
const cmsSchemaVersionV2 = "v2"
const defaultCityConcurrency = 10

// NewReposHandler creates a new ReposHandler
func NewReposHandler(conf Config, pcms *plateaucms.CMS) (*ReposHandler, error) {
	reposv3 := datacatalogv3.NewRepos(pcms)
	reposv2 := datacatalogv2adapter.NewRepos()

	if conf.GraphqlMaxComplexity <= 0 {
		conf.GraphqlMaxComplexity = gqlComplexityLimit
	}

	if conf.CityConcurrency <= 0 {
		conf.CityConcurrency = defaultCityConcurrency
	}

	if conf.DiskCache {
		reposv3.EnableCache(true)
	}

	if conf.Debug {
		reposv3.EnableDebug(true)
	}

	if conf.Host != "" {
		reposv3.SetHost(conf.Host)
	}

	return &ReposHandler{
		reposv3:            reposv3,
		reposv2:            reposv2,
		pcms:               pcms,
		gqlComplexityLimit: conf.GraphqlMaxComplexity,
		cacheUpdateKey:     conf.CacheUpdateKey,
		geocodingAppID:     conf.GeocodingAppID,
		cityConcurrency:    conf.CityConcurrency,
		cacheURL:           conf.CacheURL,
		host:               conf.Host,
		qt:                 qt,
	}, nil
}

func (h *ReposHandler) Middleware() echo.MiddlewareFunc {
	return h.pcms.AuthMiddleware(plateaucms.AuthMiddlewareConfig{
		Key:             pidParamName,
		FindDataCatalog: true,
		UseDefault:      true,
	})
}

func (h *ReposHandler) Handler(admin bool) echo.HandlerFunc {
	return func(c echo.Context) error {
		merged, err := h.prepareMergedRepo(c, admin)
		if err != nil {
			return err
		}

		srv := plateauapi.NewService(merged, plateauapi.FixedComplexityLimit(h.gqlComplexityLimit))

		adminContext(c, admin, admin, admin && isAlpha(c))
		srv.ServeHTTP(c.Response(), c.Request())
		return nil
	}
}

func (h *ReposHandler) SimplePlateauDatasetsAPI() echo.HandlerFunc {
	return func(c echo.Context) error {
		merged, err := h.prepareMergedRepo(c, false)
		if err != nil {
			return err
		}

		ctx := c.Request().Context()
		res, err := FetchSimplePlateauDatasets(ctx, merged, h.host)
		if err != nil {
			return err
		}

		return c.JSONPretty(http.StatusOK, res, "  ")
	}
}

func (h *ReposHandler) CityGMLFiles(admin bool) echo.HandlerFunc {
	var geocoder GeoCoder
	if h.geocodingAppID != "" {
		g := geocoding.NewClient(h.geocodingAppID)
		geocoder = func(ctx context.Context, address string) (quadtree.Bounds, error) {
			return g.Bounds(ctx, address)
		}
	}

	return func(c echo.Context) error {
		ctx := c.Request().Context()
		conditions := c.Param(conditionsParamName)

		// Get feature types from query parameter
		var featureTypes []string
		if typesParam := c.QueryParam("types"); typesParam != "" {
			featureTypes = strings.Split(typesParam, ",")
		}

		bounds, filter, typeFilter, err := parseCityGMLFilesQuery(ctx, conditions, featureTypes, geocoder)
		if err != nil {
			if errors.Is(err, rerror.ErrNotFound) {
				return echo.NewHTTPError(http.StatusNotFound, "not found")
			}

			return echo.NewHTTPError(http.StatusBadRequest, err)
		}

		var cityIDs []string
		if len(bounds) > 0 {
			for _, b := range bounds {
				cityIDs = append(cityIDs, h.qt.FindRect(b.QBounds())...)
			}
		} else {
			// conditions is just a city id
			cityIDs = strings.Split(conditions, ",")
		}
		cityIDs = lo.Uniq(cityIDs)

		merged, err := h.prepareMergedRepo(c, admin)
		if err != nil {
			return err
		}

		adminContext(c, true, admin, admin && isAlpha(c))
		ctx = c.Request().Context() // do not forget to update context

		// Expand ward codes to include their parent city codes. CityGML data for
		// designated cities (e.g. Sapporo 01100) is registered at the city level,
		// while the govpolygon quadtree only returns ward codes (01101, 01102, ...).
		if len(bounds) > 0 {
			expanded := make([]string, 0, len(cityIDs)*2)
			for _, cid := range cityIDs {
				expanded = append(expanded, cid)
				area, err := merged.Area(ctx, plateauapi.AreaCode(cid))
				if err != nil {
					log.Warnfc(ctx, "datacatalog: failed to resolve area %s: %v", cid, err)
					continue
				}
				if w, ok := area.(*plateauapi.Ward); ok && w != nil && w.CityCode != "" {
					expanded = append(expanded, string(w.CityCode))
				}
			}
			cityIDs = lo.Uniq(expanded)
		}

		// Pre-fetch feature type names from CMS (unfiltered, includes types like dem
		// that have CityGML files but no 3D Tiles datasets).
		plateauFeatureTypes, err := h.pcms.PlateauFeatureTypes(ctx)
		if err != nil {
			return fmt.Errorf("failed to get feature types: %w", err)
		}
		featureTypeNames := plateauFeatureTypes.CodeNameMap()

		// Fetch cities concurrently
		results := make([]*CityGMLFilesCity, len(cityIDs))
		errg := errgroup.Group{}
		errg.SetLimit(h.cityConcurrency) // Limit concurrent fetches to avoid overwhelming upstream servers

		for i, cid := range cityIDs {
			i := i     // Capture loop variable
			cid := cid // Capture loop variable
			errg.Go(func() error {
				start := time.Now()
				cityGMLFiles, err := FetchCityGMLFiles(ctx, merged, cid, featureTypeNames)
				duration := time.Since(start)

				if err != nil {
					log.Warnfc(ctx, "datacatalog: failed to fetch citygml files for city %s in %s: %v", cid, duration, err)
					return nil // Allow partial failures - continue fetching other cities
				}
				if cityGMLFiles != nil {
					log.Debugfc(ctx, "datacatalog: fetched citygml files for city %s in %s", cid, duration)
					results[i] = cityGMLFiles
				}
				return nil
			})
		}

		if err := errg.Wait(); err != nil {
			return err
		}

		// Collect non-nil results
		cities := make([]*CityGMLFilesCity, 0, len(results))
		for _, city := range results {
			if city != nil {
				cities = append(cities, city)
			}
		}

		res := applyCityGMLCityFilter(cities, filter, typeFilter)
		if len(res.Cities) == 0 {
			return echo.NewHTTPError(http.StatusNotFound, "not found")
		}

		return c.JSON(http.StatusOK, res)
	}
}

func (h *ReposHandler) UpdateCacheHandler(c echo.Context) error {
	ctx := c.Request().Context()

	if h.cacheUpdateKey != "" {
		b := struct {
			Key string `json:"key"`
		}{}
		if err := c.Bind(&b); err != nil {
			return echo.ErrUnauthorized
		}
		if b.Key != h.cacheUpdateKey {
			return echo.ErrUnauthorized
		}
	}

	metadata, err := h.pcms.AllMetadata(ctx, true)
	if err != nil {
		return fmt.Errorf("datacatalogv3: failed to get all metadata: %w", err)
	}

	ctx = plateaucms.SetAllCMSMetadataFromContext(ctx, metadata)

	if err := h.UpdateCache(ctx); err != nil {
		log.Errorfc(ctx, "datacatalog: failed to update cache: %v", err)
		return c.JSON(http.StatusInternalServerError, "failed to update cache")
	}

	return c.JSON(http.StatusOK, "ok")
}

func (h *ReposHandler) WarningHandler(c echo.Context) error {
	pid := c.Param(pidParamName)
	md := plateaucms.GetCMSMetadataFromContext(c.Request().Context())
	if md.DataCatalogProjectAlias != pid || !isV3(md) {
		return echo.NewHTTPError(http.StatusNotFound, "not found")
	}

	if !md.Auth {
		return echo.NewHTTPError(http.StatusUnauthorized, "unauthorized")
	}

	t := h.reposv3.UpdatedAt(pid)
	res := ""
	if !t.IsZero() {
		res = fmt.Sprintf("updated at: %s\n", t.Format(time.RFC3339))
	}
	res += strings.Join(h.reposv3.Warnings(pid), "\n")
	return c.String(http.StatusOK, res)
}

func (h *ReposHandler) UpdateCache(ctx context.Context) error {
	// If cache URL is set, reload from GCS instead of CMS
	if h.cacheURL != "" {
		return h.reloadFromCache(ctx)
	}

	g, ctx := errgroup.WithContext(ctx)

	for _, p := range h.reposv3.Projects() {
		p := p
		g.Go(func() error {
			return h.updateV3(ctx, p)
		})
	}

	for _, p := range h.reposv2.Projects() {
		p := p
		g.Go(func() error {
			return h.updateV2(ctx, p)
		})
	}

	return g.Wait()
}

// reloadFromCache reloads all repos from cache storage (GCS or local file)
func (h *ReposHandler) reloadFromCache(ctx context.Context) error {
	reader, err := datacatalogv3.NewRepoReaderFromURL(ctx, h.cacheURL)
	if err != nil {
		return fmt.Errorf("datacatalogv3: failed to create cache reader: %w", err)
	}
	defer func() {
		_ = reader.Close()
	}()

	if err := h.reposv3.LoadAllFromStorage(ctx, reader); err != nil {
		return fmt.Errorf("datacatalogv3: failed to reload from cache: %w", err)
	}

	log.Infofc(ctx, "datacatalog: reloaded repos from cache: %s", h.cacheURL)
	return nil
}

func (h *ReposHandler) Init(ctx context.Context) error {
	metadata, err := h.pcms.AllMetadata(ctx, true)
	if err != nil {
		return fmt.Errorf("datacatalogv3: failed to get all metadata: %w", err)
	}

	ctx = plateaucms.SetAllCMSMetadataFromContext(ctx, metadata)

	plateauMetadata := metadata.PlateauProjects()
	if err := h.prepareAll(ctx, plateauMetadata); err != nil {
		return err
	}

	return nil
}

// InitFromCache initializes the repository from a cache storage (e.g., GCS or local file).
// This skips the CMS API calls and loads data directly from the cache.
// cacheURL can be:
//   - gs://bucket/path for GCS
//   - /path/to/cache for local filesystem
func (h *ReposHandler) InitFromCache(ctx context.Context, cacheURL string) error {
	reader, err := datacatalogv3.NewRepoReaderFromURL(ctx, cacheURL)
	if err != nil {
		return fmt.Errorf("datacatalogv3: failed to create cache reader: %w", err)
	}
	defer func() {
		_ = reader.Close()
	}()

	if err := h.reposv3.LoadAllFromStorage(ctx, reader); err != nil {
		return fmt.Errorf("datacatalogv3: failed to load from cache: %w", err)
	}

	return nil
}

func (h *ReposHandler) prepareMergedRepo(c echo.Context, auth bool) (plateauapi.Repo, error) {
	ctx := c.Request().Context()
	md := plateaucms.GetCMSMetadataFromContext(ctx)
	if auth && !md.Auth {
		return nil, echo.NewHTTPError(http.StatusUnauthorized, "unauthorized")
	}

	pid := c.Param(pidParamName)
	mds := plateaucms.GetAllCMSMetadataFromContext(ctx)
	merged := h.PrepareAndGetMergedRepo(ctx, pid, mds)
	if merged == nil {
		return nil, echo.NewHTTPError(http.StatusNotFound, "not found")
	}

	log.Debugfc(ctx, "datacatalogv3: use repo for %s: %s", pid, merged.Name())
	return merged, nil
}

// PrepareAndGetMergedRepo prepares and returns a merged repo for the given project
func (h *ReposHandler) PrepareAndGetMergedRepo(ctx context.Context, project string, metadata plateaucms.MetadataList) plateauapi.Repo {
	var mds plateaucms.MetadataList
	if project == "" {
		mds = metadata.PlateauProjects()
	} else {
		mds = metadata.FindDataCatalogAndSub(project)
	}

	if err := h.prepareAll(ctx, mds); err != nil {
		log.Errorfc(ctx, "failed to prepare repos: %v", err)
	}

	repos := make([]plateauapi.Repo, 0, len(mds))
	for _, s := range mds {
		if r := h.getRepo(ctx, s); r != nil {
			repos = append(repos, r)
		}
	}

	if len(repos) == 0 {
		return nil
	}

	if len(repos) == 1 {
		return repos[0]
	}

	merged := plateauapi.NewMerger(repos...)
	if err := merged.Init(ctx); err != nil {
		log.Errorfc(ctx, "datacatalogv3: failed to initialize merged repo: %v", err)
		return nil
	}

	return merged
}

func (h *ReposHandler) getRepo(ctx context.Context, md plateaucms.Metadata) (repo plateauapi.Repo) {
	if md.DataCatalogProjectAlias == "" {
		return
	}

	if isV2(md) {
		// v2: fetch lazily if not cached
		repo = h.reposv2.GetOrFetch(ctx, md.DataCatalogProjectAlias)
	} else if isV3(md) {
		repo = h.reposv3.Repo(md.DataCatalogProjectAlias)
	}
	return
}

func (h *ReposHandler) prepareAll(ctx context.Context, metadata plateaucms.MetadataList) error {
	errg, ctx := errgroup.WithContext(ctx)
	for _, md := range metadata {
		md := md

		errg.Go(func() error {
			if err := h.prepare(ctx, md); err != nil {
				return fmt.Errorf("failed to prepare repo for %s: %w", md.DataCatalogProjectAlias, err)
			}
			return nil
		})
	}
	return errg.Wait()
}

func (h *ReposHandler) prepare(ctx context.Context, md plateaucms.Metadata) error {
	if isV2(md) {
		return h.prepareV2(ctx, md)
	}
	return h.prepareV3(ctx, md)
}

func (h *ReposHandler) prepareV2(ctx context.Context, md plateaucms.Metadata) error {
	if !isV2(md) {
		return nil
	}

	f, err := newFetcherV2(md)
	if err != nil {
		return err
	}

	if err := h.reposv2.Prepare(ctx, f); err != nil {
		return fmt.Errorf("failed to prepare v2 repo for %s: %w", md.DataCatalogProjectAlias, err)
	}

	return nil
}

func (h *ReposHandler) prepareV3(ctx context.Context, md plateaucms.Metadata) error {
	if !isV3(md) {
		return nil
	}

	cms, err := md.CMS()
	if err != nil {
		return fmt.Errorf("datacatalogv3: failed to create cms for %s: %w", md.DataCatalogProjectAlias, err)
	}

	if err := h.reposv3.Prepare(ctx, md.DataCatalogProjectAlias, md.PlateauYear(), md.IsPlateau(), cms); err != nil {
		return fmt.Errorf("failed to prepare v3 repo for %s: %w", md.DataCatalogProjectAlias, err)
	}

	return nil
}

func (h *ReposHandler) updateV2(ctx context.Context, prj string) error {
	if _, err := h.reposv2.Update(ctx, prj); err != nil {
		return fmt.Errorf("datacatalogv2: failed to update repo %s: %w", prj, err)
	}
	return nil
}

func (h *ReposHandler) updateV3(ctx context.Context, prj string) error {
	if _, err := h.reposv3.Update(ctx, prj); err != nil {
		return fmt.Errorf("datacatalogv3: failed to update repo %s: %w", prj, err)
	}
	return nil
}

func isV2(md plateaucms.Metadata) bool {
	return md.DataCatalogSchemaVersion == "" || md.DataCatalogSchemaVersion == cmsSchemaVersionV2
}

func isV3(md plateaucms.Metadata) bool {
	return md.DataCatalogSchemaVersion == cmsSchemaVersion
}

func adminContext(c echo.Context, bypassAdminRemoval, includeBeta, includeAlpha bool) {
	ctx := c.Request().Context()
	ctx = datacatalogv3.AdminContext(ctx, bypassAdminRemoval, includeBeta, includeAlpha)
	c.SetRequest(c.Request().WithContext(ctx))
}

func newFetcherV2(md plateaucms.Metadata) (*datacatalogv2adapter.Fetcher, error) {
	c, err := md.CMS()
	if err != nil {
		return nil, fmt.Errorf("datacatalogv2: failed to create cms for %s: %w", md.DataCatalogProjectAlias, err)
	}

	baseFetcher, err := datacatalogv2.NewFetcher(md.CMSBaseURL)
	if err != nil {
		return nil, fmt.Errorf("datacatalogv2: failed to create fetcher %s: %w", md.DataCatalogProjectAlias, err)
	}

	opts := datacatalogv2.FetcherDoOptions{}
	// if md.Name != "" {
	// 	opts.Subproject = md.SubPorjectAlias
	// 	opts.CityName = md.Name
	// }

	fetcher := datacatalogv2adapter.NewFetcher(baseFetcher, c, md.DataCatalogProjectAlias, opts)

	return fetcher, nil
}

func isAlpha(c echo.Context) bool {
	return c.Request().URL.Query().Has("alpha")
}
