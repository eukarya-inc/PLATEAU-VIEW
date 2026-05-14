//! In-process LRU cache wrapping any `DemProvider`.
//!
//! The terrain endpoint resamples each Cesium tile from one or more
//! Web-Mercator XYZ DEM tiles. With the recent walk-up-on-404 fallback in
//! [`super::CompositeDemProvider`], every quantized-mesh request at a
//! zoom where the upstream over-promises coverage (Mapterhorn 404s past
//! z=14 in parts of Japan) decodes the parent z=14 tile *again* from
//! scratch — HTTP fetch + WebP/PNG decode + bilinear resample to 512×512.
//! That's the dominant memory-and-CPU cost per request, and Cloud Run
//! was OOM-killing instances at ~1.2 GB RSS under modest concurrency.
//!
//! Wrapping the base provider in this LRU lets all the children at higher
//! zooms share one decoded copy of the parent tile. moka's `future::Cache`
//! does single-flight de-duplication (`get_with`), so even a stampede of
//! concurrent requests for the same parent triggers exactly one upstream
//! fetch.

use std::sync::Arc;

use async_trait::async_trait;
use moka::future::Cache;

use super::dem::{DemError, DemProvider, DemTile, GeoBounds};

/// Memoised wrapper around any `DemProvider`. Cache keys include
/// `tile_size` because callers may legitimately request multiple sizes
/// (raster terrarium output vs. mesh sampling), and the elevation grid
/// is resolution-dependent.
pub struct CachedDemProvider {
    inner: Arc<dyn DemProvider>,
    cache: Cache<(u8, u32, u32, u32), Arc<DemTile>>,
}

impl CachedDemProvider {
    /// `max_entries` caps the LRU size in tile count, not bytes. Each
    /// 512×512 f64 tile is ~2 MiB — sizing for ~200 entries gives a
    /// 400 MiB ceiling that fits comfortably under a 2 GiB Cloud Run
    /// limit alongside the existing terrain output cache.
    pub fn new(inner: Arc<dyn DemProvider>, max_entries: u64) -> Self {
        Self {
            inner,
            cache: Cache::builder().max_capacity(max_entries).build(),
        }
    }
}

#[async_trait]
impl DemProvider for CachedDemProvider {
    async fn get_tile_elevations(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError> {
        let key = (z, x, y, tile_size);
        if let Some(cached) = self.cache.get(&key).await {
            return Ok((*cached).clone());
        }
        // Single-flight via moka's `try_get_with`: concurrent misses for
        // the same key fan into one upstream fetch. Errors are not
        // retained — `try_get_with` only caches on success, so a
        // transient upstream failure won't be pinned for the LRU's
        // lifetime. `DemError` is `Clone`, so we can propagate the
        // original variant through `Arc<E>` without losing structure
        // (`NotFound`, `Http`, `Decode`, …).
        let arc = self
            .cache
            .try_get_with(key, async {
                let tile = self.inner.get_tile_elevations(z, x, y, tile_size).await?;
                Ok::<Arc<DemTile>, DemError>(Arc::new(tile))
            })
            .await
            .map_err(|e| (*e).clone())?;
        Ok((*arc).clone())
    }

    fn native_tile_size(&self) -> u32 {
        self.inner.native_tile_size()
    }

    fn max_zoom(&self) -> u8 {
        self.inner.max_zoom()
    }

    fn version(&self) -> &str {
        self.inner.version()
    }

    fn slug(&self) -> &str {
        self.inner.slug()
    }

    async fn preload(&self) -> Result<(), DemError> {
        self.inner.preload().await
    }

    fn bounds(&self) -> Option<GeoBounds> {
        self.inner.bounds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    /// DEM provider that counts upstream calls and sleeps to widen the
    /// window during which concurrent callers can pile onto the same key.
    struct CountingProvider {
        calls: AtomicUsize,
        delay: Duration,
    }

    #[async_trait]
    impl DemProvider for CountingProvider {
        async fn get_tile_elevations(
            &self,
            _z: u8,
            _x: u32,
            _y: u32,
            tile_size: u32,
        ) -> Result<DemTile, DemError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            Ok(DemTile {
                elevations: vec![0.0; (tile_size * tile_size) as usize],
                etag: None,
            })
        }

        fn native_tile_size(&self) -> u32 {
            256
        }
        fn max_zoom(&self) -> u8 {
            14
        }
        fn version(&self) -> &str {
            "test"
        }
        fn slug(&self) -> &str {
            "test"
        }
    }

    /// Regression: a stampede of concurrent callers for the same tile must
    /// trigger exactly one upstream fetch. Pre-single-flight this would
    /// race upstream N times.
    #[tokio::test]
    async fn test_single_flight_coalesces_concurrent_misses() {
        let inner = Arc::new(CountingProvider {
            calls: AtomicUsize::new(0),
            delay: Duration::from_millis(50),
        });
        let provider = Arc::new(CachedDemProvider::new(inner.clone(), 32));

        let mut handles = Vec::new();
        for _ in 0..16 {
            let p = provider.clone();
            handles.push(tokio::spawn(async move {
                p.get_tile_elevations(5, 1, 2, 256).await.unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(
            inner.calls.load(Ordering::SeqCst),
            1,
            "concurrent misses must coalesce into a single upstream fetch"
        );
    }
}
