//! Terrain (quantized-mesh-1.0) and Terrarium raster HTTP handlers.
//!
//! - `GET /terrain/layer.json[?geoid=...]`
//! - `GET /terrain/{z}/{x}/{y}.terrain[?geoid=...]` — gzipped quantized-mesh-1.0
//! - `GET /terrarium/{z}/{x}/{y}.{png|webp|avif}[?geoid=...]` — ellipsoid-height terrarium
//! - `GET /terrarium/tilejson.json[?geoid=...&format=...]`
//!
//! Heights are served as ellipsoidal (orthometric Mapterhorn + japan-geoid offset).
//! Tiles entirely outside the geoid's coverage area respond 404.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh64::xxh64;

use super::format::{TileFormat, encode_image, parse_y_and_format};
use super::response::{compute_etag, etag_matches, not_modified_response, tile_response};
use super::state::AppState;
use crate::cache::CacheObjectMeta;
use crate::terrain::{
    DemProvider, Geoid, GeoidModel,
    ellipsoid::apply_geoid_to_grid,
    geodetic::{
        CESIUM_TILE_SIZE, GeodeticBounds, fetch_geodetic_tile_elevations, geodetic_tms_bounds,
    },
    layer_json::TileAvailability,
    mesh_gen::{QuantizedMeshOptions, generate_quantized_mesh_tile},
    quantized_mesh::TileBounds as QmBounds,
    terrarium::encode_terrarium,
};

/// Approximate bounds of the Japan geoid coverage, used for `layer.json`
/// availability. GSIGEO2011 and JPGEO2024 both cover Japan including
/// outlying islands; this box is intentionally generous so Cesium will
/// still request edge tiles that we may actually serve via partial coverage.
const JAPAN_BOUNDS_WEST: f64 = 122.0;
const JAPAN_BOUNDS_SOUTH: f64 = 20.0;
const JAPAN_BOUNDS_EAST: f64 = 154.0;
const JAPAN_BOUNDS_NORTH: f64 = 46.0;

fn japan_availability(max_zoom: u8) -> Vec<Vec<TileAvailability>> {
    (0..=max_zoom)
        .map(|z| {
            vec![TileAvailability::from_bounds_geodetic(
                z,
                JAPAN_BOUNDS_WEST,
                JAPAN_BOUNDS_SOUTH,
                JAPAN_BOUNDS_EAST,
                JAPAN_BOUNDS_NORTH,
            )]
        })
        .collect()
}

/// Shared terrain state, constructed from config at startup.
pub struct TerrainState {
    pub dem: Arc<dyn DemProvider>,
    pub tile_size: u32,
    pub default_geoid: GeoidModel,
    pub max_zoom: u8,
    pub max_error: f64,
}

#[derive(Debug, Deserialize, Default)]
pub struct GeoidQuery {
    #[serde(default)]
    pub geoid: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

#[allow(clippy::result_large_err)]
fn resolve_geoid(state: &TerrainState, q: &GeoidQuery) -> Result<GeoidModel, Response> {
    match q.geoid.as_deref() {
        None => Ok(state.default_geoid),
        Some(s) => s
            .parse::<GeoidModel>()
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("{e}")).into_response()),
    }
}

/// Cache-key builder. Keeps DEM version, DEM upstream etag (if any), geoid,
/// and pixel size in separate path segments so that CDN partial-purge is not
/// required: different versions/geoids live at different keys.
struct TerrainCacheKey<'a> {
    prefix: &'a str,
    dem_slug: &'a str,
    dem_version: &'a str,
    dem_etag_digest: &'a str,
    geoid: GeoidModel,
    z: u32,
    x: u32,
    y: u32,
    ext: &'a str,
    size: Option<u32>,
}

