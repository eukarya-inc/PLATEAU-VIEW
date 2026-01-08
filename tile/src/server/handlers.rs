//! HTTP request handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Json, Response},
};
use serde::Serialize;
use xxhash_rust::xxh64::xxh64;

use super::format::{encode_image, parse_y_and_format};
use super::response::{
    compute_etag, error_response, etag_matches, not_modified_response, tile_response,
};
use super::state::AppState;
use crate::cache::CacheObjectMeta;

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

/// Viewer HTML template.
const VIEWER_HTML: &str = include_str!("viewer.html");

/// Viewer HTML for debugging tiles.
pub async fn viewer(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sources = state.list_sources().await;
    let sources_json = serde_json::to_string(&sources).unwrap_or_else(|_| "[]".to_string());

    Html(VIEWER_HTML.replace("{{SOURCES_JSON}}", &sources_json))
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

    // Check cache first (includes format in path)
    // Validate etag_hash to ensure cache is fresh after config changes
    let cache_key = format!("{name}/{fmt}/{z}/{x}/{y}.{fmt}");
    if let Some(cached) = state
        .cache
        .get_validated(&cache_key, Some(&etag_hash))
        .await
    {
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

    // Store in cache with metadata (content-type and etag_hash for validation)
    let meta = CacheObjectMeta {
        content_type: Some(format.content_type().to_string()),
        etag_hash: Some(etag_hash),
    };
    state
        .cache
        .put(&cache_key, encoded_bytes.clone(), Some(meta))
        .await;

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

    // Get host from header
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    // Build tile URL template (use https if host doesn't look like localhost)
    let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };
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
