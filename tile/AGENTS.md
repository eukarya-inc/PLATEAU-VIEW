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
│   ├── cache/           # In-memory cache
│   ├── cog/             # COG reading and rendering
│   ├── tile/            # Tile sources (XYZ, COG, Composite)
│   └── server/          # HTTP server and handlers
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

## Environment Variables

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `CONFIG_URL` | Yes | - | Config JSON URL (file://, http://, https://) |
| `PORT` | No | 8080 | Server port |
| `CACHE_SIZE_MB` | No | 512 | Memory cache size in MB |
| `RELOAD_SECRET` | No | - | Secret for config reload endpoint |
| `CORS_ORIGINS` | No | * (all) | Allowed CORS origins (comma-separated, or "*" for all) |

## Layer Types

Available layer types in config JSON:

- `xyz`: XYZ tile proxy (supports URL template `{z}/{x}/{y}`)
- `cog`: Cloud Optimized GeoTIFF (supports HTTP/GCS/S3)
- `maplibre`: MapLibre style (planned for future, currently ignored)
