//! HTTP request handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
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
    /// `"raster"` (default, addressable under `/tiles/{name}/...`), `"dem"`
    /// (folded into the composite terrain provider; addressable under
    /// `/terrain/{name}/...`), or `"qmesh-mirror"` (the synthetic entry for
    /// the pre-rendered mirror configured via `TERRAIN_MIRROR_URL`).
    /// Surfaced here so the debug viewers can decide which dropdown to
    /// populate without re-deriving the type from heuristics.
    #[serde(rename = "type")]
    source_type: &'static str,
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
    // Track each source's kind alongside its layers. A single source only ever
    // produces one kind (raster vs dem) in `inventory_snapshot`, so the first
    // entry we see for a name is authoritative.
    let mut grouped: BTreeMap<String, (&'static str, Vec<LayerEntry>)> = BTreeMap::new();
    for e in entries {
        let (bounds, min_zoom, max_zoom, kind) = match &e.kind {
            LayerEntryKind::Raster(s) => {
                let b = s.bounds().await.map(|b| [b.west, b.south, b.east, b.north]);
                let (mn, mx) = s.zoom_range();
                (b, mn, mx, "raster")
            }
            LayerEntryKind::Dem(p) => {
                let b = p.bounds().map(|b| [b.west, b.south, b.east, b.north]);
                (b, None, Some(p.max_zoom()), "dem")
            }
        };
        let entry = grouped.entry(e.source_name).or_insert((kind, Vec::new()));
        entry.1.push(LayerEntry {
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
        .map(|(name, (source_type, mut layers))| {
            layers.sort_by_key(|l| l.layer_index);
            SourceEntry {
                name,
                source_type,
                layers,
            }
        })
        .collect();

    // Surface the pre-rendered quantized-mesh mirror (if configured) as a
    // synthetic source so debug viewers can populate their terrain picker
    // without needing to probe env vars. Distinct `type` lets the viewer
    // skip mirror-incompatible UI (e.g. the `?heights=` selector).
    //
    // When a mirror is present, also surface the default DEM source as an
    // explicit entry — even when it has no overlays in config — so the
    // viewer's "two or more entries" gate fires and the comparison picker
    // actually appears.
    if state.get_mirror().is_some() {
        if !sources
            .iter()
            .any(|s| s.name == crate::server::state::DEFAULT_DEM_SOURCE_KEY)
        {
            sources.push(SourceEntry {
                name: crate::server::state::DEFAULT_DEM_SOURCE_KEY.to_string(),
                source_type: "dem",
                layers: Vec::new(),
            });
        }
        sources.push(SourceEntry {
            name: crate::server::state::MIRROR_SOURCE_KEY.to_string(),
            source_type: "qmesh-mirror",
            layers: Vec::new(),
        });
    }

    sources.sort_by(|a, b| a.name.cmp(&b.name));

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&SourcesResponse { sources }).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `GET /tiles/catalog.json` — public-facing list of available tile sources
/// with end-user-relevant fields (name, description, per-format TileJSON
/// URLs). Unlike [`get_sources`], this intentionally omits internal details
/// like upstream layer URLs, layer indices, versions, or per-layer bounds —
/// those expose CMS-side implementation and shouldn't be in a consumer
/// catalog.
#[derive(Serialize)]
struct CatalogResponse {
    tiles: Vec<CatalogEntry>,
}

#[derive(Serialize)]
struct CatalogEntry {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// TileJSON URLs keyed by format/encoding. For `/tiles/{name}` sources
    /// this is `png` / `webp` / `avif`; for the built-in terrain endpoints
    /// it's `quantized-mesh` (Cesium) or `png` / `webp` / `avif` (MapLibre
    /// raster-dem).
    urls: std::collections::BTreeMap<String, String>,
}

/// Raster output formats advertised by the catalog. Kept in sync with the
/// formats accepted by [`get_tilejson`] and the terrain raster endpoints.
const CATALOG_RASTER_FORMATS: &[&str] = &["png", "webp", "avif"];

pub async fn get_catalog(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config = state.config_manager.get().await;

    let mut tiles: Vec<CatalogEntry> = config
        .sources
        .into_iter()
        // DEM-typed sources are folded into the composite terrain provider
        // rather than exposed under `/tiles/{name}/...`; hide them from the
        // catalog (terrain itself appears under the built-in `terrain` entry
        // and — for named DEM sources — appears below).
        .filter(|(name, src)| !src.is_dem(name))
        .map(|(name, src)| {
            let urls = CATALOG_RASTER_FORMATS
                .iter()
                .map(|fmt| {
                    (
                        (*fmt).to_string(),
                        format!("/tiles/{name}/tilejson.json?format={fmt}"),
                    )
                })
                .collect();
            CatalogEntry {
                name,
                description: src.description,
                urls,
            }
        })
        .collect();

    // Built-in terrain endpoints aren't in `config.sources` (they're driven
    // by env vars), so register them by hand. Names match the URL prefixes.
    // URLs are origin-relative for the same reason as the tilejson handlers:
    // fronting proxies (Cloud Run / Cloudflare) sometimes rewrite `Host` to
    // `localhost`, so an absolute URL built from request headers can't be
    // trusted. Clients resolve these against the catalog's own URL.
    let raster_dem_urls = |slug: &str| -> std::collections::BTreeMap<String, String> {
        CATALOG_RASTER_FORMATS
            .iter()
            .map(|fmt| {
                (
                    (*fmt).to_string(),
                    format!("/{slug}/tilejson.json?format={fmt}"),
                )
            })
            .collect()
    };
    tiles.push(CatalogEntry {
        name: "terrain".to_string(),
        description: Some(
            "Cesium quantized-mesh terrain (ellipsoidal heights, Japan coverage)".to_string(),
        ),
        urls: std::iter::once((
            "quantized-mesh".to_string(),
            "/terrain/layer.json".to_string(),
        ))
        .collect(),
    });
    tiles.push(CatalogEntry {
        name: "terrarium".to_string(),
        description: Some(
            "MapLibre raster-dem source (Terrarium encoding, ellipsoidal heights)".to_string(),
        ),
        urls: raster_dem_urls("terrarium"),
    });
    tiles.push(CatalogEntry {
        name: "mapbox".to_string(),
        description: Some(
            "MapLibre raster-dem source (Mapbox Terrain-RGB encoding, ellipsoidal heights)"
                .to_string(),
        ),
        urls: raster_dem_urls("mapbox"),
    });

    tiles.sort_by(|a, b| a.name.cmp(&b.name));

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&CatalogResponse { tiles }).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// Viewer HTML for debugging tiles. The page fetches `/tiles/sources.json`
/// on load to populate its source dropdown and inspector. Source HTML is
/// loaded from disk at request time — see [`super::static_assets`].
pub async fn viewer() -> Response {
    super::static_assets::serve_html("viewer.html")
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
    state.maybe_revalidate().await;
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

    // Take the reload mutex so this path can't overlap with a lazy
    // revalidation that's in flight (and vice versa). Manual `/reload` always
    // rebuilds sources, even when the content hash matches, so the operator
    // gets a deterministic "force reapply" semantic.
    let mutex = state.reload_mutex();
    let _guard = mutex.lock().await;
    match state.config_manager.reload().await {
        Ok(_changed) => {
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

    // Origin-relative tile URL — fronting proxies (Cloud Run / Cloudflare)
    // sometimes rewrite `Host` / `X-Forwarded-Host` to `localhost`, so an
    // absolute URL built from request headers can't be trusted. MapLibre and
    // Cesium resolve relative URLs against the tilejson location, matching
    // the behavior of the terrain `raster_tilejson` handler.
    let tile_url = format!("/tiles/{name}/{{z}}/{{x}}/{{y}}.{format}");

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
