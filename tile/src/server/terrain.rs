//! Terrain (quantized-mesh-1.0) and raster DEM HTTP handlers.
//!
//! - `GET /terrain/layer.json[?heights=...]`
//! - `GET /terrain/{z}/{x}/{y}.terrain[?heights=...]` — gzipped quantized-mesh-1.0
//!   (Cesium **TMS Geodetic** addressing).
//! - `GET /terrarium/{z}/{x}/{y}.{png|webp|avif}[?heights=...]` — Mapzen
//!   Terrarium tiles in **Web Mercator XYZ**.
//! - `GET /terrarium/tilejson.json[?heights=...&format=...]`
//! - `GET /mapbox/{z}/{x}/{y}.{png|webp|avif}[?heights=...]` — Mapbox
//!   Terrain-RGB tiles in **Web Mercator XYZ**.
//! - `GET /mapbox/tilejson.json[?heights=...&format=...]`
//!
//! Heights default to ellipsoidal (orthometric DEM + the source's geoid).
//! `?heights=orthometric` serves the DEM as-is and `?heights=geoid` the geoid
//! surface alone; see [`HeightMode`]. The geoid *model* is **not** selectable
//! per request — it is a property of the DEM source (config `geoid`, falling
//! back to `TERRAIN_DEFAULT_GEOID`), because a model is bound to a vertical
//! datum and mixing the two produces meaningless numbers. The old `?geoid=`
//! parameter is therefore rejected with 400 rather than ignored.
//!
//! Tiles entirely outside the geoid's coverage area respond 404.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use terrain_codec::heightmap::{HeightmapFormat, encode_pixel as encode_heightmap_pixel};
use terrain_codec::layer_json::{
    LayerJson, LayerJsonConfig, TerrainFormat, TileAvailability, TilingScheme,
};
use terrain_codec::normals::BufferedElevations;
use terrain_codec::quantized_mesh::TileBounds as QmBounds;
use xxhash_rust::xxh64::xxh64;

use super::format::{TileFormat, encode_image, parse_y_and_format};
use super::response::{compute_etag, etag_matches, not_modified_response, tile_response};
use super::state::{AppState, TerrainBackend};
use crate::cache::CacheObjectMeta;
use crate::terrain::{
    DemProvider, Geoid, GeoidModel, HeightMode, MirrorSource,
    ellipsoid::{
        apply_height_mode_to_grid, apply_height_mode_to_grid_sized, apply_height_mode_to_xyz_grid,
    },
    extract_and_upsample,
    geodetic::{GeodeticBounds, fetch_geodetic_tile_elevations_with_halo, geodetic_tms_bounds},
    mesh_gen::{NormalMode, QuantizedMeshOptions, generate_quantized_mesh_tile},
    webmercator::xyz_tile_bounds,
};

