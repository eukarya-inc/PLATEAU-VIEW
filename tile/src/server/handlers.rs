//! HTTP request handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use image::ImageFormat;
use xxhash_rust::xxh64::xxh64;

use super::state::AppState;
use crate::tile::TileError;

/// Supported tile image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileFormat {
    Png,
    WebP,
    Avif,
}

impl TileFormat {
    /// Parse format from file extension (e.g., "png", "webp", "avif").
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" => Some(Self::Png),
            "webp" => Some(Self::WebP),
            "avif" => Some(Self::Avif),
            _ => None,
        }
    }

    /// Get the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
        }
    }

    /// Get the MIME content type for this format.
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
        }
    }

    /// Convert to image crate's ImageFormat.
    pub fn image_format(&self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::WebP => ImageFormat::WebP,
            Self::Avif => ImageFormat::Avif,
        }
    }
}

/// Parse "123.png" into (123, TileFormat::Png).
fn parse_y_and_format(y_ext: &str) -> Option<(u32, TileFormat)> {
    let (y_str, ext) = y_ext.rsplit_once('.')?;
    let y: u32 = y_str.parse().ok()?;
    let format = TileFormat::from_extension(ext)?;
    Some((y, format))
}

/// Compute ETag from version (optional), format, and tile coordinates.
fn compute_etag(
    version: Option<&str>,
    source: &str,
    format: TileFormat,
    z: u32,
    x: u32,
    y: u32,
) -> String {
    let fmt = format.extension();
    let input = match version {
        Some(v) => format!("{v}/{source}/{fmt}/{z}/{x}/{y}"),
        None => format!("{source}/{fmt}/{z}/{x}/{y}"),
    };
    let hash = xxh64(input.as_bytes(), 0);
    // Use weak ETag (W/) since the representation might vary
    format!("W/\"{hash:x}\"")
}

/// Check if the request's If-None-Match header matches the ETag.
fn etag_matches(headers: &HeaderMap, etag: &str) -> bool {
    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH)
        && let Ok(value) = if_none_match.to_str()
    {
        // Handle multiple ETags (comma-separated) and "*"
        if value == "*" {
            return true;
        }
        return value.split(',').any(|v| v.trim() == etag);
    }
    false
}

/// Health check handler.
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Viewer HTML for debugging tiles.
pub async fn viewer(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sources = state.list_sources().await;
    let sources_json = serde_json::to_string(&sources).unwrap_or_else(|_| "[]".to_string());

    Html(format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Tile Viewer</title>
    <script src="https://unpkg.com/maplibre-gl@4.7.1/dist/maplibre-gl.js"></script>
    <link href="https://unpkg.com/maplibre-gl@4.7.1/dist/maplibre-gl.css" rel="stylesheet" />
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }}
        #map {{ position: absolute; top: 50px; bottom: 0; width: 100%; }}
        .controls {{
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 50px;
            background: #1a1a2e;
            display: flex;
            align-items: center;
            padding: 0 16px;
            gap: 16px;
            z-index: 1000;
        }}
        .controls label {{
            color: #fff;
            font-size: 14px;
        }}
        .controls select {{
            padding: 8px 12px;
            border-radius: 4px;
            border: none;
            background: #16213e;
            color: #fff;
            font-size: 14px;
        }}
        .controls select#source {{
            min-width: 200px;
        }}
        .controls select#format {{
            min-width: 80px;
        }}
        .controls select:focus {{
            outline: 2px solid #0f4c75;
        }}
        .controls button {{
            padding: 8px 16px;
            border-radius: 4px;
            border: none;
            background: #16213e;
            color: #fff;
            font-size: 14px;
            cursor: pointer;
            transition: background 0.2s;
        }}
        .controls button:hover {{
            background: #1f3460;
        }}
        .controls button.active {{
            background: #0f4c75;
        }}
        .info {{
            margin-left: auto;
            color: #888;
            font-size: 12px;
        }}
        .maplibregl-ctrl-attrib {{
            background: rgba(255,255,255,0.8) !important;
        }}
    </style>
