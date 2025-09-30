package lodstat

import (
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

type Config struct {
	DataCatalogAPIURL   string
	DataCatalogAPIToken string
}

func Echo(conf Config, g *echo.Group) error {
	apiClient, err := NewAPIClient(conf)
	if err != nil {
		return fmt.Errorf("failed to initialize sdk client: %w", err)
	}

	g.GET("mvt/:ft/:level/tilejson.json", func(c echo.Context) error {
		featureType := c.Param("ft")
		levelStr := c.Param("level")
		level, err := strconv.Atoi(levelStr)
		if err != nil || !isValidLevel(level) {
			return echo.ErrNotFound
		}

		host := c.Request().Host
		b, err := tilesetJSON(host, featureType, level)
		if err != nil {
			log.Errorf("lodstat: failed to create tileset json: %v", err)
			return echo.ErrInternalServerError
		}

		return c.Blob(http.StatusOK, "application/json", b)
	})

	g.GET("mvt/:ft/:level/:z/:x/:y", func(c echo.Context) error {
		const ext = ".mvt"

		ctx := c.Request().Context()
		featureType := c.Param("ft")
		levelStr := c.Param("level")
		zStr := c.Param("z")
		xStr := c.Param("x")
		yStr := c.Param("y")
		if !strings.HasSuffix(yStr, ext) {
			return echo.ErrNotFound
		}

		yStr = strings.TrimSuffix(yStr, ext)
		level, err := strconv.Atoi(levelStr)
		if err != nil || !isValidLevel(level) {
			return echo.ErrNotFound
		}

		z, err := strconv.Atoi(zStr)
		if err != nil {
			return echo.ErrNotFound
		}

		if z < minZoom {
			return echo.ErrNotFound
		}

		x, err := strconv.Atoi(xStr)
		if err != nil {
			return echo.ErrNotFound
		}

		y, err := strconv.Atoi(yStr)
		if err != nil {
			return echo.ErrNotFound
		}

		b, err := renderMVT(ctx, apiClient, featureType, level, z, x, y)
		if err != nil {
			log.Errorf("lodstat: failed to render mvt: %v", err)
			return echo.ErrInternalServerError
		}

		return c.Blob(http.StatusOK, "application/vnd.mapbox-vector-tile", b)
	})

	return nil
}

func isValidLevel(level int) bool {
	return level == 2 || level == 3 || level == 4
}