/// Attribution shown for both the DEM-generated `/terrain/dem/...` source and
/// the R2 quantized-mesh mirror — same upstream lineage (PLATEAU + Mapterhorn
/// + 国土地理院), so the credit line is identical.
const TERRAIN_ATTRIBUTION_HTML: &str = r#"<a href="https://www.mlit.go.jp/plateau/" target="_blank">PLATEAU</a> | <a href="https://mapterhorn.com/" target="_blank">Mapterhorn</a> | <a href="https://www.gsi.go.jp/" target="_blank">国土地理院</a>"#;

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

    fn heightmap_format(self) -> HeightmapFormat {
        match self {
            Self::Terrarium => HeightmapFormat::Terrarium,
            Self::Mapbox => HeightmapFormat::Mapbox,
        }
    }

    /// Encode elevations directly into a fresh `RgbaImage`. Writes each
    /// pixel straight into the destination buffer so no intermediate
    /// `Vec<f32>` (for the f64→f32 cast) or `Vec<u8>` (for the RGB stream)
    /// is allocated — only the final `RgbaImage` that the PNG/WebP encoder
    /// consumes.
    fn encode_to_rgba(self, elevations: &[f64], width: u32, height: u32) -> image::RgbaImage {
        let fmt = self.heightmap_format();
        debug_assert_eq!(elevations.len(), (width as usize) * (height as usize));
        let mut out = image::RgbaImage::new(width, height);
        for (dst, &elev) in out.pixels_mut().zip(elevations.iter()) {
            let [r, g, b] = encode_heightmap_pixel(fmt, elev as f32);
            *dst = image::Rgba([r, g, b, 255]);
        }
        out
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
            vec![TileAvailability::from_bounds_geodetic_tms(
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
    /// Geoid model this DEM source's elevations are referenced to. Comes from
    /// the source's `geoid` in the config JSON, else `TERRAIN_DEFAULT_GEOID`.
    /// Fixed per source — requests cannot change it.
    pub geoid: GeoidModel,
    pub max_zoom: u8,
    pub max_error: f64,
}

#[derive(Debug, Deserialize, Default)]
pub struct TerrainQuery {
    /// Vertical surface to serve: `orthometric` | `geoid` | `ellipsoidal`
    /// (default). See [`HeightMode`].
    #[serde(default)]
    pub heights: Option<String>,
    /// Retired model selector, still parsed so we can 400 on it explicitly
    /// instead of silently serving a different model than the caller asked for.
    #[serde(default)]
    pub geoid: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

/// Resolve the requested [`HeightMode`], rejecting the retired `?geoid=`
/// model selector.
///
/// This is a deliberate breaking change: a caller that still passes
/// `?geoid=gsigeo2011` gets a 400 naming the replacement and its valid values.
/// Ignoring the parameter would silently serve a *different* geoid than asked
/// for, which is exactly the class of vertical-datum mistake this change
/// exists to prevent.
#[allow(clippy::result_large_err)]
fn resolve_height_mode(q: &TerrainQuery) -> Result<HeightMode, Response> {
    if let Some(v) = q.geoid.as_deref() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "the `geoid` query parameter has been removed (got `geoid={v}`): the geoid \
                 model is a property of the DEM source (config `geoid`, or the \
                 TERRAIN_DEFAULT_GEOID env var), not of the request. Use \
                 `heights={valid}` instead (default: {default})",
                valid = HeightMode::valid_values().replace(", ", "|"),
                default = HeightMode::default(),
            ),
        )
            .into_response());
    }
    match q.heights.as_deref().map(str::trim) {
        None | Some("") => Ok(HeightMode::default()),
        Some(s) => s
            .parse::<HeightMode>()
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("{e}")).into_response()),
    }
}

/// Cache-key builder. Keeps DEM version, DEM upstream etag (if any), geoid
/// model, height mode, and pixel size in separate path segments so that CDN
/// partial-purge is not required: different versions/models/modes live at
/// different keys.
///
/// Both the model and the mode belong here. The model because a source's
/// configured `geoid` can change on a config reload without the DEM slug or
/// version changing; the mode because one source now serves three different
/// surfaces from identical DEM input.
struct TerrainCacheKey<'a> {
    prefix: &'a str,
    dem_slug: &'a str,
    dem_version: &'a str,
    dem_etag_digest: &'a str,
    geoid: GeoidModel,
    heights: HeightMode,
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
            "{prefix}/{dem_slug}/{dem_version}/{dem_etag_digest}/{g}/{h}/{size}{z}/{x}/{y}.{ext}",
            prefix = self.prefix,
            dem_slug = self.dem_slug,
            dem_version = self.dem_version,
            dem_etag_digest = self.dem_etag_digest,
            g = self.geoid.slug(),
            h = self.heights.slug(),
            size = size_str,
            z = self.z,
            x = self.x,
            y = self.y,
            ext = self.ext,
        )
    }
}

/// ETag components shared by every terrain endpoint: the source's geoid model
/// and the requested height mode. Both must be present or the three modes (and
/// two models) could serve each other's bytes from memory / R2 / the CDN.
fn vertical_etag_keys(geoid: GeoidModel, heights: HeightMode) -> [String; 2] {
    [
        format!("geoid:{}", geoid.slug()),
        format!("heights:{}", heights.slug()),
    ]
}

