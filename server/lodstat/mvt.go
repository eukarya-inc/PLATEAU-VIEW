package lodstat

import (
	"context"
	"fmt"
	"math"
	"net/url"
	"sort"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/geo"
	"github.com/paulmach/orb"
	"github.com/paulmach/orb/encoding/mvt"
	"github.com/paulmach/orb/geojson"
)

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
	codes := make([]string, 0, len(lc.Codes))
	for code := range lc.Codes {
		codes = append(codes, code)
	}
	sort.Strings(codes)
	for _, code := range codes {
		m := lc.Codes[code]
		properties := lc.Properties(code, featureType)
		if properties == nil {
			continue
		}

		minP := toTileLocal(m.Bounds.Min, z, x, y)
		maxP := toTileLocal(m.Bounds.Max, z, x, y)
		f := geojson.NewFeature(orb.Bound{
			Min: orb.Point{minP.X, minP.Y},
			Max: orb.Point{maxP.X, maxP.Y},
		}.ToPolygon())
		f.Properties = properties
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
