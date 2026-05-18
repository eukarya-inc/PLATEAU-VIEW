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
- **Terrain**: Cesium quantized-mesh-1.0 (TMS Geodetic) plus Mapzen Terrarium and Mapbox Terrain-RGB raster tiles (Web Mercator XYZ), generated from a Mapterhorn DEM and a selectable `japan-geoid` model (GSIGEO2011 / JPGEO2024 / JPGEO2024+Hrefconv). Heights are **ellipsoidal** (orthometric + geoid). Tiles fully outside the geoid coverage respond 404.

## Terrain

The server generates Cesium quantized-mesh-1.0 (`/terrain/`, **TMS Geodetic** addressing as Cesium expects) plus Mapzen Terrarium (`/terrarium/`) and Mapbox Terrain-RGB v1 (`/mapbox/`) raster tiles on the fly from a DEM source plus a geoid model. The two raster endpoints serve **Web Mercator XYZ** tiles — same projection as a normal MapLibre/Mapbox raster source. The output is in **ellipsoidal heights**, ready to drop into Cesium (or any MapLibre style that consumes Mapbox/Terrarium DEM) without a vertical-datum mismatch against 3D Tiles or geocoded data.

### Why Mapterhorn for the DEM

[Mapterhorn](https://www.mapterhorn.com) is our default DEM upstream because of how it's packaged and licensed:

- **Single source, global coverage with high-resolution Japan.** Mapterhorn merges many open national DEMs into one consistent set. For Japan it builds on **国土地理院 (GSI)** elevation data, so we get the local resolution we need without stitching multiple providers ourselves.
- **Distributed as PMTiles.** Every region is a single `.pmtiles` archive, and `pmtiles extract --bbox=...` downloads only the bytes inside a bounding box. That makes a Japan-only, production-ready mirror a single command — see [`scripts/japan-pmtiles/`](scripts/japan-pmtiles/).
- **Friendly licensing.** Code is BSD-3, terrain data is CC-BY-4.0 / OGL / CC0 family. The full attribution list is at [mapterhorn.com/attribution](https://www.mapterhorn.com/attribution); we credit Mapterhorn and 国土地理院 in the layer.json automatically.
- **Modern format.** 512 px Terrarium-encoded WebP — smaller transfers and a strict drop-in replacement for the deprecated AWS Elevation Tiles.

In production the recommended setup is to mirror Mapterhorn's Japan slice into your own R2 / GCS bucket and point the tile server at it; this avoids hitting `tiles.mapterhorn.com` for every request and keeps you in control of cache invalidation.

### Why a separate geoid model

Mapterhorn (and most public DEM tile services) encode **orthometric** heights — height above the geoid, i.e. roughly mean sea level. Cesium's globe is the WGS84 ellipsoid, so feeding orthometric heights directly causes a 30–40 m vertical offset over Japan, which makes 3D Tiles buildings float or sink. We resolve this by adding a **geoid height** at every grid point, using the [`japan-geoid`](https://crates.io/crates/japan-geoid) crate (GSIGEO2011 / JPGEO2024 / JPGEO2024+Hrefconv). The model is selectable per-request via `?geoid=...`, with each model living in a separate cache key so switching is instant and partial-purge-free.

Tiles whose bounds lie **entirely outside** the selected geoid's coverage respond `404`. Tiles partially outside the coverage are rendered, with the out-of-coverage pixels treated as geoid offset = 0.

### Layering DEM overlays on top of the base

The base DEM is set via `DEM_URL` (env var). To **patch in higher-resolution data over a specific area** — for example a city-level COG, a regional Terrarium PMTiles, or an XYZ DEM service — declare a special source named **`dem`** in the config JSON:

```jsonc
{
  "sources": {
    "ortho": { "layers": [/* regular raster layers */] },

    "dem": {
      "layers": [
        // Order matters: index 0 = bottom-most overlay, last = frontmost.
        { "type": "pmtiles", "url": "gs://my-bucket/japan-2m.pmtiles",
          "encoding": "terrarium", "version": "v1",
          "maxZoom": 14, "nativeTileSize": 512 },

        { "type": "cog", "url": "https://.../tokyo-1m.tif",
          "version": "tokyo-2025q1", "nodata": -9999 },

        { "type": "xyz", "url": "https://.../detailed/{z}/{x}/{y}.png",
          "encoding": "mapbox", "maxZoom": 18, "nativeTileSize": 256 }
      ]
    }
  }
}
```

- The source named `"dem"` is **not** exposed under `/tiles/dem/...`; its layers feed the terrain endpoint instead.
- Each overlay paints over the layers below it pixel-by-pixel. Where an overlay has no data (NaN / nodata), the layer underneath shows through.
- At startup, every COG / PMTiles overlay's metadata is fetched in parallel and indexed into an R*-tree. Per-tile rendering only fetches overlays whose bbox intersects the tile, so the cost stays flat as you add more local overlays.
- Cache keys aggregate base + every overlay's ETag (or `failed:slug` markers when an overlay's fetch fails for that tile), so updating any archive in place rolls all serving caches without a CDN partial purge.
- Each pod refreshes every COG overlay's upstream ETag every **5 minutes** (single HEAD per overlay), so a CMS-side file swap that doesn't bump the config hash is picked up automatically — no `/reload` needed. The pod's own memory and persistent caches invalidate on ETag mismatch; downstream HTTP caches still honour their `Cache-Control: max-age` so end-users see the new tiles after at most one CDN TTL.

### Preparing COG DEM overlays

When you build a COG to use as a `dem` overlay, **how you generate the overviews matters a lot at low zooms**. The default `gdal_translate -of COG` pipeline uses `average` resampling for overviews — which, even when nodata-aware, has a footprint-growing behavior: any 2×2 group with *at least one* valid pixel keeps a valid value in the parent. After five levels (×32 downsample) a single 5 m land pixel in the middle of the sea has spread to a ~160 m square block of "land" surrounding it.

When `CompositeDemProvider` then paints that bloated land block over the base DEM (which correctly reads ~0 m for the surrounding sea), you get **tall thin "spikes" around every small island, harbour, and coastline** at low zooms. The footprint mismatch is the problem, not the elevation values themselves.

The fix is to **build overviews with `gdaladdo -r nearest`** (no footprint growth — small features either survive at their exact position or get dropped, both of which are visually fine) and then **tell the COG driver to use those existing overviews** instead of regenerating with `average`. If you're starting from multiple source tiles, also **mosaic them with `gdalwarp -r near`** — anything else (default `bilinear`, `cubic`, …) blends real elevations with the nodata sentinel at every mask boundary and seeds the same spike pattern at the *source* level, which then leaks into every downstream overview:

```bash
# 0. (Only if you have multiple source tiles.) Mosaic to a single GeoTIFF
#    with nearest-neighbour resampling so nodata never blends with real
#    elevations. Set -srcnodata and -dstnodata explicitly to the sentinel.
gdalwarp -r near -multi \
  -co COMPRESS=ZSTD -co PREDICTOR=3 -co TILED=YES -co BIGTIFF=IF_SAFER \
  -srcnodata <SENTINEL> -dstnodata <SENTINEL> \
  -t_srs EPSG:4326 \
  source-tiles/*.tif input.tif

# 1. Translate to a plain tiled GeoTIFF (no overviews yet). If you ran
#    step 0 with the COG-compatible compression options above, you can
#    skip this step and let gdaladdo write overviews into `input.tif`
#    directly.
gdal_translate -of GTiff \
  -co COMPRESS=ZSTD -co PREDICTOR=3 -co TILED=YES -co BIGTIFF=IF_SAFER \
  input.tif tmp.tif

# 2. Build footprint-preserving overviews with nearest-neighbour.
gdaladdo -r nearest tmp.tif 2 4 8 16 32

# 3. Wrap as COG, reusing the overviews we just built.
gdal_translate -of COG \
  -co COMPRESS=ZSTD -co PREDICTOR=3 \
  -co OVERVIEWS=FORCE_USE_EXISTING -co BIGTIFF=IF_SAFER \
  tmp.tif output.tif

rm tmp.tif
```

Make sure the band's `nodata` value is set on the input before step 2 (`gdal_edit.py -a_nodata <value> input.tif` if it isn't). `gdaladdo` reads it from the band metadata, and `nearest` won't synthesise spurious values that fall outside your real elevation range.