</head>
<body>
    <div class="controls">
        <label for="source">Source:</label>
        <select id="source"></select>
        <label for="format">Format:</label>
        <select id="format">
            <option value="png">PNG</option>
            <option value="webp">WebP</option>
            <option value="avif">AVIF</option>
        </select>
        <button id="osmToggle" class="active">OSM</button>
        <span class="info" id="coords"></span>
    </div>
    <div id="map"></div>
    <script>
        const sources = {sources_json};
        const sourceSelect = document.getElementById('source');
        const formatSelect = document.getElementById('format');
        const osmToggle = document.getElementById('osmToggle');
        const coordsEl = document.getElementById('coords');
        let osmEnabled = true;

        // Populate source dropdown
        sources.forEach(name => {{
            const option = document.createElement('option');
            option.value = name;
            option.textContent = name;
            sourceSelect.appendChild(option);
        }});

        // Initialize map with OSM base
        const map = new maplibregl.Map({{
            container: 'map',
            style: {{
                version: 8,
                sources: {{
                    osm: {{
                        type: 'raster',
                        tiles: ['https://tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png'],
                        tileSize: 256,
                        attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
                    }}
                }},
                layers: [{{
                    id: 'osm',
                    type: 'raster',
                    source: 'osm',
                    minzoom: 0,
                    maxzoom: 19
                }}]
            }},
            center: [139.7, 35.7],
            zoom: 10,
            hash: true
        }});

        map.addControl(new maplibregl.NavigationControl());

        function toggleOsm() {{
            osmEnabled = !osmEnabled;
            osmToggle.classList.toggle('active', osmEnabled);
            if (map.getLayer('osm')) {{
                map.setLayoutProperty('osm', 'visibility', osmEnabled ? 'visible' : 'none');
            }}
        }}

        function updateSource() {{
            const sourceName = sourceSelect.value;
            const format = formatSelect.value;

            // Remove old layers and sources
            if (map.getLayer('tiles')) map.removeLayer('tiles');
            if (map.getSource('tiles')) map.removeSource('tiles');

            if (!sourceName) return;

            // Add new source
            map.addSource('tiles', {{
                type: 'raster',
                tiles: [`${{window.location.origin}}/tiles/${{sourceName}}/{{z}}/{{x}}/{{y}}.${{format}}`],
                tileSize: 256,
                attribution: `${{sourceName}} (${{format.toUpperCase()}})`
            }});

            map.addLayer({{
                id: 'tiles',
                type: 'raster',
                source: 'tiles',
                minzoom: 0,
                maxzoom: 22
            }});
        }}

        // Update coords display
        map.on('mousemove', (e) => {{
            const {{ lng, lat }} = e.lngLat;
            const zoom = map.getZoom().toFixed(1);
            coordsEl.textContent = `z${{zoom}} ${{lat.toFixed(5)}}, ${{lng.toFixed(5)}}`;
        }});

        osmToggle.addEventListener('click', toggleOsm);
        sourceSelect.addEventListener('change', updateSource);
        formatSelect.addEventListener('change', updateSource);

        // Load first source
        if (sources.length > 0) {{
            sourceSelect.value = sources[0];
            map.on('load', updateSource);
        }}
    </script>
</body>
</html>
"##
    ))
}

