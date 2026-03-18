# tile

High-performance tile server with Cloud Optimized GeoTIFF (COG) overlay support, written in Rust.

## Features

- **XYZ Tile Proxy**: Fetch and serve tiles from remote XYZ tile servers
- **COG Tile Generation**: Generate tiles from Cloud Optimized GeoTIFF files with HTTP range requests
- **Layer Composition**: Overlay multiple COG layers on top of base XYZ tiles
- **Multi-band NoData**: Support for multi-band nodata values with multiple patterns (e.g., black AND white as transparent)
- **Auto Overview Selection**: Automatically select the best resolution overview for each zoom level
- **Bilinear Interpolation**: Smooth tile rendering with bilinear interpolation
- **Memory Caching**: Fast in-memory tile cache using moka
- **Remote Configuration**: Load configuration from remote URL with manual reload
- **HTTP/2 (h2c)**: Support for HTTP/2 cleartext connections (auto-detects HTTP/1.1 and HTTP/2)
- **Smart ETag**: Per-tile ETag calculation based on covering layers with If-None-Match support for 304 responses
- **Configurable Cache-Control**: Set custom Cache-Control headers via environment variable
- **Multi-Format Output**: Support for PNG, WebP, and AVIF image formats

## Quick Start

### Build

```bash
cargo build --release
```

### Run

```bash
CONFIG_URL=https://example.com/config.json ./target/release/tile
```

### Docker

```bash
docker build -t tile .
docker run -e CONFIG_URL=https://example.com/config.json -p 8080:8080 tile
```

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `CONFIG_URL` | Yes | - | URL to the configuration JSON file |
| `PORT` | No | `8080` | HTTP server port |
| `CACHE_SIZE_MB` | No | `512` | Memory cache size in MB |
| `RELOAD_SECRET` | No | - | Secret token for `/reload` endpoint (if set, requires `Authorization: Bearer <token>`) |
| `CORS_ORIGINS` | No | `*` | Allowed CORS origins (comma-separated, or `*` for all) |
| `PRELOAD_MODE` | No | `sync` | COG metadata preload mode: `sync` (blocking), `background` (non-blocking), or `lazy` (on first request) |
| `TILE_CACHE_URL` | No | - | Persistent tile cache URL (see below) |
| `CACHE_CONTROL` | No | `public, max-age=3600, must-revalidate` | Cache-Control header value for tile responses |
| `RUST_LOG` | No | `info` | Log level (trace, debug, info, warn, error) |

### Persistent Cache Configuration

The tile server supports two-tier caching: fast in-memory cache (moka) + optional persistent storage.

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TILE_CACHE_URL` | No | - | Persistent cache URL (`file://`, `gs://`, `s3://`, `r2://`) |
| `TILE_CACHE_MODE` | No | `read-write` | Cache mode: `read-write`, `read-only`, `write-only`, or `none` |
| `TILE_CACHE_CONTROL` | No | - | Cache-Control header for objects stored in persistent cache (e.g., `public, max-age=31536000`) |
| `R2_ACCOUNT_ID` | For R2 | - | Cloudflare R2 account ID |
| `R2_ACCESS_KEY_ID` | For R2 | - | Cloudflare R2 access key ID |
| `R2_SECRET_ACCESS_KEY` | For R2 | - | Cloudflare R2 secret access key |

#### Cache URL Examples

```bash
# Local file cache
TILE_CACHE_URL=file:///var/cache/tiles

# Google Cloud Storage
TILE_CACHE_URL=gs://my-bucket/tiles

# Amazon S3
TILE_CACHE_URL=s3://my-bucket/tiles

# Cloudflare R2 (requires R2_* env vars)
TILE_CACHE_URL=r2://my-bucket/tiles
```

#### Cache Modes

| Mode | Read Persistent | Write Persistent | Use Case |
|------|-----------------|------------------|----------|
| `read-write` | Yes | Yes | Default. Server is primary cache source |
| `read-only` | Yes | No | Persistent storage is managed externally |
| `write-only` | No | Yes | CDN/Worker handles reads (e.g., Cloudflare Worker + R2) |
| `none` | No | No | Disable persistent cache entirely |

