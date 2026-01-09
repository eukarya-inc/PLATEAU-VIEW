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
| `CONFIG_URL` | Yes | - | Config JSON URL (file://, http://, https://) |
| `PORT` | No | 8080 | Server port |
| `CACHE_SIZE_MB` | No | 512 | Memory cache size in MB |
| `RELOAD_SECRET` | No | - | Secret for config reload endpoint |
| `CORS_ORIGINS` | No | * (all) | Allowed CORS origins (comma-separated, or "*" for all) |
| `PRELOAD_MODE` | No | sync | Preload mode: "sync" (blocking, default), "background" (non-blocking), or "lazy" (on first request) |
| `TILE_CACHE_URL` | No | - | Persistent cache URL (file://, gs://, s3://, r2://) |
| `TILE_CACHE_MODE` | No | read-write | Cache mode: "read-write", "read-only", "write-only", or "none" |
| `TILE_CACHE_CONTROL` | No | - | Cache-Control header for stored objects (e.g., "public, max-age=31536000") |
| `CACHE_CONTROL` | No | - | Cache-Control header value (e.g., "public, max-age=3600") |
| `R2_ACCOUNT_ID` | For R2 | - | Cloudflare R2 account ID |
| `R2_ACCESS_KEY_ID` | For R2 | - | Cloudflare R2 access key ID |
| `R2_SECRET_ACCESS_KEY` | For R2 | - | Cloudflare R2 secret access key |

## Layer Types

Available layer types in config JSON:

- `xyz`: XYZ tile proxy (supports URL template `{z}/{x}/{y}`)
- `cog`: Cloud Optimized GeoTIFF (supports HTTP/GCS/S3)
- `maplibre`: MapLibre style.json rendering (requires `maplibre` feature, Linux only)

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
