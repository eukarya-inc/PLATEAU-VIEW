package datacatalog

import (
	"net/http"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/mvttilejson"
	"github.com/labstack/echo/v4"
)

// MVTTileJSONAPI returns a handler that serves a dynamically generated
// TileJSON 3.0 document for a single municipality's MVT dataset.
//
// Path parameter format: <cityCode>-<type>[-lod<N>]-<year|latest>
// Example: /datacatalog/mvt/13101-luse-2025/tilejson.json
func (h *ReposHandler) MVTTileJSONAPI() echo.HandlerFunc {
	return func(c echo.Context) error {
		spec, err := mvttilejson.ParseSpec(c.Param("spec"))
		if err != nil {
			return echo.NewHTTPError(http.StatusBadRequest, err.Error())
		}

		merged, err := h.prepareMergedRepo(c, false)
		if err != nil {
			return err
		}

		if _, matched := setRevisionETag(c, merged.Revision()); matched {
			return c.NoContent(http.StatusNotModified)
		}

		ctx := c.Request().Context()
		simple, err := h.fetchSimplePlateauDatasets(ctx, merged, h.host)
		if err != nil {
			return err
		}

		inputs := make([]mvttilejson.Input, 0, len(simple.Datasets))
		for _, d := range simple.Datasets {
			if d == nil {
				continue
			}
			var city, ward string
			if d.City != nil {
				city = *d.City
			}
			if d.Ward != nil {
				ward = *d.Ward
			}
			inputs = append(inputs, mvttilejson.Input{
				Name:     d.Name,
				URL:      d.URL,
				Format:   d.Format,
				TypeCode: d.TypeCode,
				TypeName: d.Type,
				Year:     d.Year,
				LOD:      d.LOD,
				Interior: d.Interior,
				Layers:   d.Layers,
				PrefCode: d.PrefCode,
				CityCode: d.CityCode,
				WardCode: d.WardCode,
				Pref:     d.Pref,
				City:     city,
				Ward:     ward,
				Spec:     d.Spec,
			})
		}

		match := mvttilejson.Select(inputs, spec)
		if match == nil {
			return echo.NewHTTPError(http.StatusNotFound, "no matching MVT dataset")
		}

		return c.JSON(http.StatusOK, mvttilejson.Build(*match))
	}
}