impl TerrainCacheKey<'_> {
    fn to_key(&self) -> String {
        let size_str = self.size.map_or_else(String::new, |s| format!("{s}/"));
        format!(
            "{prefix}/{dem_slug}/{dem_version}/{dem_etag_digest}/{g}/{size}{z}/{x}/{y}.{ext}",
            prefix = self.prefix,
            dem_slug = self.dem_slug,
            dem_version = self.dem_version,
            dem_etag_digest = self.dem_etag_digest,
            g = self.geoid.slug(),
            size = size_str,
            z = self.z,
            x = self.x,
            y = self.y,
            ext = self.ext,
        )
    }
}

fn digest(s: &str) -> String {
    format!("{:x}", xxh64(s.as_bytes(), 0))
}

// ─────────────────────────────── Handlers ───────────────────────────────

/// Embedded Cesium viewer for quick eyeballing of terrain output.
const VIEWER_HTML: &str = include_str!("terrain_viewer.html");

/// GET /terrain-viewer
pub async fn terrain_viewer(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sources = state.list_sources().await;
    let sources_json = serde_json::to_string(&sources).unwrap_or_else(|_| "[]".to_string());
    let sources_json_safe = sources_json.replace("</", "<\\/");
    Html(VIEWER_HTML.replace("{{SOURCES_JSON}}", &sources_json_safe))
}

/// GET /terrain/layer.json
pub async fn terrain_layer_json(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<GeoidQuery>,
) -> Response {
    let terrain = state.terrain.clone();
    let geoid_model = match resolve_geoid(&terrain, &q) {
        Ok(g) => g,
        Err(r) => return r,
    };

    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");
    let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    let tiles_template = format!(
        "{scheme}://{host}/terrain/{{z}}/{{x}}/{{y}}.terrain?geoid={}",
        geoid_model.slug()
    );

    let config = crate::terrain::layer_json::LayerJsonConfig {
        tiles_template,
        version: terrain.dem.version().to_string(),
        attribution: Some(
            r#"<a href="https://www.mlit.go.jp/plateau/" target="_blank">PLATEAU</a> | <a href="https://mapterhorn.com/" target="_blank">Mapterhorn</a> | <a href="https://www.gsi.go.jp/" target="_blank">国土地理院</a>"#
                .to_string(),
        ),
        available: japan_availability(terrain.max_zoom),
        min_zoom: Some(0),
        max_zoom: Some(terrain.max_zoom),
        scheme: "tms".to_string(),
        bounds: Some([
            JAPAN_BOUNDS_WEST,
            JAPAN_BOUNDS_SOUTH,
            JAPAN_BOUNDS_EAST,
            JAPAN_BOUNDS_NORTH,
        ]),
        extensions: vec!["octvertexnormals".to_string()],
        format: "quantized-mesh-1.0".to_string(),
        metadata_availability: None,
    };

    let layer = crate::terrain::layer_json::generate_layer_json(&config);
    match serde_json::to_string(&layer) {
        Ok(json) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/json"),
                (
                    header::CACHE_CONTROL,
                    state
                        .cache_control
                        .as_deref()
                        .unwrap_or("public, max-age=3600"),
                ),
            ],
            json,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("layer.json: {e}"),
        )
            .into_response(),
    }
}