fn digest(s: &str) -> String {
    format!("{:x}", xxh64(s.as_bytes(), 0))
}

// ─────────────────────────────── Handlers ───────────────────────────────

/// Cesium viewer for quick eyeballing of terrain output. Source HTML is
/// loaded from disk at request time — see [`super::static_assets`].
///
/// GET /terrain-viewer
pub async fn terrain_viewer() -> axum::response::Response {
    super::static_assets::serve_html("terrain_viewer.html")
}

/// GET /terrain/layer.json   (default DEM source)
pub async fn terrain_layer_json(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    query: Query<TerrainQuery>,
) -> Response {
    terrain_layer_json_impl(state, headers, query, None).await
}

/// GET /terrain/{name}/layer.json
pub async fn terrain_layer_json_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    query: Query<TerrainQuery>,
) -> Response {
    terrain_layer_json_impl(state, headers, query, Some(name)).await
}

async fn terrain_layer_json_impl(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<TerrainQuery>,
    name: Option<String>,
) -> Response {
    match state.resolve_terrain(name.as_deref()).await {
        Some(TerrainBackend::Mirror(m)) => mirror_layer_json_response(m, &state).await,
        Some(TerrainBackend::Dem(t)) => dem_layer_json_response(t, &state, headers, q),
        None => (StatusCode::NOT_FOUND, "Unknown terrain source").into_response(),
    }
}

fn dem_layer_json_response(
    terrain: Arc<super::terrain::TerrainState>,
    state: &AppState,
    headers: HeaderMap,
    q: TerrainQuery,
) -> Response {
    // Validate the query even though layer.json's body doesn't depend on it:
    // a stale `?geoid=` must fail here too, otherwise Cesium would propagate it
    // onto every tile request and the caller would only see the 400 at tile
    // level (or, worse, take the 400s for missing tiles).
    let height_mode = match resolve_height_mode(&q) {
        Ok(m) => m,
        Err(r) => return r,
    };

    // Use a relative tile URL so Cesium resolves it against the layer.json
    // location. Avoids any reliance on `Host` / `X-Forwarded-Host`, which
    // fronting load balancers (Cloud Run / Cloudflare) sometimes rewrite to
    // `localhost` and trigger Chrome's Private Network Access prompt.
    // The viewer passes `heights` via `Resource.queryParameters`, which Cesium
    // automatically propagates onto every derived tile request — so we
    // don't put it in the template here (avoids duplicate `?heights=`).
    let _ = &headers;
    let _ = height_mode;
    let tiles_template = "{z}/{x}/{y}.terrain".to_string();

    let config = LayerJsonConfig {
        tiles_template,
        version: terrain.dem.version().to_string(),
        attribution: Some(TERRAIN_ATTRIBUTION_HTML.to_string()),
        available: japan_availability(terrain.max_zoom),
        min_zoom: Some(0),
        max_zoom: Some(terrain.max_zoom),
        scheme: TilingScheme::Tms,
        bounds: Some([
            JAPAN_BOUNDS_WEST,
            JAPAN_BOUNDS_SOUTH,
            JAPAN_BOUNDS_EAST,
            JAPAN_BOUNDS_NORTH,
        ]),
        extensions: vec!["octvertexnormals".to_string()],
        format: TerrainFormat::QuantizedMesh1,
        metadata_availability: None,
    };

    let layer = LayerJson::from_config(&config);
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

/// Serve the mirror's `layer.json`. The stored file is the verbatim Ion
/// document (with its original attribution and tile template); we deserialize
/// it just enough to overwrite `attribution` with the PLATEAU credit and to
/// normalize `tiles` to a single relative template that resolves against
/// whatever URL the layer.json was loaded from.
async fn mirror_layer_json_response(mirror: Arc<MirrorSource>, state: &AppState) -> Response {
    let bytes = match mirror.fetch_layer_json().await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "terrain mirror: layer.json missing from upstream bucket",
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "terrain mirror: layer.json fetch failed");
            return (
                StatusCode::BAD_GATEWAY,
                "terrain mirror: failed to read layer.json",
            )
                .into_response();
        }
    };
    let mut layer: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("terrain mirror: layer.json parse: {e}"),
            )
                .into_response();
        }
    };
    if let Some(obj) = layer.as_object_mut() {
        obj.insert(
            "attribution".into(),
            serde_json::Value::String(TERRAIN_ATTRIBUTION_HTML.into()),
        );
        obj.insert(
            "tiles".into(),
            serde_json::Value::Array(vec![serde_json::Value::String(
                "{z}/{x}/{y}.terrain".into(),
            )]),
        );
    }
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
            format!("terrain mirror: layer.json serialize: {e}"),
        )
            .into_response(),
    }
}

