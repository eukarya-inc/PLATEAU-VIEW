package geocoding

import (
	"net/http"
	"strconv"
	"strings"

	"github.com/labstack/echo/v4"
)

type HandlerConfig struct {
	GSIURL string
}

func Echo(g *echo.Group, conf *HandlerConfig) error {
	gsiURL := ""
	if conf != nil {
		gsiURL = conf.GSIURL
	}

	gsiClient := NewGSIClient(nil, gsiURL)

	// GET /geocoding?lon=139.7&lat=35.6&includeRadii=true
	g.GET("", func(c echo.Context) error {
		ctx := c.Request().Context()

		// Parse query parameters
		lonStr := c.QueryParam("lon")
		latStr := c.QueryParam("lat")
		includeRadiiStr := c.QueryParam("includeRadii")

		if lonStr == "" || latStr == "" {
			return echo.NewHTTPError(http.StatusBadRequest, "lon and lat are required")
		}

		lon, err := strconv.ParseFloat(lonStr, 64)
		if err != nil {
			return echo.NewHTTPError(http.StatusBadRequest, "invalid lon value")
		}

		lat, err := strconv.ParseFloat(latStr, 64)
		if err != nil {
			return echo.NewHTTPError(http.StatusBadRequest, "invalid lat value")
		}

		includeRadii := includeRadiiStr == "true" || includeRadiiStr == "1"

		// Fetch from GSI API
		result, err := gsiClient.Fetch(ctx, lon, lat)
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			return echo.NewHTTPError(http.StatusInternalServerError, "failed to fetch from GSI API")
		}

		if result == nil || result.MunicipalityCode == "" {
			return c.JSON(http.StatusOK, nil)
		}

		// Build areas from municipality code
		areas, err := BuildAreas(result.MunicipalityCode, includeRadii)
		if err != nil {
			return echo.NewHTTPError(http.StatusInternalServerError, "failed to build areas")
		}

		// Convert full-width space to half-width
		address := strings.ReplaceAll(result.Name, "\u3000", " ")

		return c.JSON(http.StatusOK, &Areas{
			Address: address,
			Areas:   areas,
		})
	})

	return nil
}