/// GET /terrain/{z}/{x}/{y}.terrain
pub async fn terrain_tile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((z, x, y_ext)): Path<(u8, u32, String)>,
    Query(q): Query<GeoidQuery>,
) -> Response {
    let terrain = state.terrain.clone();

    // Parse y and validate extension.
    let y: u32 = match y_ext.strip_suffix(".terrain").and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            return (StatusCode::BAD_REQUEST, "Expected {y}.terrain suffix").into_response();
        }
    };

    let geoid_model = match resolve_geoid(&terrain, &q) {
        Ok(g) => g,
        Err(r) => return r,
    };
    let geoid = Geoid::load(geoid_model);

    // 404 if tile is entirely outside geoid coverage.
    let bounds = geodetic_tms_bounds(z, x, y);
    if !geoid.bounds_have_any_coverage(bounds.west, bounds.south, bounds.east, bounds.north) {
        return (StatusCode::NOT_FOUND, "Out of geoid coverage").into_response();
    }

    // Fetch Mercator DEM tiles + resample to 65×65 geodetic grid.
    let dem_zoom = z.min(terrain.dem.max_zoom());
    let fetch = match fetch_geodetic_tile_elevations(
        terrain.dem.as_ref(),
        dem_zoom,
        x,
        y,
        terrain.dem.native_tile_size(),
    )
    .await
    {
        Ok(r) => r,
        Err(crate::terrain::DemError::NotFound) => {
            return (StatusCode::NOT_FOUND, "DEM tile not found").into_response();
        }
        Err(e) => {
            tracing::error!(error=%e, "DEM fetch failed");
            return (StatusCode::BAD_GATEWAY, "Upstream DEM error").into_response();
        }
    };

    // ETag composition: dem-version + aggregated dem-etags + geoid slug + coord.
    let upstream_etag_digest = digest(&fetch.source_etags.join("|"));
    let etag_keys: Vec<String> = vec![
        format!("dem-ver:{}", terrain.dem.version()),
        format!("dem-etag:{}", upstream_etag_digest),
        format!("geoid:{}", geoid_model.slug()),
    ];
    let etag = compute_etag(&etag_keys, "terrain", TileFormat::Png, z as u32, x, y);
    let etag_hash = format!("{:x}", xxh64(etag_keys.join("|").as_bytes(), 0));

    if etag_matches(&headers, &etag) {
        return not_modified_response(&etag, state.cache_control.as_deref());
    }

    let cache_key = TerrainCacheKey {
        prefix: "terrain",
        dem_slug: terrain.dem.slug(),
        dem_version: terrain.dem.version(),
        dem_etag_digest: &upstream_etag_digest,
        geoid: geoid_model,
        z: z as u32,
        x,
        y,
        ext: "terrain",
        size: None,
    }
    .to_key();

    let meta = CacheObjectMeta {
        content_type: Some("application/vnd.quantized-mesh".to_string()),
        etag_hash: Some(etag_hash.clone()),
        etag: Some(etag.clone()),
    };

    let max_error = terrain.max_error;
    let max_zoom = terrain.max_zoom;
    let mut elevations = fetch.elevations;
    let bounds_copy = bounds;
    let geoid_for_gen = geoid_model;

    let result = state
        .cache
        .get_or_generate(
            &cache_key,
            Some(&etag_hash),
            Some(meta),
            move || async move {
                // Apply geoid (ortho → ellipsoid).
                let g = Geoid::load(geoid_for_gen);
                apply_geoid_to_grid(&bounds_copy, &mut elevations, &g);

                let qm_bounds = QmBounds::new(
                    bounds_copy.west,
                    bounds_copy.south,
                    bounds_copy.east,
                    bounds_copy.north,
                );
                let opts = QuantizedMeshOptions {
                    max_error,
                    include_normals: true,
                    include_water_mask: false,
                    water_mask: None,
                    // Per-tile metadata extension is only needed when layer.json
                    // declares `metadataAvailability` (dynamic availability). We
                    // ship a static `available` array instead, so metadata would
                    // make Cesium try to call `_availability.addAvailableTileRange`
                    // on an undefined tracker and blow up.
                    include_metadata: false,
                    tile_x: Some(x),
                    tile_y: Some(y),
                    current_zoom: Some(z),
                    max_zoom: Some(max_zoom),
                    compression_level: 6,
                };
                let tile = generate_quantized_mesh_tile(&elevations, &qm_bounds, &opts);
                Ok::<_, crate::tile::TileError>(tile.data)
            },
        )
        .await;

    match result {
        Ok(bytes) => {
            let cc = state.cache_control.as_deref();
            let mut resp =
                tile_response_raw(bytes, "application/vnd.quantized-mesh", Some(&etag), cc);
            resp.headers_mut()
                .insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
            resp
        }
        Err(e) => {
            tracing::error!("terrain generate failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "terrain generation failed",
            )
                .into_response()
        }
    }
}

