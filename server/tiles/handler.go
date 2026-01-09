package tiles

import (
	"context"
	_ "embed"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"sync"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
)

const modelKey = "tiles"

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
	pcms    *plateaucms.CMS
	lock    sync.RWMutex
	host    *url.URL
	tileURL *url.URL
	tiles   Tiles
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
	}, nil
}

func (h *Handler) Init(ctx context.Context) {
	h.lock.Lock()
	defer h.lock.Unlock()

	tiles, err := initTiles(ctx, h.pcms)
	if err != nil {
		log.Errorfc(ctx, "tiles: failed to init tiles: %v", err)
		return
	}

	h.tiles = tiles
	if len(h.tiles) == 0 {
		log.Debugfc(ctx, "tiles: no tiles found")
		return
	}

	log.Debugfc(ctx, "tiles: initialized: \n%s", h.tiles)
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
	h.lock.RLock()
	defer h.lock.RUnlock()

	baseURL := h.getBaseURL(c)

	if h.tiles == nil {
		return c.JSON(http.StatusOK, Tiles{}.ToTileServerConfig(baseURL))
	}

	config := h.tiles.ToTileServerConfig(baseURL)
	return c.JSON(http.StatusOK, config)
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
