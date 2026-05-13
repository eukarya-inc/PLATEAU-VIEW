//! COG DEM source.
//!
//! Wraps `crate::cog::CogReader` to expose a GeoTIFF / Cloud-Optimized GeoTIFF
//! containing a single elevation band as a [`DemProvider`]. The COG header is
//! read once on `preload()` to populate `bounds()` and choose the best IFD
//! per tile request.

use std::f64::consts::PI;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use object_store::{
    GetOptions, ObjectStore, client::ClientConfigKey, http::HttpBuilder, path::Path as ObjectPath,
};
use tokio::sync::{Mutex, OnceCell, RwLock};
use url::Url;
use xxhash_rust::xxh64::xxh64;

use super::dem::{DemError, DemProvider, DemTile, GeoBounds};
use crate::cog::{CogReader, TileBounds};

/// How long to wait before retrying a failed `upstream_etag` HEAD. A transient
/// network blip during `preload()` shouldn't poison the per-tile etag for the
/// pod's lifetime — when the cached value is still `None`, subsequent tile
/// requests retry past this cooldown until HEAD succeeds. The cooldown keeps
/// us from hammering a genuinely broken upstream on every tile.
const UPSTREAM_ETAG_RETRY_INTERVAL: Duration = Duration::from_secs(60);

/// Cached upstream-ETag state. `value` is the parsed HTTP ETag (when HEAD
/// succeeded); `last_attempt` is the last HEAD wall-clock time regardless of
/// outcome, used to throttle retries.
struct UpstreamEtagState {
    value: Option<String>,
    last_attempt: Option<Instant>,
}

pub struct CogDemSource {
    url: String,
    /// xxh64 of the COG URL, computed once at construction. Mixed into the
    /// per-tile etag when [`upstream_etag`] is `None` so a HEAD-fetch failure
    /// doesn't make two different URLs collapse to the same cache key. Hex
    /// pre-rendered to avoid re-formatting on every tile request.
    url_hash_hex: String,
    nodata: Option<f64>,
    version: String,
    slug: String,
    max_zoom: u8,
    native_tile_size: u32,
    reader: OnceCell<Arc<CogReader>>,
    bounds_cell: OnceCell<Option<GeoBounds>>,
    /// Upstream object ETag, lazily fetched via HEAD on first use (or in
    /// `preload()`). Mixed into the per-tile etag so a CMS-side COG swap
    /// (same URL pattern, new content) busts downstream tile caches without
    /// requiring a manual version bump in config. Wrapped in `RwLock` rather
    /// than `OnceCell` so a failed HEAD can be retried later — see
    /// [`UPSTREAM_ETAG_RETRY_INTERVAL`].
    upstream_etag: RwLock<UpstreamEtagState>,
    /// Single-flight gate around the actual HEAD request. Concurrent tile
    /// requests that find `upstream_etag.value == None` AND the retry
    /// cooldown has elapsed `try_lock` this; only the winner does the HEAD,
    /// losers return the current value (probably still `None`) immediately
    /// and skip until the winner updates the cell.
    upstream_etag_retry: Mutex<()>,
}

impl CogDemSource {
    pub fn new(
        slug: impl Into<String>,
        url: impl Into<String>,
        nodata: Option<f64>,
        version: impl Into<String>,
        max_zoom: u8,
        native_tile_size: u32,
    ) -> Self {
        let url = url.into();
        let url_hash_hex = format!("{:x}", xxh64(url.as_bytes(), 0));
        Self {
            url,
            url_hash_hex,
            nodata,
            version: version.into(),
            slug: slug.into(),
            max_zoom,
            native_tile_size,
            reader: OnceCell::new(),
            bounds_cell: OnceCell::new(),
            upstream_etag: RwLock::new(UpstreamEtagState {
                value: None,
                last_attempt: None,
            }),
            upstream_etag_retry: Mutex::new(()),
        }
    }

