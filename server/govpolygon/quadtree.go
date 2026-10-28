package govpolygon

import (
	"github.com/JamesLMilner/quadtree-go"
	geojson "github.com/paulmach/go.geojson"
)

type Quadtree struct {
	qt *quadtree.Quadtree
	ft map[quadtree.Bounds]*geojson.Feature
}

func NewQuadtree(f []*geojson.Feature, margin float64) *Quadtree {
	if f == nil {
		f = JapanCityFeatures
	}

	ft := map[quadtree.Bounds]*geojson.Feature{}
	qt := &quadtree.Quadtree{
		MaxObjects: 10,
		MaxLevels:  100,
		Objects:    make([]quadtree.Bounds, 0),
		Nodes:      make([]quadtree.Quadtree, 0),
	}

	for _, f := range f {
		b, ok := bounds(f.Geometry)
		if !ok {
			continue
		}
		b.X -= margin
		b.Y -= margin
		b.Width += margin
		b.Height += margin
		qt.Insert(b)
		ft[b] = f
	}

	return &Quadtree{
		qt: qt,
		ft: ft,
	}
}

func (q *Quadtree) Find(lng, lat float64) (string, bool) {
	res := q.qt.RetrieveIntersections(quadtree.Bounds{
		X: lng,
		Y: lat,
	})
	if len(res) == 0 {
		return "", false
	}

	for _, f := range res {
		f2 := q.ft[f]
		if isPointInPolygonFeature(lng, lat, f2) {
			code, _ := f2.Properties["code"].(string)
			if code != "" {
				return code, true
			}
		}
	}

	return "", false
}

func (q *Quadtree) FindRect(b quadtree.Bounds) []string {
	res := q.qt.RetrieveIntersections(b)
	var codes []string
	for _, b := range res {
		code, _ := q.ft[b].Properties["code"].(string)
		if code != "" {
			codes = append(codes, code)
		}
	}
	return codes
}

func bounds(g *geojson.Geometry) (b quadtree.Bounds, _ bool) {
	if !g.IsMultiPolygon() && !g.IsPolygon() {
		return
	}

	if g.IsPolygon() {
		g = &geojson.Geometry{
			Type:         "MultiPolygon",
			MultiPolygon: [][][][]float64{g.Polygon},
		}
	}

	minlat := -1.0
	minlng := -1.0
	maxlat := -1.0
	maxlng := -1.0

	for _, polygon := range g.MultiPolygon {
		for _, ring := range polygon {
			for _, p := range ring {
				lng := p[0]
				lat := p[1]

				if minlat == -1 || lat < minlat {
					minlat = lat
				}
				if minlng == -1 || lng < minlng {
					minlng = lng
				}

				if maxlat == -1 || lat > maxlat {
					maxlat = lat
				}
				if maxlng == -1 || lng > maxlng {
					maxlng = lng
				}
			}
		}
	}

	return quadtree.Bounds{
		X:      minlng,
		Y:      minlat,
		Width:  maxlng - minlng,
		Height: maxlat - minlat,
	}, true
}

func isPointInPolygonFeature(lng, lat float64, f *geojson.Feature) bool {
	if f.Geometry == nil {
		return false
	}

	// the feature is shared by concurrent lookups, so it must not be modified here
	var polygons [][][][]float64
	switch {
	case f.Geometry.IsPolygon():
		polygons = [][][][]float64{f.Geometry.Polygon}
	case f.Geometry.IsMultiPolygon():
		polygons = f.Geometry.MultiPolygon
	default:
		return false
	}

	for _, polygon := range polygons {
		if len(polygon) == 0 || !isPointInPolygon(lng, lat, polygon[0]) {
			continue
		}
		// the remaining rings are holes (e.g. an enclave of another city)
		inHole := false
		for _, hole := range polygon[1:] {
			if isPointInPolygon(lng, lat, hole) {
				inHole = true
				break
			}
		}
		if !inHole {
			return true
		}
	}

	return false
}

func isPointInPolygon(lng, lat float64, polygon [][]float64) bool {
	var count int
	plen := len(polygon)
	for i := 0; i < plen; i++ {
		j := (i + 1) % plen
		if ((polygon[i][1] > lat) != (polygon[j][1] > lat)) &&
			(lng < (polygon[j][0]-polygon[i][0])*(lat-polygon[i][1])/(polygon[j][1]-polygon[i][1])+polygon[i][0]) {
			count++
		}
	}
	return count%2 != 0
}
