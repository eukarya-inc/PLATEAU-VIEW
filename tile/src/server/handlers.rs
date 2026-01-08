//! HTTP request handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use image::ImageFormat;

use super::state::AppState;
use crate::tile::TileError;

/// Health check handler.
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Get tile handler.
pub async fn get_tile(
    State(state): State<Arc<AppState>>,
    Path((name, z, x, y_ext)): Path<(String, u32, u32, String)>,
) -> Response {
    // Parse y from "123.png" format
    let y: u32 = match y_ext.trim_end_matches(".png").parse() {
        Ok(y) => y,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid y coordinate").into_response();
        }
    };

    tracing::debug!(source = %name, z = z, x = x, y = y, "Tile request received");

    // Check cache first
    let cache_key = format!("{name}/{z}/{x}/{y}");
    if let Some(cached) = state.cache.get(&cache_key).await {
        tracing::debug!(source = %name, z = z, x = x, y = y, "Cache hit");
        return png_response(cached);
    }
    tracing::debug!(source = %name, z = z, x = x, y = y, "Cache miss");

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

    // Encode to PNG
    let png_bytes = match encode_png(&tile) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to encode PNG: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode image").into_response();
        }
    };

    // Store in cache
    state.cache.put(&cache_key, png_bytes.clone()).await;

    png_response(png_bytes)
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

fn png_response(data: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        data,
    )
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

fn encode_png(img: &image::RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    img.write_to(&mut cursor, ImageFormat::Png)?;
    Ok(bytes)
}
