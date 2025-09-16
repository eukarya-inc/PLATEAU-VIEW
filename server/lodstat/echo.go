package lodstat

import (
	"fmt"
	"math"
	"net/http"
	"net/url"
	"strconv"
	"strings"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/geo"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/geo/jisx0410"
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
	sdkClient, err := NewAPIClient(conf)
	if err != nil {
		return fmt.Errorf("failed to initialize sdk client: %w", err)
	}

	g.GET("/:ft/:level/:z/:x/:y", func(c echo.Context) error {
		const ext = ".mvt"
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
		if err != nil {
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
		condition := fmt.Sprintf("s:%d/%d/%d", z, x, y)
		cityFiles, err := sdkClient.QueryDatasetFilesAll(c.Request().Context(), url.PathEscape(condition))
		if err != nil {
			log.Errorf("failed to query dataset files: %v", err)
			return echo.ErrInternalServerError
		}

		features := map[string]jisx0410.MeshCode{}
		lod0Count := map[string]int{}
		lod1Count := map[string]int{}
		lod2Count := map[string]int{}
		lod3Count := map[string]int{}
		lod4Count := map[string]int{}
		for _, cityFile := range cityFiles {
			for ft, gmlFiles := range cityFile {
				if featureType != "all" && ft != featureType {
					continue
				}
				for _, file := range gmlFiles {
					mesh, err := jisx0410.Parse(file.Code)
					if err != nil {
						continue
					}
					if mesh.Level != level {
						continue
					}
					if _, ok := features[file.Code]; !ok {
						features[file.Code] = mesh
					}
					if lod := file.LOD0; lod != nil && *lod > 0 {
						lod0Count[file.Code] += *lod
					}
					if lod := file.LOD1; lod != nil && *lod > 0 {
						lod1Count[file.Code] += *lod
					}
					if lod := file.LOD2; lod != nil && *lod > 0 {
						lod2Count[file.Code] += *lod
					}
					if lod := file.LOD3; lod != nil && *lod > 0 {
						lod3Count[file.Code] += *lod
					}
					if lod := file.LOD4; lod != nil && *lod > 0 {
						lod4Count[file.Code] += *lod
					}
				}
			}
		}

		lodLayer := geojson.NewFeatureCollection()
		for code, m := range features {
			minP := toTileLocal(m.Bounds.Min, z, x, y)
			maxP := toTileLocal(m.Bounds.Max, z, x, y)
			f := geojson.NewFeature(orb.Bound{
				Min: orb.Point{minP.X, minP.Y},
				Max: orb.Point{maxP.X, maxP.Y},
			}.ToPolygon())

			if featureType != "all" {
				f.Properties["featureType"] = featureType
			}
			f.Properties["meshCode"] = code

			f.Properties["lod0"] = true
			f.Properties["lod1"] = true
			f.Properties["lod2"] = true
			f.Properties["lod3"] = false
			f.Properties["lod4"] = false

			f.Properties["lod0Count"] = lod0Count[code]
			f.Properties["lod1Count"] = lod1Count[code]
			f.Properties["lod2Count"] = lod2Count[code]
			f.Properties["lod3Count"] = lod3Count[code]
			f.Properties["lod4Count"] = lod4Count[code]
			lodLayer.Append(f)
		}
		layer := mvt.NewLayer("lodstat", lodLayer)
		layer.Version = 2
		var layers mvt.Layers
		layers = append(layers, layer)
		b, err := mvt.Marshal(layers)
		if err != nil {
			log.Errorf("failed to marshal mvt: %v", err)
			return echo.ErrInternalServerError
		}
		return c.Blob(http.StatusOK, "application/vnd.mapbox-vector-tile", b)
	})
	return nil
}

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
