//! HTTP response utilities for tile server.

use axum::{
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use xxhash_rust::xxh64::xxh64;

use super::format::TileFormat;
use crate::tile::TileError;

/// Compute ETag from layer keys and tile coordinates.
/// Keys are sorted to ensure consistent ordering across requests.
pub fn compute_etag(
    keys: &[String],
    source: &str,
    format: TileFormat,
    z: u32,
    x: u32,
    y: u32,
) -> String {
    let fmt = format.extension();

    // Sort keys for consistent ordering
    let mut sorted_keys = keys.to_vec();
    sorted_keys.sort();

    // Build input string: "source/format/z/x/y|key1|key2|..."
    let keys_str = sorted_keys.join("|");
    let input = format!("{source}/{fmt}/{z}/{x}/{y}|{keys_str}");

    let hash = xxh64(input.as_bytes(), 0);
    // Use weak ETag (W/) since the representation might vary
    format!("W/\"{hash:x}\"")
}

/// Check if the request's If-None-Match header matches the ETag.
pub fn etag_matches(headers: &HeaderMap, etag: &str) -> bool {
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

/// Build a 304 Not Modified response with ETag.
pub fn not_modified_response(etag: &str, cache_control: Option<&str>) -> Response {
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
pub fn tile_response(
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

/// Build an error response for tile errors.
pub fn error_response(e: TileError) -> Response {
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
