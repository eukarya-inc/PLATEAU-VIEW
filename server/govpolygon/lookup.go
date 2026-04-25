package govpolygon

import (
	"sync"

	geojson "github.com/paulmach/go.geojson"
)

type BBox struct {
	MinLng, MinLat, MaxLng, MaxLat float64
}

var (
	bboxIndexOnce sync.Once
	bboxIndex     map[string]BBox
)

func buildBBoxIndex() {
	bboxIndex = make(map[string]BBox, len(JapanCityFeatures))
	for _, f := range JapanCityFeatures {
		code, ok := f.Properties["code"].(string)
		if !ok || code == "" {
			continue
		}
		bb, ok := computeBBox(f)
		if !ok {
			continue
		}
		if existing, found := bboxIndex[code]; found {
			bboxIndex[code] = mergeBBox(existing, bb)
		} else {
			bboxIndex[code] = bb
		}
	}
}

// BBoxByCode returns the bounding box (lng/lat in degrees) of the feature
// identified by the given 5-digit municipal code.
func BBoxByCode(code string) (BBox, bool) {
	bboxIndexOnce.Do(buildBBoxIndex)
	b, ok := bboxIndex[code]
	return b, ok
}

func computeBBox(f *geojson.Feature) (BBox, bool) {
	if f == nil || f.Geometry == nil {
		return BBox{}, false
	}
	g := f.Geometry
	if !g.IsMultiPolygon() && !g.IsPolygon() {
		return BBox{}, false
	}

	polys := g.MultiPolygon
	if g.IsPolygon() {
		polys = [][][][]float64{g.Polygon}
	}

	first := true
	var b BBox
	for _, polygon := range polys {
		for _, ring := range polygon {
			for _, p := range ring {
				lng, lat := p[0], p[1]
				if first {
					b = BBox{MinLng: lng, MaxLng: lng, MinLat: lat, MaxLat: lat}
					first = false
					continue
				}
				if lng < b.MinLng {
					b.MinLng = lng
				}
				if lng > b.MaxLng {
					b.MaxLng = lng
				}
				if lat < b.MinLat {
					b.MinLat = lat
				}
				if lat > b.MaxLat {
					b.MaxLat = lat
				}
			}
		}
	}
	if first {
		return BBox{}, false
	}
	return b, true
}

func mergeBBox(a, b BBox) BBox {
	out := a
	if b.MinLng < out.MinLng {
		out.MinLng = b.MinLng
	}
	if b.MinLat < out.MinLat {
		out.MinLat = b.MinLat
	}
	if b.MaxLng > out.MaxLng {
		out.MaxLng = b.MaxLng
	}
	if b.MaxLat > out.MaxLat {
		out.MaxLat = b.MaxLat
	}
	return out
}
