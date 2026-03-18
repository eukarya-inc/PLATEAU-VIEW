package geocoding

import (
	"errors"
	"net/http"
	"strconv"
	"strings"

	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

type HandlerConfig struct {
	GSIURL       string
	NominatimURL string
}

func Echo(g *echo.Group, conf *HandlerConfig) error {
	gsiURL := ""
	nominatimURL := ""
	if conf != nil {
		gsiURL = conf.GSIURL
		nominatimURL = conf.NominatimURL
	}

	gsiClient := NewGSIClient(nil, gsiURL)
	nominatimClient := NewNominatimClient(nil, nominatimURL, "")

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
		usedFallback := false
		if err != nil {
			if ctx.Err() != nil {
				return nil
			}
			// Fallback to Nominatim when GSI API is unavailable
			if errors.Is(err, ErrGSIUnavailable) {
				log.Warnfc(ctx, "geocoding: GSI API unavailable (lon=%v, lat=%v): %v, falling back to Nominatim", lon, lat, err)
				result, err = nominatimClient.Fetch(ctx, lon, lat)
				if err != nil {
					if ctx.Err() != nil {
						return nil
					}
					log.Errorfc(ctx, "geocoding: Nominatim fallback also failed (lon=%v, lat=%v): %v", lon, lat, err)
					return echo.NewHTTPError(http.StatusBadGateway, "geocoding services are temporarily unavailable")
				}
				usedFallback = true
				log.Infofc(ctx, "geocoding: successfully used Nominatim fallback (lon=%v, lat=%v)", lon, lat)
			} else {
				log.Errorfc(ctx, "geocoding: GSI API error (lon=%v, lat=%v): %v", lon, lat, err)
				return echo.NewHTTPError(http.StatusInternalServerError, "failed to fetch from GSI API")
			}
		}

		if result != nil && result.MunicipalityCode != "" {
			if usedFallback {
				log.Debugfc(ctx, "geocoding: resolved via Nominatim: code=%s, name=%s", result.MunicipalityCode, result.Name)
			} else {
				log.Debugfc(ctx, "geocoding: resolved via GSI: code=%s, name=%s", result.MunicipalityCode, result.Name)
			}
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
