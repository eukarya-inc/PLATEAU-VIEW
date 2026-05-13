package tiles

import (
	"net/url"
	"path"
	"strings"

	"github.com/samber/lo"
)

// TileServerConfig represents the configuration for the tile server
type TileServerConfig struct {
	Sources map[string]TileServerSourceConfig `json:"sources"`
	Cache   *TileServerCacheConfig            `json:"cache,omitempty"`
}

// TileServerSourceConfig represents a named tile source configuration
type TileServerSourceConfig struct {
	Description string                  `json:"description,omitempty"`
	Layers      []TileServerLayerConfig `json:"layers"`
}

// TileServerLayerConfig represents a layer configuration (XYZ or COG)
type TileServerLayerConfig struct {
	Type   string                 `json:"type"`
	URL    string                 `json:"url"`
	Range  *TileServerRangeConfig `json:"range,omitempty"`
	NoData any                    `json:"nodata,omitempty"`
	Order  int                    `json:"order,omitempty"`
}

// TileServerRangeConfig represents zoom/coordinate range configuration
type TileServerRangeConfig struct {
	ZMin *uint `json:"z_min,omitempty"`
	ZMax *uint `json:"z_max,omitempty"`
	XMin *uint `json:"x_min,omitempty"`
	XMax *uint `json:"x_max,omitempty"`
	YMin *uint `json:"y_min,omitempty"`
	YMax *uint `json:"y_max,omitempty"`
}

// TileServerCacheConfig represents cache configuration
type TileServerCacheConfig struct {
	GCSBucket string `json:"gcs_bucket,omitempty"`
}

// ToTileServerConfig converts Tiles to TileServerConfig
func (t Tiles) ToTileServerConfig(baseURL string) TileServerConfig {
	sources := make(map[string]TileServerSourceConfig)

	for name, entry := range t {
		layers := make([]TileServerLayerConfig, 0, len(entry.URLs))
		for i, u := range entry.URLs {
			var layer TileServerLayerConfig
			if isCOGURL(u.Value) {
				layer = TileServerLayerConfig{
					Type:  "cog",
					URL:   u.Value,
					Order: i,
				}
			} else {
				layer = TileServerLayerConfig{
					Type:  "xyz",
					URL:   buildTileURLTemplate(u.Value),
					Range: rangeToTileServerRange(u.Key),
				}
			}
			layers = append(layers, layer)
		}
		sources[name] = TileServerSourceConfig{
			Description: entry.Description,
			Layers:      layers,
		}
	}

	// Add MapLibre style sources
	for _, style := range []string{"dark-map", "light-map"} {
		styleURL := baseURL + "/tiles/styles/" + style
		sources[style] = TileServerSourceConfig{
			Layers: []TileServerLayerConfig{
				{
					Type: "maplibre",
					URL:  styleURL,
				},
			},
		}
	}

	return TileServerConfig{Sources: sources}
}

// isCOGURL checks if the URL points to a COG file
func isCOGURL(u string) bool {
	ext := strings.ToLower(path.Ext(u))
	return ext == ".tif" || ext == ".tiff"
}

// buildTileURLTemplate converts a base URL to a tile URL template with {z}/{x}/{y} placeholders
func buildTileURLTemplate(baseURL string) string {
	u, err := url.Parse(baseURL)
	if err != nil {
		return baseURL + "/{z}/{x}/{y}.png"
	}

	// Use path.Join for the path, then manually construct the URL to avoid escaping
	u.Path = path.Join(u.Path, "{z}", "{x}", "{y}.png")

	// Manually build the URL string to preserve curly braces
	result := u.Scheme + "://" + u.Host + u.Path
	if u.RawQuery != "" {
		result += "?" + u.RawQuery
	}
	return result
}

// rangeToTileServerRange converts Range to TileServerRangeConfig
func rangeToTileServerRange(r Range) *TileServerRangeConfig {
	// If all values are -1 (no limit), return nil
	if r.ZMin < 0 && r.ZMax < 0 && r.XMin < 0 && r.XMax < 0 && r.YMin < 0 && r.YMax < 0 {
		return nil
	}

	result := &TileServerRangeConfig{}

	if r.ZMin >= 0 {
		result.ZMin = lo.ToPtr(uint(r.ZMin))
	}
	if r.ZMax >= 0 {
		result.ZMax = lo.ToPtr(uint(r.ZMax))
	}
	if r.XMin >= 0 {
		result.XMin = lo.ToPtr(uint(r.XMin))
	}
	if r.XMax >= 0 {
		result.XMax = lo.ToPtr(uint(r.XMax))
	}
	if r.YMin >= 0 {
		result.YMin = lo.ToPtr(uint(r.YMin))
	}
	if r.YMax >= 0 {
		result.YMax = lo.ToPtr(uint(r.YMax))
	}

	return result
}