    /// Resolve the upstream object ETag, with retry on failure.
    ///
    /// Hot path (HEAD already succeeded): single read-lock, return cached
    /// value. Cooldown path (HEAD failed recently): single read-lock, return
    /// `None`. Retry path (cooldown elapsed): `try_lock` the retry gate so a
    /// single concurrent task does the HEAD; losers return the current value
    /// without blocking.
    async fn upstream_etag(&self) -> Option<String> {
        {
            let state = self.upstream_etag.read().await;
            if state.value.is_some() {
                return state.value.clone();
            }
            if state
                .last_attempt
                .is_some_and(|t| t.elapsed() < UPSTREAM_ETAG_RETRY_INTERVAL)
            {
                return None;
            }
        }
        // Claim the retry slot — non-blocking. Losing the race means another
        // task is already doing the HEAD; we return whatever's cached now
        // rather than queueing behind it.
        let _retry = match self.upstream_etag_retry.try_lock() {
            Ok(g) => g,
            Err(_) => return self.upstream_etag.read().await.value.clone(),
        };
        // Re-check under the retry lock: the winner of a previous race may
        // have updated the state between our first read and the `try_lock`.
        {
            let state = self.upstream_etag.read().await;
            if state.value.is_some() {
                return state.value.clone();
            }
            if state
                .last_attempt
                .is_some_and(|t| t.elapsed() < UPSTREAM_ETAG_RETRY_INTERVAL)
            {
                return None;
            }
        }
        let fetched = fetch_upstream_etag(&self.url).await;
        let mut state = self.upstream_etag.write().await;
        state.value = fetched.clone();
        state.last_attempt = Some(Instant::now());
        fetched
    }

    async fn reader(&self) -> Result<Arc<CogReader>, DemError> {
        self.reader
            .get_or_try_init(|| async {
                let (store, path) = build_object_store(&self.url)?;
                CogReader::open(store, path)
                    .await
                    .map(Arc::new)
                    .map_err(|e| DemError::Http(format!("cog open: {e}")))
            })
            .await
            .cloned()
    }
}

#[async_trait]
impl DemProvider for CogDemSource {
    async fn get_tile_elevations(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError> {
        if z > self.max_zoom {
            return Err(DemError::OutOfRange);
        }
        let reader = self.reader().await?;
        let bounds = mercator_xyz_to_bounds(z, x, y);

        // Short-circuit when the COG bounds don't overlap this tile.
        if let Some(cog_bounds) = reader.bounds() {
            let g = GeoBounds::new(
                cog_bounds.west,
                cog_bounds.south,
                cog_bounds.east,
                cog_bounds.north,
            );
            let req = GeoBounds::new(bounds.west, bounds.south, bounds.east, bounds.north);
            if !g.intersects(&req) {
                return Ok(DemTile {
                    elevations: vec![f64::NAN; (tile_size * tile_size) as usize],
                    etag: None,
                });
            }
        }

        // Prefer the explicit `nodata` from config, but fall back to the
        // COG's own `GDAL_NODATA` tag. Without this fallback, sentinel pixels
        // (e.g. -9999 from gdal_translate) leak through as real elevations,
        // creating Cesium-visible "pits" along COG edges and outside the
        // valid raster footprint.
        let nodata_value = self.nodata.or_else(|| reader.nodata_from_metadata());
        let elevations = reader
            .read_tile_elevation(&bounds, tile_size, nodata_value)
            .await
            .map_err(|e| DemError::Decode(format!("cog read: {e}")))?;

        // Mix the upstream ETag (when available) into the per-tile etag so
        // any change to the underlying COG file flips downstream cache keys.
        // When HEAD fails we fall back to a URL-hash-derived marker rather
        // than just `version`: two different URLs share the same `version`
        // constant, so a bare-version fallback would let a stale HEAD cell
        // serve tiles from the wrong asset URL via the cache. Hashing the
        // URL keeps swapped assets at distinct cache keys even when HEAD
        // can't reach the upstream.
        let etag = match self.upstream_etag().await {
            Some(e) => Some(format!("{}:{e}", self.version)),
            None => Some(format!("{}:no-etag:{}", self.version, self.url_hash_hex)),
        };
        Ok(DemTile { elevations, etag })
    }

    fn native_tile_size(&self) -> u32 {
        self.native_tile_size
    }
    fn max_zoom(&self) -> u8 {
        self.max_zoom
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn slug(&self) -> &str {
        &self.slug
    }

    async fn preload(&self) -> Result<(), DemError> {
        let reader = self.reader().await?;
        if let Some(b) = reader.bounds() {
            let _ = self
                .bounds_cell
                .set(Some(GeoBounds::new(b.west, b.south, b.east, b.north)));
        }
        // Warm the upstream ETag cell so the first tile request doesn't
        // pay the HEAD round-trip.
        let _ = self.upstream_etag().await;
        Ok(())
    }

    fn bounds(&self) -> Option<GeoBounds> {
        self.bounds_cell.get().and_then(|b| *b)
    }
}

/// One-shot HEAD against the upstream COG URL. Returns the parsed ETag with
/// any `W/` weak-prefix and surrounding quotes stripped, or `None` if the
/// backend doesn't return one / the request failed. Failures are logged.
async fn fetch_upstream_etag(url: &str) -> Option<String> {
    let (store, path) = match build_object_store(url) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error=%e, "cog HEAD: build_object_store failed");
            return None;
        }
    };
    let opts = GetOptions {
        head: true,
        ..Default::default()
    };
    match store.get_opts(&path, opts).await {
        Ok(result) => result
            .meta
            .e_tag
            .as_ref()
            .map(|s: &String| s.trim_start_matches("W/").trim_matches('"').to_string())
            .filter(|s| !s.is_empty()),
        Err(e) => {
            tracing::warn!(error=%e, "cog HEAD failed; ETag unavailable");
            None
        }
    }
}

