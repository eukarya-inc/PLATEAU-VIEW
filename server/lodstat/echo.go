package lodstat

import (
	"context"
	"fmt"
	"math"
	"net/http"
	"net/url"
	"strconv"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/geo"
	"github.com/labstack/echo/v4"
	"github.com/paulmach/orb"
	"github.com/paulmach/orb/encoding/mvt"
	"github.com/paulmach/orb/geojson"
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

	g.GET("/:ft/:level/tilejson.json", func(c echo.Context) error {
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

	g.GET("/:ft/:level/:z/:x/:y", func(c echo.Context) error {
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

func renderMVT(ctx context.Context, apiClient *APIClient, featureType string, level, z, x, y int) ([]byte, error) {
	// get lodstat data
	condition := fmt.Sprintf("s:%d/%d/%d", z, x, y)
	cityFiles, err := apiClient.QueryDatasetFilesAll(ctx, url.PathEscape(condition))
	if err != nil {
		return nil, fmt.Errorf("failed to query dataset files: %w", err)
	}

	// collect data
	lc := newLodstatContext()
	lc.CollectAll(level, featureType, cityFiles)

	// create GeoJSON
	lodLayer := geojson.NewFeatureCollection()
	for code, m := range lc.Features {
		minP := toTileLocal(m.Bounds.Min, z, x, y)
		maxP := toTileLocal(m.Bounds.Max, z, x, y)
		f := geojson.NewFeature(orb.Bound{
			Min: orb.Point{minP.X, minP.Y},
			Max: orb.Point{maxP.X, maxP.Y},
		}.ToPolygon())
		f.Properties = lc.Properties(code, featureType)
		lodLayer.Append(f)
	}

	// render MVT
	layer := mvt.NewLayer("lodstat", lodLayer)
	layer.Version = 2
	var layers mvt.Layers
	layers = append(layers, layer)
	b, err := mvt.Marshal(layers)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal mvt: %w", err)
	}

	return b, nil
}

// toTileLocal converts WGS84 coordinates to local tile coordinates (0-4096), which is Web Mercator.
func toTileLocal(p geo.Point2, z, tx, ty int) geo.Point2 {
	u := (p.X + 180.0) / 360.0
	phi := p.Y * math.Pi / 180.0
	v := (1.0 - math.Log(math.Tan(phi)+1.0/math.Cos(phi))/math.Pi) / 2.0

	fx := u*math.Exp2(float64(z)) - float64(tx)
	fy := v*math.Exp2(float64(z)) - float64(ty)

	fx *= 4096
	fy *= 4096
	return geo.Point2{X: fx, Y: fy}
}

func isValidLevel(level int) bool {
	return level == 2 || level == 3 || level == 4
}
