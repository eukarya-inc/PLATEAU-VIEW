# PLATEAU Tile Server

A high-performance tile server written in Rust. Supports XYZ tile proxying and Cloud Optimized GeoTIFF (COG) rendering.

## Tech Stack

- **Language**: Rust (Edition 2024)
- **Web Framework**: Axum
- **Async Runtime**: Tokio
- **Image Processing**: image crate
- **COG Reading**: tiff crate + object_store (HTTP/GCS/S3)

## Directory Structure

```
tile/
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Library root
│   ├── config.rs        # Configuration management (remote loading)
│   ├── cache/           # Two-tier cache (memory + persistent storage)
│   ├── cog/             # COG reading and rendering
│   ├── tile/            # Tile sources (XYZ, COG, Composite)
│   └── server/          # HTTP server and handlers
├── static/              # Viewer HTML (served at runtime, not embedded)
├── worker/              # Cloudflare Worker (TypeScript) - R2 cache frontend
├── tests/               # E2E tests
└── fixtures/            # Test COG files
```

## Development Commands

```bash
# Build
cargo build

# Test
cargo test

# Format
cargo fmt

# Lint
cargo clippy --all-targets -- -D warnings

# Run dev server
CONFIG_URL=file://path/to/config.json cargo run
```

## Important Notes

