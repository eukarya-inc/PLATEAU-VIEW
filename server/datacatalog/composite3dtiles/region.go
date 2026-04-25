package composite3dtiles

import (
	"math"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/govpolygon"
)

type heightRange struct{ min, max float64 }

// PLATEAU 3D Tiles share roughly the same vertical scale across feature types,
// so a single default range is sufficient for top-level frustum culling.
// Terrain (dem) is the only outlier that needs a wider range.
var (
	defaultHeight = heightRange{-50, 500}
	heightByType  = map[string]heightRange{
		"dem": {-100, 4000},
	}
)

// japanRegion is the bounding region used for the root tile.
var japanRegion = [6]float64{
	deg2rad(122.0), deg2rad(20.0),
	deg2rad(154.0), deg2rad(46.0),
	-100, 4000,
}

func deg2rad(d float64) float64 { return d * math.Pi / 180 }

// RegionFor resolves the boundingVolume.region for the given municipal code
// and dataset type. Returns false when the code is not found in govpolygon.
func RegionFor(areaCode, typeCode string) ([6]float64, bool) {
	bb, ok := govpolygon.BBoxByCode(areaCode)
	if !ok {
		return [6]float64{}, false
	}
	h, ok := heightByType[typeCode]
	if !ok {
		h = defaultHeight
	}
	return [6]float64{
		deg2rad(bb.MinLng), deg2rad(bb.MinLat),
		deg2rad(bb.MaxLng), deg2rad(bb.MaxLat),
		h.min, h.max,
	}, true
}