#### Picking a nodata sentinel

Any sentinel works for the tile server, but **small magnitudes are easier on every other tool** in your pipeline. Recommended (in order of preference):

1. **`-9999`** — the de-facto DEM convention. Tiny tolerance gap, never collides with real Japanese elevations (lowest land < −10 m, highest peak < 4000 m), survives 16-bit signed integer formats if you ever downcast.
2. **`255` / `0`** — fine for tightly-bounded datasets (e.g. an urban DSM that can't physically be near those values).
3. **`f32::MIN` (`-3.4028235e+38`)** — avoid if you can. `gdalwarp` accepts it but the value is so extreme that *any* non-nearest resampler creates fringe values across many orders of magnitude (we've seen `-2.7e+37` in production), which then have to be caught by the server's defensive guards rather than the strict nodata check. Stick to `-9999` and the data stays clean for QGIS, downstream processors, and statistics tools too.

> 🛡️ **Defense-in-depth in the server.** Even if a corrupted COG slips through, the tile server has three independent guards so a single bad pixel can no longer black out an entire terrain tile (the failure mode we observed in production with a `f32::MIN`-sentinel overlay):
>
> 1. **Adaptive nodata tolerance** (`src/cog/reader.rs`) — `max(0.5 m, |nodata| · 1e-3)`. Small sentinels get the 0.5 m floor that catches `254.99996` next to `255`; huge sentinels get a proportional band wide enough to absorb bilinear-blended fringe values.
> 2. **Physical elevation guard** (`src/cog/decode.rs`, `MAX_PHYSICAL_ELEVATION_M = 50_000`) — anything beyond ±50 km is dropped to NaN at decode time. Mt. Everest is 8.85 km, Mariana Trench −10.9 km; anything bigger is corruption.
> 3. **Mesh-generator sanitisation** (`src/terrain/mesh_gen.rs`) — `find_height_range` and the Martini sample callback both reject non-finite / out-of-range heights so a stray bad value can't drag `min_height` to −10³⁷ and collapse the quantized-mesh bounding sphere or horizon-occlusion point.
>
> Mosaicking with `-r near` is still the right thing to do — keeping the data clean upstream means *other* tools (QGIS, downstream processors) also see well-formed values.

> ⚠️ The COG driver-only option for adopting pre-built overviews is **`OVERVIEWS=FORCE_USE_EXISTING`**. The GTiff-driver equivalent `COPY_SRC_OVERVIEWS=YES` is silently ignored by `-of COG` (GDAL only emits a `Warning 6`), and the result is a COG whose overviews were regenerated with `average` — exactly the spike-producing case.

> ℹ️ **Don't reach for `-r average` or `-r mode` to "smooth" things.** Both also keep a pixel valid whenever the 2×2 group contains *any* valid pixel, so they grow the land footprint identically (measured +6 pp valid-pixel ratio at OV4 vs. +0.2 pp for `nearest`). `mode` preserves peak elevations slightly better, but produces the same spike pattern. They are only safe when your dataset has no nodata mask at all.

To re-check after build:

```bash
# Should print OVERVIEW_RESAMPLING=NEAREST and the per-overview NoData/Min/Max.
gdalinfo -mm output.tif | grep -E "(OVERVIEW_RESAMPLING|NoData|Overviews|Min/Max)"

# Sanity-check that the valid-pixel ratio is *stable* across overview levels
# (a level that "grows" the land area is the symptom of averaging spillover):
python3 -c "
from osgeo import gdal
ds = gdal.Open('output.tif'); b = ds.GetRasterBand(1); nd = b.GetNoDataValue()
for i in range(b.GetOverviewCount()):
    a = b.GetOverview(i).ReadAsArray()
    print(f'OV{i}: {100*(a!=nd).sum()/a.size:.1f}% valid')
"
```

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
| `CONFIG_URL` | No | - | URL to the configuration JSON file. Omit to run with terrain-only defaults; required only to enable `/tiles/...` sources |
| `CONFIG_TTL_SECS` | No | `60` | Lazy revalidation TTL. Each tile/terrain request checks whether the config has been re-fetched within this window; on the first miss per pod the request synchronously re-fetches and, if the body hash changed, rebuilds sources before serving. `0` disables (manual `/reload` only). Synchronous on purpose — Cloud Run throttles CPU outside the active request, so a background poller could be paused or killed mid-rebuild |
| `PORT` | No | `8080` | HTTP server port |
| `CACHE_SIZE_MB` | No | `512` | Memory cache size in MB |
| `RELOAD_SECRET` | No | - | Secret token for `/reload` endpoint (if set, requires `Authorization: Bearer <token>`) |
| `CORS_ORIGINS` | No | `*` | Allowed CORS origins (comma-separated, or `*` for all) |
| `PRELOAD_MODE` | No | `sync` | COG metadata preload mode: `sync` (blocking), `background` (non-blocking), or `lazy` (on first request) |
| `TILE_CACHE_URL` | No | - | Persistent tile cache URL (see below) |
| `CACHE_CONTROL` | No | `public, max-age=3600, must-revalidate` | Cache-Control header value for tile responses |
| `NO_CACHE` | No | - | If truthy (`1`/`true`/`yes`/`on`), disables all caching: memory cache → 0, persistent cache → off, `Cache-Control` → `no-store, must-revalidate`. Handy during local terrain iteration |
| `RUST_LOG` | No | `info` | Log level (trace, debug, info, warn, error) |

### Terrain Variables

The terrain endpoint's base DEM and output settings are operational concerns and live in env vars (the config JSON only describes `/tiles/...` overlay sources).

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DEM_URL` | No | Mapterhorn public endpoint | DEM source URL. If it ends with `.pmtiles`, the server reads it as a PMTiles archive. Schemes: `https://` (any HTTPS host), `gs://bucket/key` (Google Cloud Storage, supports private via ADC / `GOOGLE_APPLICATION_CREDENTIALS`), `s3://bucket/key` (AWS S3 / S3-compatible), `r2://bucket/key` (Cloudflare R2 — set `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`), `file:///path/to.pmtiles`. Anything else is treated as a Mapterhorn-style `{z}/{x}/{y}` template |
| `DEM_VERSION` | No | `v1` | Internal version key, mixed into cache keys. Bump for an explicit cache break |
| `DEM_MAX_ZOOM` | No | `15` | Upstream DEM max zoom (clamps `/terrain/` requests above this) |
| `DEM_NATIVE_TILE_SIZE` | No | `512` | Native tile pixel size in the upstream archive (PMTiles only; Mapterhorn is always 512) |
| `TERRAIN_TILE_SIZE` | No | `256` | Output raster tile pixel size for `/terrarium/` and `/mapbox/` |
| `TERRAIN_DEFAULT_GEOID` | No | `gsigeo2011` | Default geoid model when `?geoid=` is not specified. One of `gsigeo2011`, `jpgeo2024`, `jpgeo2024-hrefconv`, `none` |
| `TERRAIN_MAX_ZOOM` | No | `18` | Max zoom advertised in `/terrain/layer.json` and the raster `tilejson.json` endpoints. Above `DEM_MAX_ZOOM` both the quantized-mesh and raster endpoints fall back to the parent DEM tile and bilinear-upsample the relevant sub-region. |
| `TERRAIN_MAX_ERROR` | No | `5.0` | Martini mesh-simplification error in meters (lower = more triangles) |

```bash
# Self-hosted PMTiles on a public R2 bucket (HTTPS)
DEM_URL=https://pub-xxxx.r2.dev/japan-dem-v1.pmtiles cargo run

# Private GCS bucket (uses application-default credentials)
DEM_URL=gs://my-private-bucket/japan-dem-v1.pmtiles cargo run

# Private R2 bucket (S3-compatible)
R2_ACCOUNT_ID=xxxx R2_ACCESS_KEY_ID=xxxx R2_SECRET_ACCESS_KEY=xxxx \
  DEM_URL=r2://my-bucket/japan-dem-v1.pmtiles cargo run

# Local file
DEM_URL=file:///abs/path/to/japan-dem-v1.pmtiles cargo run

# Default Mapterhorn upstream — fine for development
cargo run
```

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
| GET | `/terrain/layer.json` | Cesium quantized-mesh-1.0 layer.json. Query: `?geoid=gsigeo2011\|jpgeo2024\|jpgeo2024-hrefconv\|none` |
| GET | `/terrain/:z/:x/:y.terrain` | Quantized-mesh-1.0 tile (gzipped, octvertexnormals). Same `?geoid=` query |
| GET | `/terrarium/tilejson.json` | TileJSON for the Terrarium raster output. Query: `?geoid=...&format=png\|webp\|avif` |
| GET | `/terrarium/:z/:x/:y.{format}` | Terrarium raster of **ellipsoidal** heights (orthometric DEM + geoid offset) |
| GET | `/mapbox/tilejson.json` | TileJSON for the Mapbox Terrain-RGB v1 raster output |
| GET | `/mapbox/:z/:x/:y.{format}` | Mapbox Terrain-RGB v1 raster of ellipsoidal heights |
| GET | `/terrain-viewer` | Embedded Cesium preview of the terrain output |
| GET | `/health` | Health check |
| POST | `/reload` | Force-reload configuration (requires `Authorization: Bearer <RELOAD_SECRET>` if secret is set). Always rebuilds sources, even when the config body hash is unchanged. Most operators don't need to call this — see `CONFIG_TTL_SECS` for the lazy-revalidation path that picks up CMS changes automatically |

> See [Terrain](#terrain) above for what these endpoints output and how the geoid query parameter works. To self-host the DEM, see [`scripts/japan-pmtiles/`](scripts/japan-pmtiles/).

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

#### PMTiles Layer

Reads image tiles from a PMTiles archive. Same URL schemes as DEM PMTiles
(`https://`, `gs://`, `s3://`, `r2://`, `file://`).

```json
{
  "type": "pmtiles",
  "url": "https://pub-xxx.r2.dev/imagery.pmtiles",
  "range": { "z_min": 0, "z_max": 18 }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | URL to the `.pmtiles` archive |
| `range` | object | No | Coordinate range restriction |

When this layer type is used inside the special `sources.dem` source, the
extra DEM-only fields `encoding` (`terrarium` \| `mapbox`), `maxZoom`, and
`nativeTileSize` apply.

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
