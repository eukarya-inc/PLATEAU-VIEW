package tiles

import (
	"context"
	_ "embed"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"sync"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

const (
	modelKey = "tiles"
	tilesTTL = 3 * time.Minute
)

//go:embed darkStyle.json
var darkStyle []byte

//go:embed lightStyle.json
var lightStyle []byte

var styles = map[string][]byte{
	"dark-map":  darkStyle,
	"light-map": lightStyle,
}

type Config struct {
	CMS     plateaucms.Config
	Host    string
	TileURL string // Redirects /tiles/* to this URL (except config.json and styles)
}

type Handler struct {
	pcms        *plateaucms.CMS
	lock        sync.RWMutex
	host        *url.URL
	tileURL     *url.URL
	tiles       Tiles
	fetchedAt   time.Time
	refreshLock sync.Mutex
	now         func() time.Time // overridable for tests
}

func New(ctx context.Context, conf Config) (*Handler, error) {
	pcms, err := plateaucms.New(conf.CMS)
	if err != nil {
		return nil, fmt.Errorf("failed to create plateau cms: %w", err)
	}

	var host, tileURL *url.URL

	if conf.Host != "" {
		host, err = url.Parse(conf.Host)
		if err != nil {
			return nil, fmt.Errorf("failed to parse host: %w", err)
		}
	}

	if conf.TileURL != "" {
		tileURL, err = url.Parse(conf.TileURL)
		if err != nil {
			return nil, fmt.Errorf("failed to parse tile url: %w", err)
		}
	}

	return &Handler{
		pcms:    pcms,
		host:    host,
		tileURL: tileURL,
		now:     time.Now,
	}, nil
}

func (h *Handler) Init(ctx context.Context) {
	if _, err := h.refresh(ctx); err != nil {
		log.Errorfc(ctx, "tiles: failed to init tiles: %v", err)
	}
}

// getTiles returns the cached tiles, refreshing from the CMS if the cache has
// expired. On refresh failure it logs and serves the stale cache so a
// transient CMS hiccup doesn't blank out /tiles/config.json.
func (h *Handler) getTiles(ctx context.Context) Tiles {
	h.lock.RLock()
	cached, fetchedAt := h.tiles, h.fetchedAt
	h.lock.RUnlock()

	if !fetchedAt.IsZero() && h.now().Sub(fetchedAt) < tilesTTL {
		return cached
	}

	tiles, err := h.refresh(ctx)
	if err != nil {
		log.Errorfc(ctx, "tiles: failed to refresh tiles: %v", err)
		return cached
	}
	return tiles
}

// refresh fetches tiles from the CMS, deduplicating concurrent callers via
// refreshLock so a stampede of /tiles/config.json requests after the TTL
// expires only fans out to the CMS once.
func (h *Handler) refresh(ctx context.Context) (Tiles, error) {
	h.refreshLock.Lock()
	defer h.refreshLock.Unlock()

	h.lock.RLock()
	fetchedAt := h.fetchedAt
	cached := h.tiles
	h.lock.RUnlock()
	if !fetchedAt.IsZero() && h.now().Sub(fetchedAt) < tilesTTL {
		return cached, nil
	}

	tiles, err := initTiles(ctx, h.pcms)
	if err != nil {
		return nil, err
	}

	h.lock.Lock()
	h.tiles = tiles
	h.fetchedAt = h.now()
	h.lock.Unlock()

	if len(tiles) == 0 {
		log.Debugfc(ctx, "tiles: no tiles found")
	} else {
		log.Debugfc(ctx, "tiles: refreshed: \n%s", tiles)
	}
	return tiles, nil
}

func (h *Handler) Route(g *echo.Group) {
	g = g.Group("/tiles")
	g.GET("/config.json", h.GetConfig)
	g.GET("/styles/:id", h.GetStyle)

	// Redirect all other requests to the tile server
	if h.tileURL != nil {
		g.Any("/*", h.redirectToTileServer)
	}
}

func (h *Handler) redirectToTileServer(c echo.Context) error {
	reqPath := c.Request().URL.Path
	subPath := strings.TrimPrefix(reqPath, "/tiles")

	redirectURL := h.tileURL.String() + "/tiles" + subPath
	if c.Request().URL.RawQuery != "" {
		redirectURL += "?" + c.Request().URL.RawQuery
	}

	return c.Redirect(http.StatusTemporaryRedirect, redirectURL)
}

// GetConfig returns the tile server configuration in JSON format
func (h *Handler) GetConfig(c echo.Context) error {
	tiles := h.getTiles(c.Request().Context())
	baseURL := h.getBaseURL(c)

	if tiles == nil {
		return c.JSON(http.StatusOK, Tiles{}.ToTileServerConfig(baseURL))
	}

	return c.JSON(http.StatusOK, tiles.ToTileServerConfig(baseURL))
}

// GetStyle returns the MapLibre style JSON for the specified style
func (h *Handler) GetStyle(c echo.Context) error {
	id := c.Param("id")
	style, ok := styles[id]
	if !ok {
		return c.JSON(http.StatusNotFound, map[string]string{"error": "not found"})
	}
	return c.Blob(http.StatusOK, "application/json", style)
}

func (h *Handler) getBaseURL(c echo.Context) string {
	if h.host != nil {
		return h.host.String()
	}

	req := c.Request()
	scheme := "https"
	if req.TLS == nil {
		scheme = req.Header.Get("X-Forwarded-Proto")
		if scheme == "" {
			scheme = "http"
		}
	}
	return scheme + "://" + req.Host
}

func initTiles(ctx context.Context, pcms *plateaucms.CMS) (Tiles, error) {
	ml, err := pcms.AllMetadata(ctx, false)
	if err != nil {
		return nil, fmt.Errorf("failed to get all metadata: %w", err)
	}

	tiles := Tiles{}

	// Tiles from the CMS system project. Loaded first so that per-area
	// project entries below can override on name clashes.
	if sysPrj := pcms.SystemProject(); sysPrj != "" {
		sysTiles, err := getTiles(ctx, pcms.MainCMS(), sysPrj)
		if err != nil {
			return nil, fmt.Errorf("failed to get tiles from system project %s: %w", sysPrj, err)
		}
		for k, v := range sysTiles {
			tiles[k] = v
		}
	}

	for _, m := range ml {
		prj := m.DataCatalogProjectAlias
		if prj == "" {
			prj = m.ProjectAlias
		}
		if prj == "" {
			continue
		}

		cms, err := m.CMS()
		if err != nil {
			continue
		}

		tiles2, err := getTiles(ctx, cms, prj)
		if err != nil {
			return nil, fmt.Errorf("failed to get tiles from %s: %w", prj, err)
		}

		for k, v := range tiles2 {
			tiles[k] = v
		}
	}

	return tiles, nil
}