All modes always use the in-memory cache (moka). Persistent failures don't block tile serving (fail-safe).

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/tiles/:name/tilejson.json` | Get TileJSON metadata for a source |
| GET | `/tiles/:name/:z/:x/:y.{format}` | Get a tile (format: `png`, `webp`, `avif`) |
| GET | `/health` | Health check |
| POST | `/reload` | Reload configuration (requires `Authorization: Bearer <RELOAD_SECRET>` if secret is set) |

### Supported Image Formats

| Extension | MIME Type | Description |
|-----------|-----------|-------------|
| `.png` | `image/png` | Lossless compression, best for graphics with transparency |
| `.webp` | `image/webp` | Modern format with good compression and transparency support |
| `.avif` | `image/avif` | Best compression ratio, newer format with growing browser support |

Example requests:
```bash
# PNG (default, widest compatibility)
curl https://example.com/tiles/ortho/10/909/403.png

# WebP (smaller file size, good browser support)
curl https://example.com/tiles/ortho/10/909/403.webp

# AVIF (smallest file size, modern browsers)
curl https://example.com/tiles/ortho/10/909/403.avif
```

### TileJSON

Each source provides a TileJSON 3.0.0 endpoint for integration with mapping libraries:

```bash
# Default format (PNG)
curl https://example.com/tiles/ortho/tilejson.json

