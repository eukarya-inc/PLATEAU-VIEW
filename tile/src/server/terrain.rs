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
    DemProvider, Geoid, GeoidModel, MirrorSource,
    ellipsoid::{apply_geoid_to_grid, apply_geoid_to_grid_sized, apply_geoid_to_xyz_grid},
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
    query: Query<GeoidQuery>,
) -> Response {
    terrain_layer_json_impl(state, headers, query, None).await
}

/// GET /terrain/{name}/layer.json
pub async fn terrain_layer_json_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    query: Query<GeoidQuery>,
) -> Response {
    terrain_layer_json_impl(state, headers, query, Some(name)).await
}

async fn terrain_layer_json_impl(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<GeoidQuery>,
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
    q: GeoidQuery,
) -> Response {
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
    query: Query<GeoidQuery>,
) -> Response {
    terrain_tile_impl(state, headers, path, query, None).await
}

/// GET /terrain/{name}/{z}/{x}/{y}.terrain
pub async fn terrain_tile_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, z, x, y_ext)): Path<(String, u8, u32, String)>,
    query: Query<GeoidQuery>,
) -> Response {
    terrain_tile_impl(state, headers, Path((z, x, y_ext)), query, Some(name)).await
}

async fn terrain_tile_impl(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((z, x, y_ext)): Path<(u8, u32, String)>,
    Query(q): Query<GeoidQuery>,
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
    const TERRAIN_MESH_ALGO_VERSION: &str = "v3-terrain-codec";
    let upstream_etag_digest = digest(&fetch.source_etags.join("|"));
    let etag_keys: Vec<String> = vec![
        format!("dem-ver:{}", terrain.dem.version()),
        format!("dem-etag:{}", upstream_etag_digest),
        format!("geoid:{}", geoid_model.slug()),
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
                // Apply geoid (ortho → ellipsoid) to both grids.
                let g = Geoid::load(geoid_for_gen);
                apply_geoid_to_grid(&bounds_copy, &mut elevations, &g);
                apply_geoid_to_grid_sized(
                    &halo_bounds,
                    &mut elevations_with_halo,
                    &g,
                    halo_grid_size,
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
    query: Query<GeoidQuery>,
) -> Response {
    raster_tile(state, headers, path, query, RasterEncoding::Terrarium, None).await
}

/// GET /terrarium/{name}/{z}/{x}/{y}.{png|webp|avif}
pub async fn terrarium_tile_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, z, x, y_ext)): Path<(String, u8, u32, String)>,
    query: Query<GeoidQuery>,
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
    query: Query<GeoidQuery>,
) -> Response {
    raster_tile(state, headers, path, query, RasterEncoding::Mapbox, None).await
}

/// GET /mapbox/{name}/{z}/{x}/{y}.{png|webp|avif}
pub async fn mapbox_tile_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path((name, z, x, y_ext)): Path<(String, u8, u32, String)>,
    query: Query<GeoidQuery>,
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
    Query(q): Query<GeoidQuery>,
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

    let geoid_model = match resolve_geoid(&terrain, &q) {
        Ok(g) => g,
        Err(r) => return r,
    };
    let geoid = Geoid::load(geoid_model);

    // Cap `z` against the configured raster max zoom before any coordinate
    // math. Without this, `z >= dem_max + 9` makes `factor > tile_size`, so
    // the inner-loop index `off_y * tile_size` in extract_and_upsample walks
    // straight off the end of `parent` and the always-on bounds check panics
    // — a cheap, unauthenticated DoS on a public endpoint. Legitimate
    // MapLibre clients cap at `maxzoom` (`terrain.max_zoom`, default 18) and
    // never send anything past it, so returning 404 is the right answer.
    if z > terrain.max_zoom {
        return (StatusCode::NOT_FOUND, "Zoom out of range").into_response();
    }

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
    query: Query<GeoidQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Terrarium, None).await
}

/// GET /terrarium/{name}/tilejson.json
pub async fn terrarium_tilejson_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    query: Query<GeoidQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Terrarium, Some(name)).await
}

/// GET /mapbox/tilejson.json
pub async fn mapbox_tilejson(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    query: Query<GeoidQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Mapbox, None).await
}

/// GET /mapbox/{name}/tilejson.json
pub async fn mapbox_tilejson_named(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    query: Query<GeoidQuery>,
) -> Response {
    raster_tilejson(state, headers, query, RasterEncoding::Mapbox, Some(name)).await
}

async fn raster_tilejson(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<GeoidQuery>,
    encoding: RasterEncoding,
    name: Option<String>,
) -> Response {
    let terrain = match state.get_terrain(name.as_deref()).await {
        Some(t) => t,
        None => return (StatusCode::NOT_FOUND, "Unknown terrain source").into_response(),
    };
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
    // Embed the source name segment when this tilejson refers to a non-default
    // DEM source, so MapLibre's resolved tile URLs hit `/{slug}/{name}/...`.
    let name_segment = name.as_deref().map(|n| format!("/{n}")).unwrap_or_default();
    let tile_url = format!(
        "/{slug}{name_segment}/{{z}}/{{x}}/{{y}}.{fmt}?geoid={geoid}",
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
