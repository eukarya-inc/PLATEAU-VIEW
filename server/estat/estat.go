// Package estat serves e-Stat small-area (町丁・字等) search and boundaries.
// The data lives in Firestore at api/estat/areas, which was populated for the
// former Node.js geo service; the queries here mirror that service so the
// existing composite indexes keep working.
package estat

import (
	"context"
	"slices"
	"strings"
)

const (
	defaultLimit = 100
	// maxLimit caps the client-controlled limit. Each query fetches limit*2
	// documents, so an unbounded value would pull millions of documents into
	// memory.
	maxLimit = 1000
	// maxDisjunctions is Firestore's limit on the product of "in" filter sizes.
	maxDisjunctions = 30
)

var areaPropertyKeys = []string{"PREF_NAME", "GST_NAME", "CSS_NAME", "S_NAME", "PREF", "CITY"}

var selectFields = append(prefixed("properties.", areaPropertyKeys), "bbox")

var searchFields = []string{"shortAddress", "middleAddress", "fullAddress"}

var compoundSearchFields = [][]string{
	{"properties.S_NAME", "properties.CSS_NAME", "properties.GST_NAME", "properties.PREF_NAME"},
	{"properties.S_NAME", "properties.CSS_NAME", "properties.GST_NAME"},
	{"properties.S_NAME", "properties.CSS_NAME", "properties.PREF_NAME"},
	{"properties.S_NAME", "properties.GST_NAME", "properties.PREF_NAME"},
	{"properties.S_NAME", "properties.CSS_NAME"},
	{"properties.S_NAME", "properties.GST_NAME"},
	{"properties.S_NAME", "properties.PREF_NAME"},
	{"properties.CSS_NAME", "properties.GST_NAME", "properties.PREF_NAME"},
	{"properties.CSS_NAME", "properties.PREF_NAME"},
	{"properties.GST_NAME", "properties.PREF_NAME"},
	{"properties.S_NAME"},
	{"properties.CSS_NAME"},
	{"properties.GST_NAME"},
	{"properties.PREF_NAME"},
}

type Area struct {
	ID                string    `json:"id"`
	PrefectureCode    string    `json:"prefectureCode"`
	MunicipalityCode  string    `json:"municipalityCode"`
	Name              string    `json:"name"`
	Address           string    `json:"address"`
	AddressComponents []string  `json:"addressComponents"`
	BBox              []float64 `json:"bbox"`
}

type AreaGeometry struct {
	ID       string   `json:"id"`
	Geometry Geometry `json:"geometry"`
}

// Geometry is a GeoJSON Polygon or MultiPolygon.
type Geometry struct {
	Type        string `json:"type"`
	Coordinates any    `json:"coordinates"`
}

// areaDoc is the subset of an area document read by searches.
type areaDoc struct {
	ID         string
	Properties map[string]any
	BBox       []float64
}

// Store runs the Firestore queries the search is built from.
type Store interface {
	// findIn returns documents where each field is in tokens, ordered by
	// household count (properties.SETAI) descending.
	findIn(ctx context.Context, fields, tokens []string, limit int) ([]areaDoc, error)
	// findPrefix returns documents whose field starts with prefix.
	findPrefix(ctx context.Context, field, prefix string, limit int) ([]areaDoc, error)
	// findGeometry returns the packed geometry of a document, or nil if the
	// document does not exist.
	findGeometry(ctx context.Context, id string) (map[string]any, error)
}

type Finder struct {
	store Store
}

func NewFinder(s Store) *Finder {
	return &Finder{store: s}
}

func (f *Finder) FindAreas(ctx context.Context, searchTokens []string, limit int) ([]Area, error) {
	if limit < 1 {
		limit = defaultLimit
	}
	limit = min(limit, maxLimit)

	searchTokens = slices.DeleteFunc(slices.Clone(searchTokens), func(s string) bool { return s == "" })
	if len(searchTokens) == 0 {
		return []Area{}, nil
	}

	result := []Area{}
	seen := map[string]struct{}{}
	for _, fields := range compoundSearchFields {
		disjunctions := 1
		filterFields := make([]string, 0, len(fields))
		for _, field := range fields {
			disjunctions *= len(searchTokens)
			if disjunctions > maxDisjunctions {
				break
			}
			filterFields = append(filterFields, field)
		}

		// Double the limit because some documents are filtered out below.
		docs, err := f.store.findIn(ctx, filterFields, searchTokens, limit*2)
		if err != nil {
			return nil, err
		}
		for _, a := range createAreas(docs, searchTokens) {
			if _, ok := seen[a.ID]; ok {
				continue
			}
			seen[a.ID] = struct{}{}
			result = append(result, a)
		}
		if len(result) >= limit {
			return result[:limit], nil
		}
	}
	if len(result) > 0 {
		return result, nil
	}

	searchToken := slices.MaxFunc(searchTokens, func(a, b string) int {
		return len([]rune(a)) - len([]rune(b))
	})
	for _, field := range searchFields {
		docs, err := f.store.findPrefix(ctx, field, searchToken, limit)
		if err != nil {
			return nil, err
		}
		if len(docs) > 0 {
			return createAreas(docs, nil), nil
		}
	}
	return []Area{}, nil
}

func (f *Finder) FindGeometry(ctx context.Context, id string) (*AreaGeometry, error) {
	packed, err := f.store.findGeometry(ctx, id)
	if err != nil || packed == nil {
		return nil, err
	}
	g, err := unpackGeometry(packed)
	if err != nil {
		return nil, err
	}
	return &AreaGeometry{ID: id, Geometry: *g}, nil
}

func createAreas(docs []areaDoc, searchTokens []string) []Area {
	areas := make([]Area, 0, len(docs))
	for _, d := range docs {
		components := make([]string, 0, 4)
		for _, key := range []string{"PREF_NAME", "GST_NAME", "CSS_NAME", "S_NAME"} {
			if s, ok := d.Properties[key].(string); ok {
				components = append(components, s)
			}
		}
		if !containsAll(components, searchTokens) {
			continue
		}
		pref := stringProp(d.Properties, "PREF")
		areas = append(areas, Area{
			ID:                d.ID,
			PrefectureCode:    pref,
			MunicipalityCode:  pref + stringProp(d.Properties, "CITY"),
			Name:              stringProp(d.Properties, "S_NAME"),
			Address:           strings.Join(components, ""),
			AddressComponents: components,
			BBox:              d.BBox,
		})
	}
	return areas
}

func containsAll(components, tokens []string) bool {
	for _, t := range tokens {
		if !slices.Contains(components, t) {
			return false
		}
	}
	return true
}

func stringProp(props map[string]any, key string) string {
	s, _ := props[key].(string)
	return s
}

func prefixed(prefix string, keys []string) []string {
	res := make([]string, len(keys))
	for i, k := range keys {
		res[i] = prefix + k
	}
	return res
}
