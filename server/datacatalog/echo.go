package datacatalog

import (
	"context"
	"fmt"
	"net/http"
	"strconv"

	"github.com/eukarya-inc/reearth-plateauview/server/datacatalog/datacatalogv2"
	"github.com/eukarya-inc/reearth-plateauview/server/plateaucms"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

type Config struct {
	plateaucms.Config
	// v3
	CacheUpdateKey       string
	PlaygroundEndpoint   string
	GraphqlMaxComplexity int
	ErrorOnInit          bool
	GeocodingAppID       string
	DiskCache            bool // for debugging
	Debug                bool // for debugging
	// v2
	DisableCache bool
	CacheTTL     int
}

func Echo(conf Config, g *echo.Group) error {
	pcms, err := plateaucms.New(conf.Config)
	if err != nil {
		return fmt.Errorf("failed to initialize plateau cms: %w", err)
	}

	// data catalog API
	updateCache, err := echov3(conf, g, pcms)
	if err != nil {
		return fmt.Errorf("failed to initialize datacatalog v3 repo: %w", err)
	}

	// attrs API
	g.GET("/attrs/:v", attrsHandler(pcms))

	// compat: PLATEAU VIEW 2.0 data catalog API
	err = datacatalogv2.Echo(datacatalogv2.Config{
		Config:       conf.Config,
		DisableCache: conf.DisableCache,
		CacheTTL:     conf.CacheTTL,
	}, g)
	if err != nil {
		return fmt.Errorf("failed to initialize datacatalog v2 API: %w", err)
	}

	// first cache update
	if err := updateCache(context.Background()); err != nil {
		if conf.ErrorOnInit {
			return err
		} else {
			log.Errorf("datacatalog: failed to update cache: %v", err)
		}
	}

	return nil
}

func attrsHandler(pcms *plateaucms.CMS) echo.HandlerFunc {
	return func(c echo.Context) error {
		ctx := c.Request().Context()
		v := c.Param("v")
		vint, err := strconv.Atoi(v)
		if err != nil {
			return echo.NewHTTPError(http.StatusNotFound, "not found")
		}

		body, err := FetchAttrList(ctx, pcms, http.DefaultClient, vint)
		if err != nil {
			return echo.NewHTTPError(http.StatusInternalServerError, "failed to fetch attribute list")
		}
		if body == nil {
			return echo.NewHTTPError(http.StatusNotFound, "not found")
		}

		defer body.Close()
		return c.Stream(http.StatusOK, "text/csv", body)
	}
}