- **Always run `cargo fmt` and `cargo clippy` before committing**
- CI treats warnings as errors with `cargo clippy -- -D warnings`
- Test COG files (fixtures/*.tif) must be in proper tiled format (strip format is not supported)
- **When adding features or making changes, don't forget to update README.md**

## Environment Variables

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `CONFIG_URL` | No | - | Config JSON URL (file://, http://, https://). Omit to run with built-in terrain only |
| `PORT` | No | 8080 | Server port |
| `CACHE_SIZE_MB` | No | 512 | Memory cache size in MB |
| `RELOAD_SECRET` | No | - | Secret for config reload endpoint |
| `CORS_ORIGINS` | No | * (all) | Allowed CORS origins (comma-separated, or "*" for all) |
| `PRELOAD_MODE` | No | sync | Preload mode: "sync" (blocking, default), "background" (non-blocking), or "lazy" (on first request) |
| `TILE_CACHE_URL` | No | - | Persistent cache URL (file://, gs://, s3://, r2://) |
| `TILE_CACHE_MODE` | No | read-write | Cache mode: "read-write", "read-only", "write-only", or "none" |
| `TILE_CACHE_CONTROL` | No | - | Cache-Control header for stored objects (e.g., "public, max-age=31536000") |
| `CACHE_CONTROL` | No | public, max-age=3600, must-revalidate | Cache-Control header for HTTP responses |
| `NO_CACHE` | No | - | Truthy disables memory+persistent cache and forces `Cache-Control: no-store` (local dev) |
| `R2_ACCOUNT_ID` | For R2 | - | Cloudflare R2 account ID |
| `R2_ACCESS_KEY_ID` | For R2 | - | Cloudflare R2 access key ID |
| `R2_SECRET_ACCESS_KEY` | For R2 | - | Cloudflare R2 secret access key |

## Layer Types

Available layer types in config JSON:

- `xyz`: XYZ tile proxy (supports URL template `{z}/{x}/{y}`)
- `cog`: Cloud Optimized GeoTIFF (supports HTTP/GCS/S3)
- `pmtiles`: raster PMTiles archive (supports https/gs/s3/r2/file)
- `maplibre`: MapLibre style.json rendering (requires `maplibre` feature, Linux only)

## Terrain

`/terrain/` (quantized-mesh-1.0, **TMS Geodetic** for Cesium), plus `/terrarium/` (Mapzen Terrarium) and `/mapbox/` (Mapbox Terrain-RGB) endpoints are built-in. Each raster endpoint also exposes a `/tilejson.json` for use as a MapLibre `raster-dem` source. They serve **Web Mercator XYZ** tiles per their respective specs — same projection as a normal MapLibre/Mapbox raster source — at `TERRAIN_TILE_SIZE` (default 256) per side. DEM defaults to Mapterhorn (512 px Terrarium WebP) and is composed with a `japan-geoid` model (GSIGEO2011 / JPGEO2024 / JPGEO2024+Hrefconv) to produce **ellipsoidal heights**. For zooms above the upstream `DEM_MAX_ZOOM`, both the raster endpoints and the Cesium quantized-mesh endpoint fall back to the parent DEM tile and bilinear-upsample the relevant sub-region (per stralift's behavior), so MapLibre's terrain mesh and Cesium's terrain LOD both stay dense at high camera zooms. Tiles outside the configured geoid coverage respond 404. Default geoid is `gsigeo2011`, overridable per-request via `?geoid=...`.

Configured via env vars (the config JSON only describes `/tiles/...` overlay sources):
- `DEM_URL` — base DEM URL. `*.pmtiles` selects the PMTiles backend; supports `https://`, `gs://`, `s3://`, `r2://`, `file://`. Anything else is read as a Mapterhorn-style `{z}/{x}/{y}` template.
- `DEM_VERSION`, `DEM_MAX_ZOOM`, `DEM_NATIVE_TILE_SIZE`
- `TERRAIN_TILE_SIZE` (default 256), `TERRAIN_DEFAULT_GEOID`, `TERRAIN_MAX_ZOOM` (default 18; raster endpoints upsample beyond `DEM_MAX_ZOOM`), `TERRAIN_MAX_ERROR`

Cache keys include the DEM version, upstream ETag digest, geoid slug, and (for Terrarium) output size. Switching geoid at request time does **not** require a CDN purge — each model lives at a separate key.

Any config-JSON source with `"type": "dem"` (or, for back-compat, the reserved name `"dem"` with no `type`) is **special-cased**: its `layers` (cog/xyz/pmtiles, in array order — index 0 = bottom-most overlay, last = frontmost) are stacked over the base DEM via `CompositeDemProvider`. Each overlay's bbox is fetched on startup and indexed in an R*-tree, so per-tile fetches only hit overlays whose footprint actually intersects the tile. Overlay fetch failures fall back to the layer below and contribute a `failed:slug` marker to the etag so caches roll back automatically when the upstream recovers.

Multiple DEM sources can coexist — each addressable under `/terrain/{name}/...`, `/terrarium/{name}/...`, and `/mapbox/{name}/...`. The unnamed endpoints (`/terrain/...` etc.) resolve to the source named `"dem"`, so the previous single-DEM setup keeps working unchanged. The base DEM and geoid are shared across every named source; only the overlay stack differs, which is useful for e.g. wiring up a `dem-staging` source whose COG overlays can be validated alongside the production `dem`.

### Quantized-mesh mirror (`TERRAIN_MIRROR_URL`)

`/terrain/` has a second backend: a **pass-through mirror** that serves pre-rendered quantized-mesh tiles read straight from an object-store bucket (e.g. the R2 mirror built by `eukarya-inc/ion-terrain-mirror`). No DEM, no Martini, no geoid composition at request time — just an `R2 GET` per tile plus the right headers.

Routing once `TERRAIN_MIRROR_URL` is set:

| URL                                        | Backend |
|--------------------------------------------|---------|
| `/terrain/layer.json`                      | Mirror  |
| `/terrain/{z}/{x}/{y}.terrain`             | Mirror  |
| `/terrain/mirror/{z}/{x}/{y}.terrain`      | Mirror  |
| `/terrain-mirror/{z}/{x}/{y}.terrain`      | Mirror (separate route — always the mirror; 404 when `TERRAIN_MIRROR_URL` is unset) |
| `/terrain/dem/{z}/{x}/{y}.terrain`         | DEM (Mapterhorn etc.) — used for side-by-side validation while DEM coverage is finalized |
| `/terrain/{other}/...`                     | DEM, looked up under the source name in the config JSON |

Stored objects must match the `ion-terrain-mirror` layout (`{prefix}/layer.json`, `{prefix}/{z}/{x}/{y}.terrain`, gzipped bytes). The handler stamps `Content-Encoding: gzip` and the standard quantized-mesh `Content-Type` on every tile response and rewrites `attribution` + `tiles` on the upstream `layer.json` so the credit line and tile template match the rest of the server. `/terrarium/...` and `/mapbox/...` raster endpoints are unaffected by the mirror — they still go through the DEM pipeline.

Terrain source code lives in `src/terrain/` (DEM providers + geodetic resampling, originally ported from <https://github.com/MIERUNE/stralift>) and `src/server/terrain.rs` (handlers). Mesh encoding, the Martini RTIN implementation, vertex-normal computation, Terrarium/Mapbox heightmap codecs, and `layer.json` types are provided by the [`terrain-codec`](https://crates.io/crates/terrain-codec) crate (which re-exports the [`martini`](https://crates.io/crates/martini) and [`quantized-mesh`](https://crates.io/crates/quantized-mesh) crates). Build & ship a Japan-only PMTiles mirror with `scripts/japan-pmtiles/`.

## Features

| Feature | Description |
|---------|-------------|
| `maplibre` | Enable MapLibre style.json rendering using maplibre-native-rs. Only works on Linux. |

### Building with MapLibre support

```bash
# Docker build (recommended - includes all dependencies)
docker build -t tile-server .

# Linux native build with maplibre
cargo build --release --features maplibre
```

**Note:** The `maplibre` feature requires:
- Linux environment (macOS/Windows not supported)
- Xvfb for headless rendering
- Mesa OpenGL libraries for software rendering

## Docker

The Dockerfile is configured for headless MapLibre rendering without GPU:

```bash
# Build
docker build -t tile-server .

# Run
docker run -p 8080:8080 \
  -e CONFIG_URL=https://example.com/config.json \
  tile-server
```

### Environment variables for headless rendering

These are pre-configured in the Dockerfile:

| Variable | Value | Description |
|----------|-------|-------------|
| `DISPLAY` | `:99` | Virtual display for Xvfb |
| `LIBGL_ALWAYS_SOFTWARE` | `1` | Force software rendering (no GPU) |
| `GALLIUM_DRIVER` | `llvmpipe` | Use LLVMpipe software renderer |