/// GET /terrarium/{z}/{x}/{y}.{png|webp|avif}
pub async fn terrarium_tile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((z, x, y_ext)): Path<(u8, u32, String)>,
    Query(q): Query<GeoidQuery>,
) -> Response {
    let terrain = state.terrain.clone();

    let (y, format) = match parse_y_and_format(&y_ext) {
        Some(parsed) => parsed,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Invalid y coordinate or unsupported format (use .png/.webp/.avif)",
            )
                .into_response();
        }
    };

    let geoid_model = match resolve_geoid(&terrain, &q) {
        Ok(g) => g,
        Err(r) => return r,
    };
    let geoid = Geoid::load(geoid_model);

    let bounds = geodetic_tms_bounds(z, x, y);
    if !geoid.bounds_have_any_coverage(bounds.west, bounds.south, bounds.east, bounds.north) {
        return (StatusCode::NOT_FOUND, "Out of geoid coverage").into_response();
    }

    let dem_zoom = z.min(terrain.dem.max_zoom());
    let fetch = match fetch_geodetic_tile_elevations(
        terrain.dem.as_ref(),
        dem_zoom,
        x,
        y,
        terrain.dem.native_tile_size(),
    )
    .await
    {
        Ok(r) => r,
        Err(crate::terrain::DemError::NotFound) => {
            return (StatusCode::NOT_FOUND, "DEM tile not found").into_response();
        }
        Err(e) => {
            tracing::error!(error=%e, "DEM fetch failed");
            return (StatusCode::BAD_GATEWAY, "Upstream DEM error").into_response();
        }
    };

    let upstream_etag_digest = digest(&fetch.source_etags.join("|"));
    let etag_keys: Vec<String> = vec![
        format!("dem-ver:{}", terrain.dem.version()),
        format!("dem-etag:{}", upstream_etag_digest),
        format!("geoid:{}", geoid_model.slug()),
        format!("size:{}", terrain.tile_size),
    ];
    let etag = compute_etag(&etag_keys, "terrarium", format, z as u32, x, y);
    let etag_hash = format!("{:x}", xxh64(etag_keys.join("|").as_bytes(), 0));

    if etag_matches(&headers, &etag) {
        return not_modified_response(&etag, state.cache_control.as_deref());
    }

    let cache_key = TerrainCacheKey {
        prefix: "terrarium",
        dem_slug: terrain.dem.slug(),
        dem_version: terrain.dem.version(),
        dem_etag_digest: &upstream_etag_digest,
        geoid: geoid_model,
        z: z as u32,
        x,
        y,
        ext: format.extension(),
        size: Some(terrain.tile_size),
    }
    .to_key();

    let meta = CacheObjectMeta {
        content_type: Some(format.content_type().to_string()),
        etag_hash: Some(etag_hash.clone()),
        etag: Some(etag.clone()),
    };

    let tile_size = terrain.tile_size;
    let mut elevations = fetch.elevations;
    let bounds_copy = bounds;
    let geoid_for_gen = geoid_model;

    let result = state
        .cache
        .get_or_generate(
            &cache_key,
            Some(&etag_hash),
            Some(meta),
            move || async move {
                let g = Geoid::load(geoid_for_gen);
                apply_geoid_to_grid(&bounds_copy, &mut elevations, &g);

                // Upsample 65×65 ellipsoid grid to tile_size × tile_size.
                let upsampled = upsample_grid(&elevations, CESIUM_TILE_SIZE, tile_size);
                let img_rgb = encode_terrarium(&upsampled, tile_size, tile_size);
                let img_rgba = rgb_to_rgba(&img_rgb);
                encode_image(&img_rgba, format)
                    .map_err(|e| crate::tile::TileError::ImageError(e.to_string()))
            },
        )
        .await;

    match result {
        Ok(bytes) => tile_response(bytes, format, Some(&etag), state.cache_control.as_deref()),
        Err(e) => {
            tracing::error!("terrarium generate failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "terrarium generation failed",
            )
                .into_response()
        }
    }
}

