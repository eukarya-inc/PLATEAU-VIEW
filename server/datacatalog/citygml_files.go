package datacatalog

import (
	"context"
	"errors"
	"fmt"
	"strings"

	"github.com/JamesLMilner/quadtree-go"
	"github.com/eukarya-inc/reearth-plateauview/server/datacatalog/geocoding"
	"github.com/eukarya-inc/reearth-plateauview/server/geo"
	"github.com/eukarya-inc/reearth-plateauview/server/geo/jisx0410"
	"github.com/eukarya-inc/reearth-plateauview/server/geo/spatialid"
	"github.com/eukarya-inc/reearth-plateauview/server/govpolygon"
	"github.com/reearth/reearthx/rerror"
	"github.com/samber/lo"
)

type GeoCoder func(ctx context.Context, address string) (quadtree.Bounds, error)

const maxBounds = 30

type CityGMLFilesResponse struct {
	Cities       []*CityGMLFilesCity           `json:"cities"`
	FeatureTypes map[string]CityGMLFeatureType `json:"featureTypes"`
}

func ParseCityGMLFilesQuery(ctx context.Context, conditions string, qt *govpolygon.Quadtree, geocoder GeoCoder) (cityIDs []string, filter cityGMLFileFilterFunc, err error) {
	switch conditionType, cond := parseConditions(conditions); conditionType {
	case "m":
		var bounds []geo.Bounds2
		for _, m := range strings.Split(cond, ",") {
			b, err := jisx0410.Parse(m)
			if err != nil {
				return nil, nil, fmt.Errorf("invalid mesh: %w", err)
			}
			bounds = append(bounds, b.Bounds)
			cityIDs = append(cityIDs, qt.FindRect(b.Bounds.QBounds())...)
		}
		if len(bounds) > maxBounds {
			return nil, nil, fmt.Errorf("too many bounds")
		}
		filter = boundsCityGMLFileFilter(bounds)
	case "mm":
		var bounds []geo.Bounds2
		var levels [7]int
		for _, m := range strings.Split(cond, ",") {
			b, err := jisx0410.Parse(m)
			if err != nil {
				return nil, nil, fmt.Errorf("invalid mesh: %w", err)
			}
			if b.Level == 0 {
				return nil, nil, fmt.Errorf("unsupported mesh: %s", m)
			}
			levels[b.Level]++
			bounds = append(bounds, b.Bounds)
			cityIDs = append(cityIDs, qt.FindRect(b.Bounds.QBounds())...)
		}
		if len(bounds) > maxBounds {
			return nil, nil, fmt.Errorf("too many bounds")
		}
		switch {
		case levels[2] == len(bounds):
			filter = levelCityGMLFileFilter(2, bounds)
		case levels[3] == len(bounds):
			filter = levelCityGMLFileFilter(3, bounds)
		default:
			return nil, nil, fmt.Errorf("bounds for different levels: %v", levels)
		}
	case "s":
		var bounds []geo.Bounds2
		for _, s := range strings.Split(cond, ",") {
			b3, err := spatialid.Bounds(s)
			if err != nil {
				return nil, nil, fmt.Errorf("invalid spatial id: %w", err)
			}
			b := b3.ToXY()
			bounds = append(bounds, b)
			cityIDs = qt.FindRect(b.QBounds())
		}
		if len(bounds) > maxBounds {
			return nil, nil, fmt.Errorf("too many bounds: %d", len(bounds))
		}
		filter = boundsCityGMLFileFilter(bounds)
	case "r":
		b, err := parseBounds(cond)
		if err != nil {
			return nil, nil, fmt.Errorf("invalid rectangle: %w", err)
		}
		filter = boundsCityGMLFileFilter([]geo.Bounds2{geo.ToBounds2(b)})
		cityIDs = qt.FindRect(b)
	case "g":
		if geocoder == nil {
			return nil, nil, fmt.Errorf("invalid condition type: %s", conditionType)
		}

		b, err := geocoder(ctx, cond)
		if errors.Is(err, geocoding.ErrNotFound) {
			return nil, nil, rerror.ErrNotFound
		}
		if err != nil {
			return nil, nil, fmt.Errorf("geocoding: %w", err)
		}
		filter = boundsCityGMLFileFilter([]geo.Bounds2{geo.ToBounds2(b)})
		cityIDs = qt.FindRect(b)
	case "":
		if cond == "" {
			return nil, nil, fmt.Errorf("no conditions")
		}
		cityIDs = strings.Split(cond, ",")
	default:
		return nil, nil, fmt.Errorf("invalid condition type: %s", conditionType)
	}

	return lo.Uniq(cityIDs), filter, nil
}

func ApplyCityGMLCityFilter(ctx context.Context, cities []*CityGMLFilesCity, filter cityGMLFileFilterFunc) *CityGMLFilesResponse {
	response := &CityGMLFilesResponse{
		FeatureTypes: make(map[string]CityGMLFeatureType),
	}

	for _, city := range cities {
		if city == nil {
			continue
		}

		if filter != nil {
			for ft, cityGmlFiles := range city.Files {
				filtered := cityGmlFiles[:0]
				for _, f := range cityGmlFiles {
					if filter(f) {
						filtered = append(filtered, f)
					}
				}
				if len(filtered) == 0 {
					delete(city.Files, ft)
				} else {
					city.Files[ft] = filtered
				}
			}
		}

		for code := range city.Files {
			if _, ok := response.FeatureTypes[code]; ok {
				continue
			}
			for code2, ft := range city.FeatureTypes {
				if code == code2 {
					response.FeatureTypes[code] = ft
					break
				}
			}
		}
		city.FeatureTypes = nil // simplify response

		if len(city.Files) > 0 {
			response.Cities = append(response.Cities, city)
		}
	}

	return response
}

func parseConditions(conditions string) (string, string) {
	t, body, found := strings.Cut(conditions, ":")
	if found {
		return t, body
	} else {
		return "", conditions
	}
}

type cityGMLFileFilterFunc func(CityGMLFile) bool

func boundsCityGMLFileFilter(bounds []geo.Bounds2) cityGMLFileFilterFunc {
	return func(f CityGMLFile) bool {
		m, _ := jisx0410.Parse(f.MeshCode)
		for _, b := range bounds {
			if m.Bounds.IsIntersect(b) {
				return true
			}
		}
		return false
	}
}

func levelCityGMLFileFilter(level int, bounds []geo.Bounds2) cityGMLFileFilterFunc {
	return func(f CityGMLFile) bool {
		m, _ := jisx0410.Parse(f.MeshCode)
		if level == 2 && m.Level != 2 {
			return false
		}
		if level == 3 && m.Level < 3 {
			return false
		}
		for _, b := range bounds {
			if m.Bounds.IsIntersect(b) {
				return true
			}
		}
		return false
	}
}
