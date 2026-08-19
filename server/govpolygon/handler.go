package govpolygon

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"sync"
	"time"

	"github.com/eukarya-inc/jpareacode"
	"github.com/eukarya-inc/jpareacode/jpareacodepref"
	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
	geojson "github.com/paulmach/go.geojson"
	"github.com/reearth/reearthx/log"
	"github.com/reearth/reearthx/util"
	"github.com/samber/lo"
	"golang.org/x/sync/singleflight"
)

var cacheDuration = 6 * time.Hour

// failureCacheDuration is how long a failed update is remembered.
// Without it, every queued request would re-issue the same expensive query
// against a data catalog that is already struggling.
var failureCacheDuration = 1 * time.Minute

// errUpdateRecentlyFailed is returned when an update is skipped because a
// recent attempt failed.
var errUpdateRecentlyFailed = errors.New("update recently failed")

type Handler struct {
	// e.g. "http://[::]:8080"
	gqlEndpoint       string
	httpClient        *http.Client
	lock              sync.RWMutex
	group             singleflight.Group
	geojson           []byte
	qt                *Quadtree
	updateIfNotExists bool
	updatedAt         time.Time
	failedAt          time.Time
}

func New(gqlEndpoint string, updateIfNotExists bool) *Handler {
	return &Handler{
		gqlEndpoint:       gqlEndpoint,
		httpClient:        http.DefaultClient,
		updateIfNotExists: updateIfNotExists,
	}
}

func (h *Handler) Route(g *echo.Group) *Handler {
	g.Use(middleware.CORS(), middleware.Gzip())
	g.GET("/plateaugovs.geojson", h.GetGeoJSON)
	g.GET("/geocoding", h.FindCodeFromLngLat)
	// g.GET("/update", h.Update, errorLogger)
	return h
}

func (h *Handler) updateIfNeed(c echo.Context) {
	if !h.updateIfNotExists {
		return
	}

	h.lock.RLock()
	exists := h.geojson != nil
	h.lock.RUnlock()
	if exists {
		return
	}

	if err := h.Update(c); err != nil && !errors.Is(err, errUpdateRecentlyFailed) {
		log.Errorfc(c.Request().Context(), "govpolygon: fail to init: %v", err)
	}
}

func (h *Handler) GetGeoJSON(c echo.Context) error {
	h.updateIfNeed(c)

	h.lock.RLock()
	defer h.lock.RUnlock()
	if h.geojson == nil {
		return c.JSON(http.StatusNotFound, "not found")
	}
	return c.JSONBlob(http.StatusOK, h.geojson)
}

func (h *Handler) Update(c echo.Context) error {
	ctx := c.Request().Context()

	// Only one update runs at a time; concurrent requests share its result
	// instead of each issuing the same expensive catalog query.
	_, err, _ := h.group.Do("update", func() (any, error) {
		return nil, h.update(ctx)
	})

	return err
}

func (h *Handler) update(ctx context.Context) error {
	h.lock.RLock()
	updatedAt, failedAt := h.updatedAt, h.failedAt
	h.lock.RUnlock()

	now := util.Now()
	if !updatedAt.IsZero() && now.Sub(updatedAt) < cacheDuration {
		return nil
	}
	if !failedAt.IsZero() && now.Sub(failedAt) < failureCacheDuration {
		return errUpdateRecentlyFailed
	}

	log.Infofc(ctx, "govpolygon: updating")

	// The query and the computation are intentionally done without holding the
	// write lock: it is taken only to swap the results in.
	geojsonj, qt, err := h.compute(ctx)
	if err != nil {
		h.lock.Lock()
		h.failedAt = util.Now()
		h.lock.Unlock()
		return err
	}

	h.lock.Lock()
	defer h.lock.Unlock()
	h.geojson = geojsonj
	h.qt = qt
	h.updatedAt = util.Now()
	h.failedAt = time.Time{}

	return nil
}