/// GET /terrarium/tilejson.json
pub async fn terrarium_tilejson(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<GeoidQuery>,
) -> Response {
    let terrain = state.terrain.clone();
    let geoid_model = match resolve_geoid(&terrain, &q) {
        Ok(g) => g,
        Err(r) => return r,
    };
    let fmt = q.format.as_deref().unwrap_or("webp");
    if !["png", "webp", "avif"].contains(&fmt) {
        return (StatusCode::BAD_REQUEST, "format must be png, webp, or avif").into_response();
    }
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");
    let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };
    let tile_url = format!(
        "{scheme}://{host}/terrarium/{{z}}/{{x}}/{{y}}.{fmt}?geoid={}",
        geoid_model.slug()
    );

    #[derive(Serialize)]
    struct TileJson<'a> {
        tilejson: &'a str,
        tiles: Vec<String>,
        name: &'a str,
        attribution: &'a str,
        scheme: &'a str,
        minzoom: u8,
        maxzoom: u8,
    }

    Json(TileJson {
        tilejson: "3.0.0",
        tiles: vec![tile_url],
        name: "terrarium-ellipsoid",
        attribution:
            r#"<a href="https://www.mlit.go.jp/plateau/" target="_blank">PLATEAU</a> | <a href="https://mapterhorn.com/" target="_blank">Mapterhorn</a> | <a href="https://www.gsi.go.jp/" target="_blank">国土地理院</a>"#,
        scheme: "xyz",
        minzoom: 0,
        maxzoom: terrain.max_zoom,
    })
    .into_response()
}

// ─────────────────────────────── Helpers ───────────────────────────────

fn rgb_to_rgba(img: &image::RgbImage) -> image::RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut out = image::RgbaImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels() {
        out.put_pixel(x, y, image::Rgba([p.0[0], p.0[1], p.0[2], 255]));
    }
    out
}

fn upsample_grid(src: &[f64], src_n: u32, dst_n: u32) -> Vec<f64> {
    if src_n == dst_n {
        return src.to_vec();
    }
    let src_n_i = src_n as usize;
    let mut out = Vec::with_capacity((dst_n * dst_n) as usize);
    for dy in 0..dst_n {
        for dx in 0..dst_n {
            let sx = dx as f64 * (src_n - 1) as f64 / (dst_n - 1).max(1) as f64;
            let sy = dy as f64 * (src_n - 1) as f64 / (dst_n - 1).max(1) as f64;
            let x0 = sx.floor() as usize;
            let y0 = sy.floor() as usize;
            let x1 = (x0 + 1).min(src_n_i - 1);
            let y1 = (y0 + 1).min(src_n_i - 1);
            let fx = sx - x0 as f64;
            let fy = sy - y0 as f64;
            let get = |x: usize, y: usize| -> f64 { src[y * src_n_i + x] };
            let v00 = get(x0, y0);
            let v10 = get(x1, y0);
            let v01 = get(x0, y1);
            let v11 = get(x1, y1);
            let v0 = v00 * (1.0 - fx) + v10 * fx;
            let v1 = v01 * (1.0 - fx) + v11 * fx;
            out.push(v0 * (1.0 - fy) + v1 * fy);
        }
    }
    out
}

/// Local variant of `tile_response` with an explicit content-type (used for
/// the `.terrain` endpoint which is not a regular image format).
fn tile_response_raw(
    data: Vec<u8>,
    content_type: &str,
    etag: Option<&str>,
    cache_control: Option<&str>,
) -> Response {
    use axum::body::Body;
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type);
    if let Some(etag) = etag {
        builder = builder.header(header::ETAG, etag);
    }
    if let Some(cc) = cache_control {
        builder = builder.header(header::CACHE_CONTROL, cc);
    }
    builder.body(Body::from(data)).unwrap().into_response()
}

// Suppress unused: `GeodeticBounds` re-export keeps clippy quiet in alt paths.
#[allow(dead_code)]
fn _assert_bounds(_b: &GeodeticBounds) {}
