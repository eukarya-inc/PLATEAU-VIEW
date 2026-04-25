//! Terrain (quantized-mesh-1.0) and raster DEM HTTP handlers.
//!
//! - `GET /terrain/layer.json[?geoid=...]`
//! - `GET /terrain/{z}/{x}/{y}.terrain[?geoid=...]` — gzipped quantized-mesh-1.0
//!   (Cesium **TMS Geodetic** addressing).
//! - `GET /terrarium/{z}/{x}/{y}.{png|webp|avif}[?geoid=...]` — ellipsoid-height
//!   Mapzen Terrarium tiles in **Web Mercator XYZ**.
//! - `GET /terrarium/tilejson.json[?geoid=...&format=...]`
//! - `GET /mapbox/{z}/{x}/{y}.{png|webp|avif}[?geoid=...]` — ellipsoid-height
//!   Mapbox Terrain-RGB tiles in **Web Mercator XYZ**.
//! - `GET /mapbox/tilejson.json[?geoid=...&format=...]`
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
    ellipsoid::{apply_geoid_to_grid, apply_geoid_to_xyz_grid},
    extract_and_upsample,
    geodetic::{GeodeticBounds, fetch_geodetic_tile_elevations, geodetic_tms_bounds},
    layer_json::TileAvailability,
    mapbox::encode_mapbox,
    mesh_gen::{QuantizedMeshOptions, generate_quantized_mesh_tile},
    quantized_mesh::TileBounds as QmBounds,
    terrarium::encode_terrarium,
    webmercator::xyz_tile_bounds,
};

/// Output encoding for the elevation raster endpoints.
#[derive(Debug, Clone, Copy)]
enum RasterEncoding {
    Terrarium,
    Mapbox,
}

impl RasterEncoding {
    fn slug(self) -> &'static str {
        match self {
            Self::Terrarium => "terrarium",
            Self::Mapbox => "mapbox",
        }
    }

    /// Internal cache/etag prefix. Includes the `-xyz` suffix to invalidate
    /// any caches written by the previous TMS-Geodetic implementation, which
    /// shared numeric `(z,x,y)` coordinates with the new XYZ scheme.
    fn cache_prefix(self) -> &'static str {
        match self {
            Self::Terrarium => "terrarium-xyz",
            Self::Mapbox => "mapbox-xyz",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Terrarium => "terrarium-ellipsoid",
            Self::Mapbox => "mapbox-terrain-rgb-ellipsoid",
        }
    }

    fn encode(self, elevations: &[f64], width: u32, height: u32) -> image::RgbImage {
        match self {
            Self::Terrarium => encode_terrarium(elevations, width, height),
            Self::Mapbox => encode_mapbox(elevations, width, height),
        }
    }
}

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

/// Resolve the public `(scheme, host)` for URLs we embed in JSON responses.
/// Front proxies (Cloud Run, Cloudflare) often rewrite the `Host` header to
/// `localhost`, so we prefer `Forwarded` / `X-Forwarded-Host` /
/// `X-Forwarded-Proto` when present, and only fall back to the `Host`
/// header (with a localhost-aware scheme guess) when nothing else is given.
pub fn external_origin(headers: &HeaderMap) -> (String, String) {
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string());

    let host = headers
        .get("x-forwarded-host")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get(header::HOST)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "localhost".to_string());

    let scheme = proto.unwrap_or_else(|| {
        if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
            "http".to_string()
        } else {
            "https".to_string()
        }
    });

    (scheme, host)
}

// ─────────────────────────────── Handlers ───────────────────────────────

/// Embedded Cesium viewer for quick eyeballing of terrain output.
const VIEWER_HTML: &str = include_str!("terrain_viewer.html");

