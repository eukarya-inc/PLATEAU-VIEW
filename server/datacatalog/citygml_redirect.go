package datacatalog

import (
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/labstack/echo/v4"
)

// CityGMLRedirectAPI returns a handler that 302-redirects to the per-city
// CityGML zip URL identified by the given spec.
//
// Path parameter format: <cityCode>-<year|latest>
// Example: /datacatalog/citygml/13101-2025/citygml.zip
//
// When year is "latest", the dataset with the newest year for the given
// cityCode is selected.
func (h *ReposHandler) CityGMLRedirectAPI() echo.HandlerFunc {
	return func(c echo.Context) error {
		cityCode, year, latest, err := parseCityGMLSpec(c.Param("spec"))
		if err != nil {
			return echo.NewHTTPError(http.StatusBadRequest, err.Error())
		}

		merged, err := h.prepareMergedRepo(c, false)
		if err != nil {
			return err
		}

		ctx := c.Request().Context()
		simple, err := FetchSimplePlateauDatasets(ctx, merged, h.host)
		if err != nil {
			return err
		}

		var match *SimpleCityGMLDataset
		for _, d := range simple.CityGML {
			if d == nil || d.CityCode != cityCode || d.URL == "" {
				continue
			}
			if !latest && d.Year != year {
				continue
			}
			if match == nil || d.Year > match.Year {
				match = d
			}
		}
		if match == nil {
			return echo.NewHTTPError(http.StatusNotFound, "no matching CityGML dataset")
		}

		return c.Redirect(http.StatusFound, match.URL)
	}
}

func parseCityGMLSpec(s string) (cityCode string, year int, latest bool, err error) {
	parts := strings.SplitN(s, "-", 2)
	if len(parts) != 2 {
		err = fmt.Errorf("invalid spec %q: expected <cityCode>-<year|latest>", s)
		return
	}
	cityCode = parts[0]
	if len(cityCode) != 5 {
		err = fmt.Errorf("invalid spec %q: cityCode must be 5 digits", s)
		return
	}
	if _, e := strconv.Atoi(cityCode); e != nil {
		err = fmt.Errorf("invalid spec %q: cityCode must be numeric", s)
		return
	}
	if parts[1] == "latest" {
		latest = true
		return
	}
	y, e := strconv.Atoi(parts[1])
	if e != nil || y < 1900 || y > 9999 {
		err = fmt.Errorf("invalid spec %q: invalid year", s)
		return
	}
	year = y
	return
}