/// GET /terrain-mirror/layer.json
pub async fn terrain_mirror_layer_json(state: State<Arc<AppState>>) -> Response {
    let State(state) = state;
    match state.get_mirror() {
        Some(m) => mirror_layer_json_response(m, &state).await,
        None => (
            StatusCode::NOT_FOUND,
            "terrain mirror not configured (set TERRAIN_MIRROR_URL)",
        )
            .into_response(),
    }
}

/// GET /terrain-mirror/{z}/{x}/{y}.terrain
pub async fn terrain_mirror_tile(
    state: State<Arc<AppState>>,
    Path((z, x, y_ext)): Path<(u32, u32, String)>,
) -> Response {
    let State(state) = state;
    let mirror = match state.get_mirror() {
        Some(m) => m,
        None => {
            return (
                StatusCode::NOT_FOUND,
                "terrain mirror not configured (set TERRAIN_MIRROR_URL)",
            )
                .into_response();
        }
    };
    let y: u32 = match y_ext.strip_suffix(".terrain").and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            return (StatusCode::BAD_REQUEST, "Expected {y}.terrain suffix").into_response();
        }
    };
    mirror_tile_response(&mirror, z, x, y, state.cache_control.as_deref()).await
}

/// Pass-through one tile from the configured quantized-mesh mirror. The
/// stored object already carries the right `Content-Type` and is gzipped
/// per the crawler's convention, so we don't decode/re-encode — just send
/// the bytes back with explicit `Content-Encoding: gzip` so Cesium can
/// inflate it client-side.
async fn mirror_tile_response(
    mirror: &MirrorSource,
    z: u32,
    x: u32,
    y: u32,
    cache_control: Option<&str>,
) -> Response {
    match mirror.fetch_tile(z, x, y).await {
        Ok(Some(bytes)) => {
            let cc = cache_control.unwrap_or("public, max-age=31536000, immutable");
            match Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    "application/vnd.quantized-mesh;extensions=octvertexnormals-metadata",
                )
                .header(header::CONTENT_ENCODING, "gzip")
                .header(header::CACHE_CONTROL, cc)
                .body(Body::from(bytes))
            {
                Ok(r) => r,
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("terrain mirror: build response: {e}"),
                )
                    .into_response(),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "tile not in terrain mirror").into_response(),
        Err(e) => {
            tracing::error!(z, x, y, error = %e, "terrain mirror: tile fetch failed");
            (
                StatusCode::BAD_GATEWAY,
                "terrain mirror: upstream fetch failed",
            )
                .into_response()
        }
    }
}

/// GET /terrain/{z}/{x}/{y}.terrain   (default DEM source)
pub async fn terrain_tile(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    path: Path<(u8, u32, String)>,
    query: Query<TerrainQuery>,
) -> Response {
    terrain_tile_impl(state, headers, path, query, None).await
}

/// GET /terrain/{name}/{z}/{x}/{y}.terrain
pub async fn terrain_tile_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, z, x, y_ext)): Path<(String, u8, u32, String)>,
    query: Query<TerrainQuery>,
) -> Response {
    terrain_tile_impl(state, headers, Path((z, x, y_ext)), query, Some(name)).await
}

