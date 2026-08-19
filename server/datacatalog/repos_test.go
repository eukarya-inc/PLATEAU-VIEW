package datacatalog

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/govpolygon"
	"github.com/labstack/echo/v4"
	geojson "github.com/paulmach/go.geojson"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// newCityQuadtree builds a quadtree with n tiny city polygons packed inside
// level-1 mesh 5339 (lat 35.333–36.0, lng 139–140) so that a single mesh query
// intersects all of them.
func newCityQuadtree(n int) *govpolygon.Quadtree {
	features := make([]*geojson.Feature, 0, n)
	for i := range n {
		lng := 139.0 + float64(i)*0.001
		lat := 35.5
		f := geojson.NewPolygonFeature([][][]float64{{
			{lng, lat},
			{lng + 0.0005, lat},
			{lng + 0.0005, lat + 0.0005},
			{lng, lat + 0.0005},
			{lng, lat},
		}})
		f.Properties["code"] = fmt.Sprintf("%05d", 13000+i)
		features = append(features, f)
	}
	return govpolygon.NewQuadtree(features, 0)
}

// A query that resolves to more cities than maxCities must be rejected with 400
// before any upstream CMS/CSV fan-out happens. This covers the mesh path (m),
// the rectangle path (r) — which only resolves cities once
// parseCityGMLFilesQuery appends its bound — and an explicit comma-separated
// city id list, which bypasses the quadtree entirely but fans out the same way.
func TestCityGMLFiles_RejectsTooManyCities(t *testing.T) {
	explicit := make([]string, 0, 60)
	for i := range 60 {
		explicit = append(explicit, fmt.Sprintf("%05d", 13000+i))
	}

	tests := []struct {
		name       string
		conditions string
	}{
		{"mesh", "m:5339"},                       // level-1 mesh covering all 60 cities
		{"rectangle", "r:139.0,35.4,139.1,35.6"}, // rectangle covering all 60 cities
		{"explicit city ids", strings.Join(explicit, ",")},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			h := &ReposHandler{
				qt:        newCityQuadtree(60),
				maxCities: 50,
			}

			e := echo.New()
			req := httptest.NewRequest(http.MethodGet, "/", nil)
			rec := httptest.NewRecorder()
			c := e.NewContext(req, rec)
			c.SetParamNames(conditionsParamName)
			c.SetParamValues(tt.conditions)

			err := h.CityGMLFiles(false)(c)

			require.Error(t, err)
			he, ok := err.(*echo.HTTPError)
			require.True(t, ok, "expected *echo.HTTPError, got %T", err)
			assert.Equal(t, http.StatusBadRequest, he.Code)
		})
	}
}

// A city id list within the cap must pass the guard: the request then fails on
// repo resolution (there is no CMS metadata in this test), not with 400.
func TestCityGMLFiles_AllowsCityIDsWithinCap(t *testing.T) {
	ids := make([]string, 0, 10)
	for i := range 10 {
		ids = append(ids, fmt.Sprintf("%05d", 13000+i))
	}

	h := &ReposHandler{
		qt:        newCityQuadtree(0),
		maxCities: 50,
	}

	e := echo.New()
	req := httptest.NewRequest(http.MethodGet, "/", nil)
	rec := httptest.NewRecorder()
	c := e.NewContext(req, rec)
	c.SetParamNames(conditionsParamName)
	c.SetParamValues(strings.Join(ids, ","))

	err := h.CityGMLFiles(false)(c)

	require.Error(t, err)
	he, ok := err.(*echo.HTTPError)
	require.True(t, ok, "expected *echo.HTTPError, got %T", err)
	assert.Equal(t, http.StatusNotFound, he.Code)
}