/// GET /terrain-viewer
pub async fn terrain_viewer() -> impl IntoResponse {
    Html(VIEWER_HTML)
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

    // Use a relative tile URL so Cesium resolves it against the layer.json
    // location. Avoids any reliance on `Host` / `X-Forwarded-Host`, which
    // fronting load balancers (Cloud Run / Cloudflare) sometimes rewrite to
    // `localhost` and trigger Chrome's Private Network Access prompt.
    // The viewer passes `geoid` via `Resource.queryParameters`, which Cesium
    // automatically propagates onto every derived tile request — so we
    // don't put it in the template here (avoids duplicate `?geoid=`).
    let _ = &headers;
    let _ = geoid_model;
    let tiles_template = "{z}/{x}/{y}.terrain".to_string();

    // Cesium quantized-mesh has no per-tile upsample fallback — clamp the
    // advertised max zoom to the upstream DEM's max so Cesium never asks
    // for a z > DEM_MAX_ZOOM tile (which would be served from a wrong
    // parent and lose the geoid offset). The raster `/mapbox` and
    // `/terrarium` endpoints upsample beyond DEM_MAX_ZOOM, so their
    // tilejson advertises the full `terrain.max_zoom`.
    let qm_max_zoom = terrain.max_zoom.min(terrain.dem.max_zoom());
    let config = crate::terrain::layer_json::LayerJsonConfig {
        tiles_template,
        version: terrain.dem.version().to_string(),
        attribution: Some(
            r#"<a href="https://www.mlit.go.jp/plateau/" target="_blank">PLATEAU</a> | <a href="https://mapterhorn.com/" target="_blank">Mapterhorn</a> | <a href="https://www.gsi.go.jp/" target="_blank">国土地理院</a>"#
                .to_string(),
        ),
        available: japan_availability(qm_max_zoom),
        min_zoom: Some(0),
        max_zoom: Some(qm_max_zoom),
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
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    path: Path<(u8, u32, String)>,
    query: Query<GeoidQuery>,
) -> Response {
    raster_tile(state, headers, path, query, RasterEncoding::Terrarium).await
}

/// GET /mapbox/{z}/{x}/{y}.{png|webp|avif}
pub async fn mapbox_tile(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    path: Path<(u8, u32, String)>,
    query: Query<GeoidQuery>,
) -> Response {
    raster_tile(state, headers, path, query, RasterEncoding::Mapbox).await
}

async fn raster_tile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((z, x, y_ext)): Path<(u8, u32, String)>,
    Query(q): Query<GeoidQuery>,
    encoding: RasterEncoding,
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

    // Web Mercator XYZ tile bounds for coverage check.
    let bounds = xyz_tile_bounds(z, x, y);
    if !geoid.bounds_have_any_coverage(bounds.west, bounds.south, bounds.east, bounds.north) {
        return (StatusCode::NOT_FOUND, "Out of geoid coverage").into_response();
    }

    // Fetch DEM at the requested XYZ tile directly — no reprojection.
    // For zooms above the upstream DEM's max, fall back to the parent tile
    // and upsample the relevant sub-region (per stralift's behavior).
    let tile_size = terrain.tile_size;
    let dem_max = terrain.dem.max_zoom();
    let (fetch_z, fetch_x, fetch_y, upsample_info) = if z > dem_max {
        let zoom_diff = z - dem_max;
        let factor = 1u32 << zoom_diff;
        (
            dem_max,
            x / factor,
            y / factor,
            Some((zoom_diff, x % factor, y % factor)),
        )
    } else {
        (z, x, y, None)
    };
    let dem_tile = match terrain
        .dem
        .get_tile_elevations(fetch_z, fetch_x, fetch_y, tile_size)
        .await
    {
        Ok(t) => t,
        Err(crate::terrain::DemError::NotFound) => {
            return (StatusCode::NOT_FOUND, "DEM tile not found").into_response();
        }
        Err(crate::terrain::DemError::OutOfRange) => {
            return (StatusCode::NOT_FOUND, "DEM zoom out of range").into_response();
        }
        Err(e) => {
            tracing::error!(error=%e, "DEM fetch failed");
            return (StatusCode::BAD_GATEWAY, "Upstream DEM error").into_response();
        }
    };

    let source_etag = dem_tile.etag.unwrap_or_default();
    let upstream_etag_digest = digest(&source_etag);
    let upsample_marker = upsample_info
        .map(|(d, _, _)| format!("upsample:{d}"))
        .unwrap_or_else(|| "upsample:0".to_string());
    let etag_keys: Vec<String> = vec![
        format!("dem-ver:{}", terrain.dem.version()),
        format!("dem-etag:{}", upstream_etag_digest),
        format!("geoid:{}", geoid_model.slug()),
        format!("size:{}", tile_size),
        format!("proj:webmercator"),
        upsample_marker,
    ];
    let etag = compute_etag(&etag_keys, encoding.cache_prefix(), format, z as u32, x, y);
    let etag_hash = format!("{:x}", xxh64(etag_keys.join("|").as_bytes(), 0));

    if etag_matches(&headers, &etag) {
        return not_modified_response(&etag, state.cache_control.as_deref());
    }

    let cache_key = TerrainCacheKey {
        prefix: encoding.cache_prefix(),
        dem_slug: terrain.dem.slug(),
        dem_version: terrain.dem.version(),
        dem_etag_digest: &upstream_etag_digest,
        geoid: geoid_model,
        z: z as u32,
        x,
        y,
        ext: format.extension(),
        size: Some(tile_size),
    }
    .to_key();

    let meta = CacheObjectMeta {
        content_type: Some(format.content_type().to_string()),
        etag_hash: Some(etag_hash.clone()),
        etag: Some(etag.clone()),
    };

    let mut elevations = if let Some((zoom_diff, sub_x, sub_y)) = upsample_info {
        extract_and_upsample(&dem_tile.elevations, tile_size, zoom_diff, sub_x, sub_y)
    } else {
        dem_tile.elevations
    };
    let geoid_for_gen = geoid_model;

    let result = state
        .cache
        .get_or_generate(
            &cache_key,
            Some(&etag_hash),
            Some(meta),
            move || async move {
                let g = Geoid::load(geoid_for_gen);
                apply_geoid_to_xyz_grid(z, x, y, tile_size, &mut elevations, &g);

                let img_rgb = encoding.encode(&elevations, tile_size, tile_size);
                let img_rgba = rgb_to_rgba(&img_rgb);
                encode_image(&img_rgba, format)
                    .map_err(|e| crate::tile::TileError::ImageError(e.to_string()))
            },
        )
        .await;

    match result {
        Ok(bytes) => tile_response(bytes, format, Some(&etag), state.cache_control.as_deref()),
        Err(e) => {
            tracing::error!(encoding = encoding.slug(), "raster generate failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "raster generation failed",
            )
                .into_response()
        }
    }
}