/// Web-Mercator XYZ → geographic bounds (the same formula used by `cog::TileBounds::from_xyz`).
fn mercator_xyz_to_bounds(z: u8, x: u32, y: u32) -> TileBounds {
    let n = (1u32 << z) as f64;
    let west = (x as f64 / n) * 360.0 - 180.0;
    let east = ((x + 1) as f64 / n) * 360.0 - 180.0;
    let north = lat_from_y(y as f64, n);
    let south = lat_from_y((y + 1) as f64, n);
    TileBounds {
        west,
        south,
        east,
        north,
    }
}

fn lat_from_y(y: f64, n: f64) -> f64 {
    (PI * (1.0 - 2.0 * y / n)).sinh().atan().to_degrees()
}

/// Build an object_store v0.12 backend from a URL. Mirrors the logic in
/// `tile::cog::CogTileSource::create_object_store` but is local to keep the
/// terrain module self-contained.
fn build_object_store(url: &str) -> Result<(Arc<dyn ObjectStore>, ObjectPath), DemError> {
    let parsed = Url::parse(url).map_err(|e| DemError::Decode(format!("invalid cog URL: {e}")))?;

    match parsed.scheme() {
        "http" | "https" => {
            let base = format!(
                "{}://{}{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or(""),
                parsed.port().map(|p| format!(":{p}")).unwrap_or_default(),
            );
            let store = HttpBuilder::new()
                .with_url(&base)
                .with_config(ClientConfigKey::AllowHttp, "true")
                .build()
                .map_err(|e| DemError::Http(format!("HTTP store init: {e}")))?;
            Ok((Arc::new(store), ObjectPath::from(parsed.path())))
        }
        "gs" => {
            let bucket = parsed
                .host_str()
                .ok_or_else(|| DemError::Decode("gs:// URL missing bucket".to_string()))?;
            let store = object_store::gcp::GoogleCloudStorageBuilder::new()
                .with_bucket_name(bucket)
                .build()
                .map_err(|e| DemError::Http(format!("GCS store init: {e}")))?;
            Ok((Arc::new(store), ObjectPath::from(parsed.path())))
        }
        "s3" => {
            let bucket = parsed
                .host_str()
                .ok_or_else(|| DemError::Decode("s3:// URL missing bucket".to_string()))?;
            let store = object_store::aws::AmazonS3Builder::from_env()
                .with_bucket_name(bucket)
                .build()
                .map_err(|e| DemError::Http(format!("S3 store init: {e}")))?;
            Ok((Arc::new(store), ObjectPath::from(parsed.path())))
        }
        other => Err(DemError::Decode(format!(
            "unsupported cog URL scheme: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xyz_to_bounds_z0() {
        let b = mercator_xyz_to_bounds(0, 0, 0);
        assert!((b.west + 180.0).abs() < 1e-9);
        assert!((b.east - 180.0).abs() < 1e-9);
    }
}