async fn terrain_tile_impl(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((z, x, y_ext)): Path<(u8, u32, String)>,
    Query(q): Query<TerrainQuery>,
    name: Option<String>,
) -> Response {
    state.maybe_revalidate().await;

    // Parse y and validate extension once — same wire format for both
    // backends so we can short-circuit any garbage upfront.
    let y: u32 = match y_ext.strip_suffix(".terrain").and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => {
            return (StatusCode::BAD_REQUEST, "Expected {y}.terrain suffix").into_response();
        }
    };

    let terrain = match state.resolve_terrain(name.as_deref()).await {
        Some(TerrainBackend::Mirror(m)) => {
            return mirror_tile_response(&m, z as u32, x, y, state.cache_control.as_deref()).await;
        }
        Some(TerrainBackend::Dem(t)) => t,
        None => return (StatusCode::NOT_FOUND, "Unknown terrain source").into_response(),
    };

    let height_mode = match resolve_height_mode(&q) {
        Ok(m) => m,
        Err(r) => return r,
    };
    let geoid_model = terrain.geoid;
    let geoid = Geoid::load(geoid_model);

    // 404 if tile is entirely outside geoid coverage.
    let bounds = geodetic_tms_bounds(z, x, y);
    if !geoid.bounds_have_any_coverage(bounds.west, bounds.south, bounds.east, bounds.north) {
        return (StatusCode::NOT_FOUND, "Out of geoid coverage").into_response();
    }

    // Fetch Mercator DEM tiles + resample to 65×65 geodetic grid, plus a
    // 1-cell halo on every side so the mesh generator can compute DEM-
    // gradient normals that stay continuous across tile boundaries.
    //
    // `fetch_geodetic_tile_elevations_with_halo` internally clamps to the
    // DEM's max zoom and bilinear-upsamples from the parent XYZ tile when z
    // exceeds it.
    const TERRAIN_NORMAL_HALO: u32 = 1;
    let fetch = match fetch_geodetic_tile_elevations_with_halo(
        terrain.dem.as_ref(),
        z,
        x,
        y,
        terrain.dem.native_tile_size(),
        TERRAIN_NORMAL_HALO,
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

    // ETag composition: dem-version + aggregated dem-etags + geoid slug +
    // mesh-algo version + coord. The mesh-algo tag exists so terrain caches
    // (memory + persistent + downstream) roll forward when we change how the
    // mesh or its normals are computed. Bump this string on any change that
    // alters serialized .terrain bytes for unchanged DEM input — otherwise
    // operators have to manually flush caches or bump DEM_VERSION.
    //
    // v3: terrain-codec 0.3.0 — pixel-centre DEM sampling via MercatorDem and
    // mesh-vertex (not full-grid) height range, both of which change bytes.
    // v4: COG edge-chunk stride fix. Chunks on a COG's right edge were read
    // with the padded block stride, scrambling rows and leaving the remainder
    // NaN, so tiles near every integer meridian encoded wrong heights from
    // DEM input that never changed — exactly the case this tag exists for.
    // The six patch/shizuoka overlays also began resolving at the same time.
    const TERRAIN_MESH_ALGO_VERSION: &str = "v4-edge-chunk-stride";
    let upstream_etag_digest = digest(&fetch.source_etags.join("|"));
    let [geoid_key, heights_key] = vertical_etag_keys(geoid_model, height_mode);
    let etag_keys: Vec<String> = vec![
        format!("dem-ver:{}", terrain.dem.version()),
        format!("dem-etag:{}", upstream_etag_digest),
        geoid_key,
        heights_key,
        format!("mesh-algo:{TERRAIN_MESH_ALGO_VERSION}"),
    ];
    let etag = compute_etag(&etag_keys, "terrain", TileFormat::Png, z as u32, x, y);
    let etag_hash = format!("{:x}", xxh64(etag_keys.join("|").as_bytes(), 0));

    if etag_matches(&headers, &etag) {
        return not_modified_response(&etag, state.cache_control.as_deref());
    }

    // Mesh-algo tag goes into the cache prefix so persistent storage rolls
    // forward independently of DEM upstream changes — see the comment on
    // `TERRAIN_MESH_ALGO_VERSION` above. The raster endpoints use their own
    // prefix ("terrarium-xyz" / "mapbox-xyz") and stay untouched.
    let terrain_cache_prefix = format!("terrain/{TERRAIN_MESH_ALGO_VERSION}");
    let cache_key = TerrainCacheKey {
        prefix: &terrain_cache_prefix,
        dem_slug: terrain.dem.slug(),
        dem_version: terrain.dem.version(),
        dem_etag_digest: &upstream_etag_digest,
        geoid: geoid_model,
        heights: height_mode,
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
    let mut elevations = fetch.elevations;
    let mut elevations_with_halo = fetch.elevations_with_halo;
    let halo_cells = fetch.halo_cells;
    let bounds_copy = bounds;
    let geoid_for_gen = geoid_model;

    // Halo-extended bounds matching `elevations_with_halo` (one cell beyond
    // the tile on every side). Used to add the geoid to the halo cells in
    // the same vertical datum as the tile interior.
    let cell_lon = (bounds.east - bounds.west) / 64.0;
    let cell_lat = (bounds.north - bounds.south) / 64.0;
    let halo_bounds = GeodeticBounds {
        west: bounds.west - cell_lon * halo_cells as f64,
        east: bounds.east + cell_lon * halo_cells as f64,
        south: bounds.south - cell_lat * halo_cells as f64,
        north: bounds.north + cell_lat * halo_cells as f64,
    };
    let halo_grid_size = 65 + 2 * halo_cells as usize;

    let result = state
        .cache
        .get_or_generate(
            &cache_key,
            Some(&etag_hash),
            Some(meta),
            move || async move {
                // Rewrite both grids onto the requested vertical surface. The
                // halo gets the same treatment as the interior so the
                // gradient-based normals stay in one datum.
                let g = Geoid::load(geoid_for_gen);
                apply_height_mode_to_grid(&bounds_copy, &mut elevations, &g, height_mode);
                apply_height_mode_to_grid_sized(
                    &halo_bounds,
                    &mut elevations_with_halo,
                    &g,
                    halo_grid_size,
                    height_mode,
                );

                let qm_bounds = QmBounds::new(
                    bounds_copy.west,
                    bounds_copy.south,
                    bounds_copy.east,
                    bounds_copy.north,
                );
                // No water-mask and no per-tile metadata extension: metadata
                // is only needed when layer.json declares `metadataAvailability`
                // (dynamic availability). We ship a static `available` array
                // instead, so metadata would make Cesium try to call
                // `_availability.addAvailableTileRange` on an undefined
                // tracker and blow up.
                let opts = QuantizedMeshOptions {
                    max_error,
                    normals: NormalMode::BufferedGradient(BufferedElevations::new(
                        elevations_with_halo,
                        crate::terrain::mesh_gen::MESH_GRID_SIZE,
                        halo_cells,
                    )),
                };
                let data = generate_quantized_mesh_tile(&elevations, &qm_bounds, opts);
                Ok::<_, crate::tile::TileError>(data)
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
    query: Query<TerrainQuery>,
) -> Response {
    raster_tile(state, headers, path, query, RasterEncoding::Terrarium, None).await
}

/// GET /terrarium/{name}/{z}/{x}/{y}.{png|webp|avif}
pub async fn terrarium_tile_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, z, x, y_ext)): Path<(String, u8, u32, String)>,
    query: Query<TerrainQuery>,
) -> Response {
    raster_tile(
        state,
        headers,
        Path((z, x, y_ext)),
        query,
        RasterEncoding::Terrarium,
        Some(name),
    )
    .await
}