func (h *Handler) compute(ctx context.Context) ([]byte, *Quadtree, error) {
	q, err := h.getCityNames(ctx)
	if err != nil {
		return nil, nil, err
	}

	g, notfound, err := ComputeGeoJSON(q)
	if err != nil {
		return nil, nil, err
	}
	if len(notfound) > 0 {
		log.Debugfc(ctx, "govpolygon: not found polygon: %v", notfound)
	}

	fc := geojson.NewFeatureCollection()
	for _, f := range g {
		fc.AddFeature(f)
	}

	geojsonj, err := json.Marshal(fc)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to marshal geojson: %w", err)
	}

	return geojsonj, NewQuadtree(g, 0), nil
}

func (h *Handler) getCityNames(ctx context.Context) ([]string, error) {
	query := `
		{
			areas(input:{
				areaTypes: [CITY, WARD]
			}) {
				name
				code
				... on City {
					prefecture {
						name
					}
				}
				... on Ward {
					prefecture {
						name
					}
					city {
						name
					}
				}
			}
		}
	`

	requestBody, err := json.Marshal(map[string]string{
		"query": query,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request body: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, h.gqlEndpoint, bytes.NewBuffer(requestBody))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")
	resp, err := h.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to send request: %w", err)
	}

	defer func() { _ = resp.Body.Close() }()
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response body: %w", err)
	}

	var responseData struct {
		Data struct {
			Areas []struct {
				Name       string `json:"name"`
				Code       string `json:"code"`
				Prefecture struct {
					Name string `json:"name"`
				} `json:"prefecture"`
				City struct {
					Name string `json:"name"`
				} `json:"city"`
			} `json:"areas"`
		} `json:"data"`
	}

	if err := json.Unmarshal(body, &responseData); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response body: %w", err)
	}

	names := make([]string, len(responseData.Data.Areas))
	for i, area := range responseData.Data.Areas {
		if area.City.Name == "東京都23区" {
			area.City.Name = ""
		}

		if area.City.Name != "" {
			names[i] = area.Prefecture.Name + "/" + area.City.Name + "/" + area.Name
		} else if area.Prefecture.Name != area.Name {
			names[i] = area.Prefecture.Name + "/" + area.Name
		} else {
			names[i] = area.Name
		}
	}

	return names, nil
}

func (h *Handler) FindCodeFromLngLat(c echo.Context) error {
	h.updateIfNeed(c)

	lngs, lats := c.QueryParam("lng"), c.QueryParam("lat")
	if lngs == "" || lats == "" {
		return c.JSON(http.StatusBadRequest, "lng and lat are required")
	}

	lng, err := strconv.ParseFloat(lngs, 64)
	if err != nil {
		return c.JSON(http.StatusBadRequest, "invalid lng")
	}

	lat, err := strconv.ParseFloat(lats, 64)
	if err != nil {
		return c.JSON(http.StatusBadRequest, "invalid lat")
	}

	h.lock.RLock()
	defer h.lock.RUnlock()
	if h.qt == nil {
		return c.JSON(http.StatusNotFound, "not found")
	}

	code, _ := h.qt.Find(lng, lat)
	city := jpareacode.CityByCodeString(code)

	if city == nil {
		return c.JSON(http.StatusOK, map[string]any{
			"lng": lng,
			"lat": lat,
		})
	}

	return c.JSON(http.StatusOK, map[string]any{
		"lng":      lng,
		"lat":      lat,
		"pref":     jpareacodepref.PrefectureNameByCodeInt(city.PrefCode),
		"prefCode": jpareacodepref.FormatPrefectureCode(city.PrefCode),
		"city":     lo.EmptyableToPtr(city.CityName),
		"cityCode": lo.EmptyableToPtr(jpareacode.FormatCityCode(city.CityCode)),
		"ward":     lo.EmptyableToPtr(city.WardName),
		"wardCode": lo.EmptyableToPtr(jpareacode.FormatCityCode(city.WardCode)),
		"code":     jpareacode.FormatCityCode(city.Code()),
	})
}
