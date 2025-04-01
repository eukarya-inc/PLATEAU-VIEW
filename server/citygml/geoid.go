package citygml

import (
	"fmt"
	"strconv"

	japangeoid "github.com/eukarya-inc/japan-geoid-go"
	"github.com/eukarya-inc/japan-geoid-go/gsigeoid2011"
	"github.com/labstack/echo/v4"
	"github.com/samber/lo"
)

var geoid *japangeoid.MemoryGrid

func init() {
	geoid = lo.Must(gsigeoid2011.Load())
}

func GeoidHanlder(c echo.Context) error {
	lat := c.QueryParam("lat")
	lng := c.QueryParam("lng")
	if lat == "" || lng == "" {
		return c.JSON(400, map[string]any{"error": "lat and lng are required"})
	}

	latFloat, err := strconv.ParseFloat(lat, 64)
	if err != nil {
		return c.JSON(400, map[string]any{"error": "lat must be a float"})
	}

	lngFloat, err := strconv.ParseFloat(lng, 64)
	if err != nil {
		return c.JSON(400, map[string]any{"error": "lng must be a float"})
	}

	height := geoid.GetHeight(lngFloat, latFloat)

	return c.JSON(200, map[string]any{
		"lat":          latFloat,
		"lng":          lngFloat,
		"geoid_height": height,
		// compatibility with https://api-vt.geolonia.com/api/altitude?lat=x&lng=x
		"geoid": fmt.Sprintf("%.3f", height),
	})
}
