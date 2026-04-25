//! HTTP request handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Json, Response},
};
use serde::Serialize;
use xxhash_rust::xxh64::xxh64;

use super::format::{TileFormat, encode_image, parse_y_and_format};
use super::response::{
    compute_etag, error_response, etag_matches, not_modified_response, tile_response,
};
use super::state::AppState;
use crate::cache::CacheObjectMeta;
use crate::tile::{TileError, TileSource};

/// Compute a hash from etag_keys for cache validation.
fn compute_etag_hash(keys: &[String]) -> String {
    let mut sorted_keys = keys.to_vec();
    sorted_keys.sort();
    let input = sorted_keys.join("|");
    format!("{:x}", xxh64(input.as_bytes(), 0))
}

/// Health check handler.
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// `GET /tiles/sources.json` — list every configured source with its layers'
/// metadata (bounds + zoom range when known). Drives both the viewer's
/// source dropdown and the inspector overlay; external tools can consume it
/// to build their own UIs.
#[derive(Serialize)]
struct SourcesResponse {
    sources: Vec<SourceEntry>,
}

#[derive(Serialize)]
struct SourceEntry {
    name: String,
    layers: Vec<LayerEntry>,
}

#[derive(Serialize)]
struct LayerEntry {
    layer_index: usize,
    layer_type: &'static str,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    /// `[west, south, east, north]` in degrees, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_zoom: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_zoom: Option<u8>,
}

pub async fn get_sources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    use crate::server::state::LayerEntryKind;
    use std::collections::BTreeMap;

    let entries = state.inventory_snapshot().await;
    let mut grouped: BTreeMap<String, Vec<LayerEntry>> = BTreeMap::new();
    for e in entries {
        let (bounds, min_zoom, max_zoom) = match &e.kind {
            LayerEntryKind::Raster(s) => {
                let b = s.bounds().await.map(|b| [b.west, b.south, b.east, b.north]);
                let (mn, mx) = s.zoom_range();
                (b, mn, mx)
            }
            LayerEntryKind::Dem(p) => {
                let b = p.bounds().map(|b| [b.west, b.south, b.east, b.north]);
                (b, None, Some(p.max_zoom()))
            }
        };
        grouped.entry(e.source_name).or_default().push(LayerEntry {
            layer_index: e.layer_idx,
            layer_type: e.layer_type,
            url: e.url,
            version: e.version,
            bounds,
            min_zoom,
            max_zoom,
        });
    }

    // Stable order: sort layers by their original index, sources alphabetically.
    let mut sources: Vec<SourceEntry> = grouped
        .into_iter()
        .map(|(name, mut layers)| {
            layers.sort_by_key(|l| l.layer_index);
            SourceEntry { name, layers }
        })
        .collect();
    sources.sort_by(|a, b| a.name.cmp(&b.name));

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&SourcesResponse { sources }).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// Viewer HTML template.
const VIEWER_HTML: &str = include_str!("viewer.html");

/// Viewer HTML for debugging tiles. The page fetches `/tiles/sources.json`
/// on load to populate its source dropdown and inspector.
pub async fn viewer() -> impl IntoResponse {
    Html(VIEWER_HTML)
}

/// Generate tile bytes from source (used in single-flight closure).
async fn generate_tile(
    source: Arc<dyn TileSource>,
    z: u32,
    x: u32,
    y: u32,
    format: TileFormat,
) -> Result<Vec<u8>, TileError> {
    // Get tile from source
    let tile = source.get_tile(z, x, y).await?.ok_or(TileError::NotFound)?;

    // Encode to requested format
    encode_image(&tile, format).map_err(TileError::from)
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

    // Get source first to ensure it exists before ETag checks
    let source = match state.get_source(&name).await {
        Some(s) => s,
        None => {
            return (StatusCode::NOT_FOUND, "Source not found").into_response();
        }
    };

    // Get ETag keys for layers that cover this tile
    let etag_keys = state
        .get_source_etag_keys(&name, z, x, y)
        .await
        .unwrap_or_default();
    let etag = compute_etag(&etag_keys, &name, format, z, x, y);
    let etag_hash = compute_etag_hash(&etag_keys);

    // Check If-None-Match header
    if etag_matches(&headers, &etag) {
        tracing::debug!(source = %name, z = z, x = x, y = y, format = fmt, "ETag match, returning 304");
        return not_modified_response(&etag, state.cache_control.as_deref());
    }

    // Cache key includes format for format-specific caching
    let cache_key = format!("{name}/{fmt}/{z}/{x}/{y}.{fmt}");

    // Metadata for persistent cache
    let meta = CacheObjectMeta {
        content_type: Some(format.content_type().to_string()),
        etag_hash: Some(etag_hash.clone()),
        etag: Some(etag.clone()),
    };

    // Get from cache or generate with single-flight deduplication
    // Multiple concurrent requests for the same tile will share the same generation
    let result = state
        .cache
        .get_or_generate(&cache_key, Some(&etag_hash), Some(meta), || {
            generate_tile(source.clone(), z, x, y, format)
        })
        .await;

    match result {
        Ok(bytes) => {
            tracing::debug!(source = %name, z = z, x = x, y = y, format = fmt, "Tile served");
            tile_response(bytes, format, Some(&etag), state.cache_control.as_deref())
        }
        Err(e) => {
            // Unwrap Arc to get the actual error
            match e.as_ref() {
                TileError::NotFound => (StatusCode::NOT_FOUND, "Tile not found").into_response(),
                _ => error_response(e.as_ref().clone()),
            }
        }
    }
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

/// TileJSON 3.0.0 response structure.
#[derive(Debug, Serialize)]
pub struct TileJson {
    tilejson: &'static str,
    tiles: Vec<String>,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribution: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheme: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minzoom: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maxzoom: Option<u32>,
}

/// Query parameters for TileJSON endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct TileJsonQuery {
    /// Output format (png, webp, avif). Defaults to png.
    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String {
    "png".to_string()
}

/// Get TileJSON metadata for a source.
pub async fn get_tilejson(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    axum::extract::Query(query): axum::extract::Query<TileJsonQuery>,
) -> Response {
    // Check if source exists
    if state.get_source(&name).await.is_none() {
        return (StatusCode::NOT_FOUND, "Source not found").into_response();
    }

    // Validate format
    let format = match query.format.as_str() {
        "png" | "webp" | "avif" => &query.format,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                "Invalid format (use png, webp, or avif)",
            )
                .into_response();
        }
    };

    // Build tile URL template using the externally-visible origin (front
    // proxies often rewrite `Host` to `localhost`, so prefer `X-Forwarded-*`).
    let (scheme, host) = crate::server::terrain::external_origin(&headers);
    let tile_url = format!("{scheme}://{host}/tiles/{name}/{{z}}/{{x}}/{{y}}.{format}");

    let tilejson = TileJson {
        tilejson: "3.0.0",
        tiles: vec![tile_url],
        name,
        attribution: Some(
            "<a href=\"https://www.mlit.go.jp/plateau/\" target=\"_blank\">PLATEAU</a>",
        ),
        scheme: Some("xyz"),
        minzoom: Some(0),
        maxzoom: Some(22),
    };

    Json(tilejson).into_response()
}
