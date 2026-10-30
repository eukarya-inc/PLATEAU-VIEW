package estat

import (
	"net/http"
	"strconv"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

// Echo registers:
//
//	GET /areas?q=渋谷&q=宇田川町&limit=10  search areas whose address components contain every q
//	GET /areas/:id/geometry               boundary of an area as GeoJSON
func Echo(g *echo.Group, f *Finder) {
	g.GET("/areas", func(c echo.Context) error {
		ctx := c.Request().Context()

		limit := 0
		if s := c.QueryParam("limit"); s != "" {
			l, err := strconv.Atoi(s)
			if err != nil {
				return echo.NewHTTPError(http.StatusBadRequest, "invalid limit value")
			}
			limit = l
		}

		areas, err := f.FindAreas(ctx, c.QueryParams()["q"], limit)
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			log.Errorfc(ctx, "estat: failed to find areas: %v", err)
			return echo.NewHTTPError(http.StatusInternalServerError, "failed to find areas")
		}
		return c.JSON(http.StatusOK, areas)
	})

	g.GET("/areas/:id/geometry", func(c echo.Context) error {
		ctx := c.Request().Context()

		geometry, err := f.FindGeometry(ctx, c.Param("id"))
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			log.Errorfc(ctx, "estat: failed to find geometry: %v", err)
			return echo.NewHTTPError(http.StatusInternalServerError, "failed to find geometry")
		}
		if geometry == nil {
			return echo.NewHTTPError(http.StatusNotFound, "area not found")
		}
		return c.JSON(http.StatusOK, geometry)
	})
}
