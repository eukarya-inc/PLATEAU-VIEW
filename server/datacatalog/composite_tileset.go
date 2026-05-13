package datacatalog

import (
	"net/http"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/datacatalog/composite3dtiles"
	"github.com/labstack/echo/v4"
)

// CompositeTilesetAPI returns a handler that serves a 3D Tiles tileset.json
// referencing per-area tilesets indirectly via content.uri.
//
// Path parameter format: <area>-<type>-lod<N>[-notexture]-<year>
// Example: /datacatalog/3dtiles/all-bldg-lod2-2025/tileset.json
func (h *ReposHandler) CompositeTilesetAPI() echo.HandlerFunc {
	return func(c echo.Context) error {
		spec, err := composite3dtiles.ParseSpec(c.Param("spec"))
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
		simple, err := FetchSimplePlateauDatasets(ctx, merged, h.host)
		if err != nil {
			return err
		}

		inputs := make([]composite3dtiles.Input, 0, len(simple.Datasets))
		for _, d := range simple.Datasets {
			if d == nil {
				continue
			}
			inputs = append(inputs, composite3dtiles.Input{
				URL:           d.URL,
				Format:        d.Format,
				FormatVersion: d.FormatVersion,
				TypeCode:      d.TypeCode,
				Year:          d.Year,
				LOD:           d.LOD,
				Interior:      d.Interior,
				Texture:       d.Texture,
				PrefCode:      d.PrefCode,
				CityCode:      d.CityCode,
				WardCode:      d.WardCode,
			})
		}

		candidates := composite3dtiles.Select(inputs, spec)
		tileset := composite3dtiles.Build(candidates, spec.Type)

		return c.JSON(http.StatusOK, tileset)
	}
}