# Specify format
curl https://example.com/tiles/ortho/tilejson.json?format=webp
```

Response:
```json
{
  "tilejson": "3.0.0",
  "tiles": ["https://example.com/tiles/ortho/{z}/{x}/{y}.png"],
  "name": "ortho",
  "attribution": "<a href=\"https://www.mlit.go.jp/plateau/\" target=\"_blank\">PLATEAU</a>",
  "scheme": "xyz",
  "minzoom": 0,
  "maxzoom": 22
}
```

Query parameters:
| Parameter | Default | Description |
|-----------|---------|-------------|
| `format` | `png` | Tile format: `png`, `webp`, or `avif` |

## Configuration

Configuration is loaded from a remote JSON file specified by `CONFIG_URL`.

### Example Configuration

```json
{
  "version": "v1.0.0",
  "sources": {
    "plateau-ortho": {
      "version": "v1.0.1",
      "layers": [
        {
          "type": "xyz",
          "url": "https://example.com/tiles/{z}/{x}/{y}.png",
          "range": {
            "z_min": 0,
            "z_max": 18
          }
        },
        {
          "type": "cog",
          "url": "https://storage.googleapis.com/bucket/ortho/area1.tif",
          "nodata": [[0, 0, 0], [255, 255, 255]],
          "order": 1
        },
        {
          "type": "cog",
          "url": "gs://bucket/ortho/area2.tif",
          "nodata": [[0, 0, 0]],
          "order": 2
        }
      ]
    },
    "dem": {
      "layers": [
        {
          "type": "cog",
          "url": "https://storage.googleapis.com/bucket/dem/elevation.tif"
        }
      ]
    }
  }
}
```

### Layer Types

#### XYZ Layer

Fetches tiles from a remote XYZ tile server.

```json
{
  "type": "xyz",
  "url": "https://example.com/{z}/{x}/{y}.png",
  "range": {
    "z_min": 0,
    "z_max": 18,
    "x_min": 0,
    "x_max": 1000,
    "y_min": 0,
    "y_max": 1000
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | URL template with `{z}`, `{x}`, `{y}` placeholders |
| `range` | object | No | Coordinate range restriction |

#### COG Layer

Generates tiles from a Cloud Optimized GeoTIFF file.

```json
{
  "type": "cog",
  "url": "https://storage.googleapis.com/bucket/image.tif",
  "nodata": [[0, 0, 0], [255, 255, 255]],
  "order": 1
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | URL to COG file (HTTP, `gs://`, `s3://`) |
| `nodata` | various | No | NoData value configuration (see below) |
| `order` | number | No | Layer order (higher = on top, default: 0) |

### NoData Configuration

NoData values can be specified in multiple formats:

```json
// Single value (all bands must match)
"nodata": 255

// Single pattern (RGB black)
"nodata": [0, 0, 0]

// Multiple patterns (black OR white)
"nodata": [[0, 0, 0], [255, 255, 255]]
```

Pixels matching any nodata pattern will be rendered as transparent.

### ETag and Cache Control

The tile server supports HTTP caching through ETag headers and configurable Cache-Control.

#### Smart ETag Calculation

ETags are calculated per-tile based on which layers actually cover that specific tile. This enables granular cache invalidation:

- **Per-tile calculation**: Each tile's ETag only includes layers that cover it
- **COG bounds awareness**: Tiles outside a COG's bounds won't be invalidated when the COG changes
- **Version support**: Each layer can have an optional `version` for cache invalidation

```json
{
  "sources": {
    "ortho": {
      "layers": [
        {
          "type": "xyz",
          "url": "https://example.com/{z}/{x}/{y}.png",
          "version": "v1.0.0"
        },
        {
          "type": "cog",
          "url": "https://example.com/overlay.tif",
          "version": "v2.0.0",
          "order": 1
        }
      ]
    }
  }
}
```

| Field | Description |
|-------|-------------|
| `version` (layer) | Optional version string for cache invalidation. When changed, invalidates cache for tiles covered by this layer |

ETag calculation:
- `W/"xxhash64(source/format/z/x/y|key1|key2|...)"` where keys are from covering layers
- Each layer contributes a key like `type:url:version` (e.g., `xyz:https://example.com/{z}/{x}/{y}.png:v1.0.0`)
- Format is included in ETag, so different formats have different ETags
- Clients can send `If-None-Match` header to receive `304 Not Modified` if cache is valid
- **Granular invalidation**: Changing a COG layer's version only invalidates tiles within that COG's geographic bounds

#### Cache-Control Header

The `CACHE_CONTROL` environment variable controls HTTP caching behavior:

```bash
# Default (1 hour with must-revalidate for quick cache invalidation)
CACHE_CONTROL="public, max-age=3600, must-revalidate"

# CDN-friendly caching with longer edge cache
CACHE_CONTROL="public, max-age=3600, s-maxage=86400"

# No caching
CACHE_CONTROL="no-cache, no-store"
```

The default `must-revalidate` ensures that expired cache entries are always revalidated with the server, enabling quick propagation of cache invalidation.

### Supported COG URL Schemes

| Scheme | Example | Description |
|--------|---------|-------------|
| `http://`, `https://` | `https://example.com/file.tif` | HTTP/HTTPS with range request support |
| `gs://` | `gs://bucket/path/file.tif` | Google Cloud Storage |
| `s3://` | `s3://bucket/path/file.tif` | Amazon S3 |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Request                             │
│                GET /tiles/ortho/10/909/403.png              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Memory Cache (moka)                       │
└─────────────────────────────────────────────────────────────┘
                              │ miss
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Persistent Cache (optional)                    │
│            (file:// / gs:// / s3:// / r2://)               │
└─────────────────────────────────────────────────────────────┘
                              │ miss
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  CompositeTileSource                        │
├─────────────────────────────────────────────────────────────┤
│  1. XyzTileSource (base)                                    │
│     └─ Fetch from remote XYZ server                         │
│                                                             │
│  2. CogTileSource (overlay, order=1)                        │
│     ├─ Check intersection with COG bounds                   │
│     ├─ Select best IFD (overview) for zoom level            │
│     ├─ Fetch tiles via HTTP range requests                  │
│     ├─ Decode & apply nodata → transparent                  │
│     └─ Bilinear interpolation                               │
│                                                             │
│  3. CogTileSource (overlay, order=2)                        │
│     └─ Same as above                                        │
├─────────────────────────────────────────────────────────────┤
│                    Image Composition                        │
│              (alpha blending overlays)                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Image Response                            │
│            (PNG/WebP/AVIF, cached by format)                │
└─────────────────────────────────────────────────────────────┘
```

## Requirements

- COG files must be in **EPSG:4326 (WGS84)** projection
- COG files should have internal tiling and overviews for best performance

### Creating COG Files

Using GDAL:

```bash
# Convert GeoTIFF to COG
gdal_translate input.tif output.tif \
  -of COG \
  -co COMPRESS=DEFLATE \
  -co OVERVIEW_RESAMPLING=BILINEAR

# Reproject to EPSG:4326 if needed
gdalwarp -t_srs EPSG:4326 input.tif output_4326.tif
gdal_translate output_4326.tif output_cog.tif -of COG
```

## Development

```bash
# Run with debug logging
RUST_LOG=debug CONFIG_URL=file:///path/to/config.json cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy
```

## License

Apache-2.0
