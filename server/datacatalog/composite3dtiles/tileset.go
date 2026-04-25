package composite3dtiles

import "sort"

type Tileset struct {
	Asset          Asset   `json:"asset"`
	GeometricError float64 `json:"geometricError"`
	Root           Tile    `json:"root"`
}

type Asset struct {
	Version string `json:"version"`
}

type Tile struct {
	BoundingVolume BoundingVolume `json:"boundingVolume"`
	GeometricError float64        `json:"geometricError"`
	Refine         string         `json:"refine,omitempty"`
	Content        *Content       `json:"content,omitempty"`
	Children       []Tile         `json:"children,omitempty"`
}

type BoundingVolume struct {
	Region [6]float64 `json:"region"`
}

type Content struct {
	URI string `json:"uri"`
}

const (
	rootGeometricError  = 50000
	childGeometricError = 500
)

// Build assembles a tileset.json from selected candidates. Candidates whose
// area code is missing from govpolygon are silently skipped. The root
// boundingVolume.region is the union of selected children, falling back to
// the whole-Japan extent when there are no children.
func Build(candidates []Candidate, typeCode string) Tileset {
	children := make([]Tile, 0, len(candidates))
	for _, c := range candidates {
		region, ok := RegionFor(c.AreaCode, typeCode)
		if !ok {
			continue
		}
		children = append(children, Tile{
			BoundingVolume: BoundingVolume{Region: region},
			GeometricError: childGeometricError,
			Content:        &Content{URI: c.URL},
		})
	}

	sort.Slice(children, func(i, j int) bool {
		return children[i].Content.URI < children[j].Content.URI
	})

	rootRegion := japanRegion
	if len(children) > 0 {
		rootRegion = children[0].BoundingVolume.Region
		for _, ch := range children[1:] {
			rootRegion = unionRegion(rootRegion, ch.BoundingVolume.Region)
		}
	}

	return Tileset{
		Asset:          Asset{Version: "1.1"},
		GeometricError: rootGeometricError,
		Root: Tile{
			BoundingVolume: BoundingVolume{Region: rootRegion},
			GeometricError: rootGeometricError,
			Refine:         "ADD",
			Children:       children,
		},
	}
}

func unionRegion(a, b [6]float64) [6]float64 {
	return [6]float64{
		min2(a[0], b[0]),
		min2(a[1], b[1]),
		max2(a[2], b[2]),
		max2(a[3], b[3]),
		min2(a[4], b[4]),
		max2(a[5], b[5]),
	}
}

func min2(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}

func max2(a, b float64) float64 {
	if a > b {
		return a
	}
	return b
}