/// GET /terrarium/tilejson.json
pub async fn terrarium_tilejson(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    query: Query<GeoidQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Terrarium).await
}

/// GET /mapbox/tilejson.json
pub async fn mapbox_tilejson(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    query: Query<GeoidQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Mapbox).await
}

async fn raster_tilejson(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<GeoidQuery>,
    encoding: RasterEncoding,
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
    // Use an origin-relative URL so MapLibre resolves it against the
    // tilejson location. Avoids any reliance on `Host` / `X-Forwarded-Host`,
    // which fronting load balancers (Cloud Run / Cloudflare) sometimes
    // rewrite to `localhost` — same fix as `/terrain/layer.json`.
    let _ = &headers;
    let tile_url = format!(
        "/{slug}/{{z}}/{{x}}/{{y}}.{fmt}?geoid={geoid}",
        slug = encoding.slug(),
        geoid = geoid_model.slug(),
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
        bounds: [f64; 4],
        encoding: &'a str,
    }

    Json(TileJson {
        tilejson: "3.0.0",
        tiles: vec![tile_url],
        name: encoding.name(),
        attribution:
            r#"<a href="https://www.mlit.go.jp/plateau/" target="_blank">PLATEAU</a> | <a href="https://mapterhorn.com/" target="_blank">Mapterhorn</a> | <a href="https://www.gsi.go.jp/" target="_blank">国土地理院</a>"#,
        scheme: "xyz",
        minzoom: 0,
        maxzoom: terrain.max_zoom,
        bounds: [
            JAPAN_BOUNDS_WEST,
            JAPAN_BOUNDS_SOUTH,
            JAPAN_BOUNDS_EAST,
            JAPAN_BOUNDS_NORTH,
        ],
        encoding: encoding.slug(),
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