/// Get tile handler.
pub async fn get_tile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, z, x, y_ext)): Path<(String, u32, u32, String)>,
) -> Response {
    // Parse y and format from "123.png" format
    let (y, format) = match parse_y_and_format(&y_ext) {
        Some(parsed) => parsed,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Invalid y coordinate or unsupported format (use .png, .webp, or .avif)",
            )
                .into_response();
        }
    };

    let fmt = format.extension();
    tracing::debug!(source = %name, z = z, x = x, y = y, format = fmt, "Tile request received");

    // Get version for ETag calculation (per-source or global)
    let version = state.get_source_version(&name).await;
    let etag = compute_etag(version.as_deref(), &name, format, z, x, y);

    // Check If-None-Match header
    if etag_matches(&headers, &etag) {
        tracing::debug!(source = %name, z = z, x = x, y = y, format = fmt, "ETag match, returning 304");
        return not_modified_response(&etag, state.cache_control.as_deref());
    }

    // Check cache first (includes format in path)
    let cache_key = format!("{name}/{fmt}/{z}/{x}/{y}.{fmt}");
    if let Some(cached) = state.cache.get(&cache_key).await {
        tracing::debug!(source = %name, z = z, x = x, y = y, format = fmt, "Cache hit");
        return tile_response(cached, format, Some(&etag), state.cache_control.as_deref());
    }
    tracing::debug!(source = %name, z = z, x = x, y = y, format = fmt, "Cache miss");

    // Get source
    let source = match state.get_source(&name).await {
        Some(s) => s,
        None => {
            return (StatusCode::NOT_FOUND, "Source not found").into_response();
        }
    };

    // Generate tile
    let tile = match source.get_tile(z, x, y).await {
        Ok(Some(img)) => img,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Tile not found").into_response();
        }
        Err(e) => {
            return error_response(e);
        }
    };

    // Encode to requested format
    let encoded_bytes = match encode_image(&tile, format) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(format = fmt, "Failed to encode image: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode image").into_response();
        }
    };

    // Store in cache
    state.cache.put(&cache_key, encoded_bytes.clone()).await;

    tile_response(
        encoded_bytes,
        format,
        Some(&etag),
        state.cache_control.as_deref(),
    )
}

/// Reload configuration handler.
/// Requires `Authorization: Bearer <RELOAD_SECRET>` header if RELOAD_SECRET is set.
pub async fn reload(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Check authorization if RELOAD_SECRET is set
    if let Some(secret) = &state.reload_secret {
        let auth_header = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        let is_authorized = match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header[7..];
                token == secret
            }
            _ => false,
        };

        if !is_authorized {
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }
    }

    match state.config_manager.reload().await {
        Ok(()) => {
            state.reload_sources().await;
            (StatusCode::OK, "Configuration reloaded").into_response()
        }
        Err(e) => {
            tracing::error!("Failed to reload config: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to reload configuration",
            )
                .into_response()
        }
    }
}

/// Build a 304 Not Modified response with ETag.
fn not_modified_response(etag: &str, cache_control: Option<&str>) -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header(header::ETAG, etag);

    if let Some(cc) = cache_control {
        builder = builder.header(header::CACHE_CONTROL, cc);
    }

    builder
        .body(axum::body::Body::empty())
        .unwrap()
        .into_response()
}

/// Build a tile response with the appropriate content type and optional headers.
fn tile_response(
    data: Vec<u8>,
    format: TileFormat,
    etag: Option<&str>,
    cache_control: Option<&str>,
) -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, format.content_type());

    if let Some(etag) = etag {
        builder = builder.header(header::ETAG, etag);
    }

    if let Some(cc) = cache_control {
        builder = builder.header(header::CACHE_CONTROL, cc);
    }

    builder
        .body(axum::body::Body::from(data))
        .unwrap()
        .into_response()
}

fn error_response(e: TileError) -> Response {
    match e {
        TileError::NotFound => (StatusCode::NOT_FOUND, "Tile not found").into_response(),
        TileError::OutOfRange => (StatusCode::NOT_FOUND, "Out of range").into_response(),
        TileError::HttpError(msg) => {
            tracing::error!("HTTP error: {}", msg);
            (StatusCode::BAD_GATEWAY, "Upstream error").into_response()
        }
        TileError::CogError(msg) => {
            tracing::error!("COG error: {}", msg);
            (StatusCode::INTERNAL_SERVER_ERROR, "COG processing error").into_response()
        }
        TileError::ImageError(msg) => {
            tracing::error!("Image error: {}", msg);
            (StatusCode::INTERNAL_SERVER_ERROR, "Image processing error").into_response()
        }
        TileError::Internal(msg) => {
            tracing::error!("Internal error: {}", msg);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response()
        }
    }
}

/// Encode an image to the specified format.
fn encode_image(img: &image::RgbaImage, format: TileFormat) -> Result<Vec<u8>, image::ImageError> {
    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    img.write_to(&mut cursor, format.image_format())?;
    Ok(bytes)
}