/// GET /mapbox/{z}/{x}/{y}.{png|webp|avif}
pub async fn mapbox_tile(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    path: Path<(u8, u32, String)>,
    query: Query<TerrainQuery>,
) -> Response {
    raster_tile(state, headers, path, query, RasterEncoding::Mapbox, None).await
}

/// GET /mapbox/{name}/{z}/{x}/{y}.{png|webp|avif}
pub async fn mapbox_tile_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, z, x, y_ext)): Path<(String, u8, u32, String)>,
    query: Query<TerrainQuery>,
) -> Response {
    raster_tile(
        state,
        headers,
        Path((z, x, y_ext)),
        query,
        RasterEncoding::Mapbox,
        Some(name),
    )
    .await
}

async fn raster_tile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((z, x, y_ext)): Path<(u8, u32, String)>,
    Query(q): Query<TerrainQuery>,
    encoding: RasterEncoding,
    name: Option<String>,
) -> Response {
    state.maybe_revalidate().await;
    let terrain = match state.get_terrain(name.as_deref()).await {
        Some(t) => t,
        None => return (StatusCode::NOT_FOUND, "Unknown terrain source").into_response(),
    };

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

    let height_mode = match resolve_height_mode(&q) {
        Ok(m) => m,
        Err(r) => return r,
    };
    let geoid_model = terrain.geoid;
    let geoid = Geoid::load(geoid_model);

    // Web Mercator XYZ tile bounds for coverage check.
    let bounds = xyz_tile_bounds(z, x, y);
    if !geoid.bounds_have_any_coverage(bounds.west, bounds.south, bounds.east, bounds.north) {
        return (StatusCode::NOT_FOUND, "Out of geoid coverage").into_response();
    }

    // Fetch DEM at the requested XYZ tile directly — no reprojection.
    // For zooms above the upstream DEM's max, fall back to the parent tile
    // and upsample the relevant sub-region.
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
    let [geoid_key, heights_key] = vertical_etag_keys(geoid_model, height_mode);
    let etag_keys: Vec<String> = vec![
        format!("dem-ver:{}", terrain.dem.version()),
        format!("dem-etag:{}", upstream_etag_digest),
        geoid_key,
        heights_key,
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
        heights: height_mode,
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
                apply_height_mode_to_xyz_grid(z, x, y, tile_size, &mut elevations, &g, height_mode);

                let img_rgba = encoding.encode_to_rgba(&elevations, tile_size, tile_size);
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
    query: Query<TerrainQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Terrarium, None).await
}

/// GET /terrarium/{name}/tilejson.json
pub async fn terrarium_tilejson_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    query: Query<TerrainQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Terrarium, Some(name)).await
}

/// GET /mapbox/tilejson.json
pub async fn mapbox_tilejson(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    query: Query<TerrainQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Mapbox, None).await
}

/// GET /mapbox/{name}/tilejson.json
pub async fn mapbox_tilejson_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    query: Query<TerrainQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Mapbox, Some(name)).await
}

async fn raster_tilejson(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<TerrainQuery>,
    encoding: RasterEncoding,
    name: Option<String>,
) -> Response {
    let terrain = match state.get_terrain(name.as_deref()).await {
        Some(t) => t,
        None => return (StatusCode::NOT_FOUND, "Unknown terrain source").into_response(),
    };
    let height_mode = match resolve_height_mode(&q) {
        Ok(m) => m,
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
    // Embed the source name segment when this tilejson refers to a non-default
    // DEM source, so MapLibre's resolved tile URLs hit `/{slug}/{name}/...`.
    let name_segment = name.as_deref().map(|n| format!("/{n}")).unwrap_or_default();
    // Pin the resolved height mode into the tile template so MapLibre keeps
    // requesting the same surface it asked this tilejson for — and so the two
    // never drift apart if the default ever changes.
    let tile_url = format!(
        "/{slug}{name_segment}/{{z}}/{{x}}/{{y}}.{fmt}?heights={heights}",
        slug = encoding.slug(),
        heights = height_mode.slug(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn query(heights: Option<&str>, geoid: Option<&str>) -> TerrainQuery {
        TerrainQuery {
            heights: heights.map(str::to_string),
            geoid: geoid.map(str::to_string),
            format: None,
        }
    }

    #[test]
    fn unparameterised_request_is_ellipsoidal() {
        assert_eq!(
            resolve_height_mode(&TerrainQuery::default()).unwrap(),
            HeightMode::Ellipsoidal
        );
        assert_eq!(
            resolve_height_mode(&query(Some(""), None)).unwrap(),
            HeightMode::Ellipsoidal
        );
    }

    #[test]
    fn each_mode_is_selectable() {
        for m in HeightMode::all() {
            assert_eq!(
                resolve_height_mode(&query(Some(m.slug()), None)).unwrap(),
                *m
            );
        }
    }

    #[test]
    fn model_name_in_request_is_rejected_with_400() {
        for stale in ["gsigeo2011", "jpgeo2024", "jpgeo2024-hrefconv", "none"] {
            let resp = resolve_height_mode(&query(None, Some(stale)))
                .expect_err("a geoid model in the request must be rejected");
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "geoid={stale}");
        }
    }

    #[test]
    fn geoid_parameter_is_rejected_even_alongside_a_valid_mode() {
        let resp = resolve_height_mode(&query(Some("ellipsoidal"), Some("jpgeo2024")))
            .expect_err("`geoid` must never be silently ignored");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejection_message_names_the_replacement_and_valid_values() {
        let resp = resolve_height_mode(&query(None, Some("gsigeo2011"))).unwrap_err();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("heights="), "{text}");
        for m in HeightMode::all() {
            assert!(text.contains(m.slug()), "{text}");
        }
        assert!(text.contains("TERRAIN_DEFAULT_GEOID"), "{text}");
    }

    #[test]
    fn unknown_mode_is_rejected_with_400() {
        let resp = resolve_height_mode(&query(Some("wgs84"), None)).unwrap_err();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    fn key_for(geoid: GeoidModel, heights: HeightMode) -> String {
        TerrainCacheKey {
            prefix: "terrain/v3",
            dem_slug: "dem",
            dem_version: "v1",
            dem_etag_digest: "abc",
            geoid,
            heights,
            z: 10,
            x: 909,
            y: 403,
            ext: "terrain",
            size: None,
        }
        .to_key()
    }

    /// The three modes must never be able to serve each other's tiles, in the
    /// persistent cache or through a downstream CDN.
    #[test]
    fn each_mode_gets_a_distinct_cache_key_and_etag() {
        let mut keys: Vec<String> = HeightMode::all()
            .iter()
            .map(|m| key_for(GeoidModel::Gsigeo2011, *m))
            .collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), HeightMode::all().len());

        let mut etags: Vec<String> = HeightMode::all()
            .iter()
            .map(|m| {
                compute_etag(
                    &vertical_etag_keys(GeoidModel::Gsigeo2011, *m),
                    "terrain",
                    TileFormat::Png,
                    10,
                    909,
                    403,
                )
            })
            .collect();
        etags.sort();
        etags.dedup();
        assert_eq!(etags.len(), HeightMode::all().len());
    }

    /// The model still has to key too: a source's configured `geoid` can change
    /// on a config reload without the DEM slug/version moving.
    #[test]
    fn each_model_gets_a_distinct_cache_key_and_etag() {
        let mut keys: Vec<String> = GeoidModel::all()
            .iter()
            .map(|g| key_for(*g, HeightMode::Ellipsoidal))
            .collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), GeoidModel::all().len());

        let mut etags: Vec<String> = GeoidModel::all()
            .iter()
            .map(|g| {
                compute_etag(
                    &vertical_etag_keys(*g, HeightMode::Ellipsoidal),
                    "terrarium-xyz",
                    TileFormat::Png,
                    10,
                    909,
                    403,
                )
            })
            .collect();
        etags.sort();
        etags.dedup();
        assert_eq!(etags.len(), GeoidModel::all().len());
    }

    #[test]
    fn cache_key_carries_model_and_mode_segments() {
        let key = key_for(GeoidModel::Jpgeo2024, HeightMode::GeoidOnly);
        assert!(key.contains("/jpgeo2024/geoid/"), "{key}");
    }
}
