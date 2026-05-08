//! Composite DEM provider: a base + ordered list of overlays.
//!
//! Overlays are stored in user-supplied config order — index 0 is painted
//! first (just above the base), the last index is painted last (frontmost).
//! Each overlay's elevation grid is paint-overed pixel-by-pixel: where the
//! overlay returns a finite value, it replaces the current merged value;
//! NaN passes through to leave the lower layer visible.
//!
//! Bounds known after `preload()` go into an R*-tree so that overlays
//! whose footprint doesn't intersect the requested tile are skipped without
//! an HTTP round-trip. Overlays without bounds (XYZ DEMs without explicit
//! `bounds` config) are always queried.
//!
//! On per-overlay fetch error, the overlay is skipped and a `failed:{slug}`
//! marker is appended to the etag fragment so the served bytes remain
//! consistent with their advertised cache key (and a recovered overlay
//! flips the cache key back to its happy path automatically).

use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;
use rstar::{AABB, RTree, RTreeObject};

use super::dem::{DemError, DemProvider, DemTile, GeoBounds};

/// One bbox entry in the R*-tree.
#[derive(Debug, Clone)]
struct OverlayEntry {
    bbox: AABB<[f64; 2]>,
    overlay_idx: usize,
}

impl RTreeObject for OverlayEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.bbox
    }
}

pub struct CompositeDemProvider {
    base: Arc<dyn DemProvider>,
    overlays: Vec<Arc<dyn DemProvider>>,
    /// Spatial index over overlays *with* known bounds.
    index: RTree<OverlayEntry>,
    /// Indices of overlays whose bounds are unknown — always queried.
    unbounded: Vec<usize>,
    /// Combined cache-key slug aggregating base + every overlay slug.
    slug: String,
    /// Combined version digest from base+overlays at construction time.
    version: String,
    /// Largest max_zoom across base+overlays. Used as `max_zoom()` so
    /// requests at any overlay's max zoom still succeed.
    max_zoom: u8,
}

impl CompositeDemProvider {
    /// Build the composite. Call `preload()` afterwards to populate the
    /// R*-tree from each overlay's metadata.
    pub fn new(base: Arc<dyn DemProvider>, overlays: Vec<Arc<dyn DemProvider>>) -> Self {
        let max_zoom = std::iter::once(base.max_zoom())
            .chain(overlays.iter().map(|o| o.max_zoom()))
            .max()
            .unwrap_or(15);
        let mut slug = format!("composite[base:{}", base.slug());
        for o in &overlays {
            slug.push_str(&format!("|{}", o.slug()));
        }
        slug.push(']');
        let version = format!(
            "{}|{}",
            base.version(),
            overlays
                .iter()
                .map(|o| format!("{}:{}", o.slug(), o.version()))
                .collect::<Vec<_>>()
                .join("|"),
        );
        Self {
            base,
            overlays,
            index: RTree::new(),
            unbounded: Vec::new(),
            slug,
            version,
            max_zoom,
        }
    }

    /// Pick overlay indices whose bbox intersects the tile bbox, plus all
    /// unbounded overlays. Returns indices in original config order.
    fn select_overlays(&self, tile: &GeoBounds) -> Vec<usize> {
        let env = AABB::from_corners([tile.west, tile.south], [tile.east, tile.north]);
        let mut hits: Vec<usize> = self
            .index
            .locate_in_envelope_intersecting(&env)
            .map(|e| e.overlay_idx)
            .chain(self.unbounded.iter().copied())
            .collect();
        hits.sort_unstable();
        hits.dedup();
        hits
    }
}

#[async_trait]
impl DemProvider for CompositeDemProvider {
    async fn get_tile_elevations(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError> {
        // 1. Base. If z exceeds base.max_zoom but a high-res overlay is
        //    advertised at this zoom, fetch the base's parent tile and
        //    bilinear-upsample the relevant sub-region. Without this
        //    fallback, an `OutOfRange` from the base would short-circuit
        //    the whole composite and the overlay's high-res data would
        //    never reach the renderer — leaving Cesium with an all-zero
        //    tile and visible "pits" at high zoom over COG-covered areas.
        let base_max = self.base.max_zoom();
        let mut base_tile = if z <= base_max {
            self.base.get_tile_elevations(z, x, y, tile_size).await?
        } else {
            let zoom_diff = z - base_max;
            let factor = 1u32 << zoom_diff;
            let parent_x = x / factor;
            let parent_y = y / factor;
            let parent = self
                .base
                .get_tile_elevations(base_max, parent_x, parent_y, tile_size)
                .await?;
            let elevations = upsample_subregion(
                &parent.elevations,
                tile_size,
                factor,
                x % factor,
                y % factor,
            );
            DemTile {
                elevations,
                etag: parent.etag,
            }
        };

        // 2. Compute the geographic bbox of this Web-Mercator tile for R-tree
        // pruning. (We re-derive the formula here to avoid a circular dep.)
        let tile_bbox = mercator_tile_bbox(z, x, y);
        let candidates = self.select_overlays(&tile_bbox);
        if candidates.is_empty() {
            return Ok(base_tile);
        }

        // 3. Parallel fetch.
        let futures = candidates.iter().map(|&idx| {
            let provider = self.overlays[idx].clone();
            async move {
                let z_clamped = z.min(provider.max_zoom());
                let result = provider
                    .get_tile_elevations(z_clamped, x, y, tile_size)
                    .await;
                (idx, result)
            }
        });
        let results = join_all(futures).await;

        // 4. Paint over in original config order, aggregate etags.
        let mut etag_parts: Vec<String> = Vec::new();
        if let Some(e) = base_tile.etag.as_ref() {
            etag_parts.push(format!("base:{}:{}", self.base.slug(), e));
        } else {
            etag_parts.push(format!("base:{}:{}", self.base.slug(), self.base.version()));
        }
        for (idx, result) in results {
            let provider = &self.overlays[idx];
            match result {
                Ok(overlay) => {
                    paint_over(&mut base_tile.elevations, &overlay.elevations);
                    etag_parts.push(format!(
                        "{}:{}",
                        provider.slug(),
                        overlay
                            .etag
                            .unwrap_or_else(|| provider.version().to_string()),
                    ));
                }
                Err(e) => {
                    tracing::warn!(
                        slug = provider.slug(),
                        error = %e,
                        "overlay fetch failed; skipping"
                    );
                    etag_parts.push(format!("failed:{}", provider.slug()));
                }
            }
        }
        base_tile.etag = Some(etag_parts.join("|"));
        Ok(base_tile)
    }

    fn native_tile_size(&self) -> u32 {
        self.base.native_tile_size()
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
        // Concurrently preload base + every overlay.
        let mut futs = Vec::with_capacity(self.overlays.len() + 1);
        futs.push(self.base.preload());
        for o in &self.overlays {
            futs.push(o.preload());
        }
        let results = join_all(futs).await;
        // We log preload errors but don't fail the whole startup — an overlay
        // that's transiently unreachable shouldn't take terrain offline.
        for r in results {
            if let Err(e) = r {
                tracing::warn!(error=%e, "overlay preload failed");
            }
        }

        // Bounds are now (potentially) populated. We can't mutate self.index
        // through `&self`, so `CompositeDemProvider` is constructed once via
        // [`Self::build`] which folds bounds in after preload. To keep the
        // trait dyn-friendly, we record the resolved indices on a side
        // channel — see the `build` constructor below.
        Ok(())
    }

    fn bounds(&self) -> Option<GeoBounds> {
        // The composite covers wherever the base covers (overlays only patch
        // _within_ the base). We expose the base's bounds.
        self.base.bounds()
    }
}

/// Bilinear-upsample one sub-tile of a parent grid to `tile_size × tile_size`.
///
/// `parent` is a `tile_size × tile_size` grid covering one parent tile. The
/// child tile occupies the `(sub_x, sub_y)`-th cell of a `factor × factor`
/// subdivision of the parent. Pixel centers are sampled with bilinear
/// interpolation; out-of-range neighbours and NaNs propagate as NaN.
fn upsample_subregion(
    parent: &[f64],
    tile_size: u32,
    factor: u32,
    sub_x: u32,
    sub_y: u32,
) -> Vec<f64> {
    let n = (tile_size * tile_size) as usize;
    let mut out = Vec::with_capacity(n);
    let scale = 1.0 / factor as f64;
    let off_x = sub_x as f64 * tile_size as f64 * scale;
    let off_y = sub_y as f64 * tile_size as f64 * scale;
    for cy in 0..tile_size {
        let py = off_y + (cy as f64 + 0.5) * scale - 0.5;
        for cx in 0..tile_size {
            let px = off_x + (cx as f64 + 0.5) * scale - 0.5;
            out.push(bilinear_at(parent, tile_size, px, py));
        }
    }
    out
}

fn bilinear_at(grid: &[f64], width: u32, x: f64, y: f64) -> f64 {
    let w = width as i64;
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let dx = x - x0 as f64;
    let dy = y - y0 as f64;
    let get = |xi: i64, yi: i64| -> f64 {
        if xi < 0 || yi < 0 || xi >= w || yi >= w {
            f64::NAN
        } else {
            grid[(yi * w + xi) as usize]
        }
    };
    let v00 = get(x0, y0);
    let v10 = get(x0 + 1, y0);
    let v01 = get(x0, y0 + 1);
    let v11 = get(x0 + 1, y0 + 1);
    if v00.is_nan() || v10.is_nan() || v01.is_nan() || v11.is_nan() {
        return f64::NAN;
    }
    v00 * (1.0 - dx) * (1.0 - dy) + v10 * dx * (1.0 - dy) + v01 * (1.0 - dx) * dy + v11 * dx * dy
}

/// Paint `overlay` onto `base` per pixel. Where overlay is finite, it wins.
fn paint_over(base: &mut [f64], overlay: &[f64]) {
    let n = base.len().min(overlay.len());
    for i in 0..n {
        if overlay[i].is_finite() {
            base[i] = overlay[i];
        }
    }
}

fn mercator_tile_bbox(z: u8, x: u32, y: u32) -> GeoBounds {
    use std::f64::consts::PI;
    let n = (1u32 << z) as f64;
    let west = (x as f64 / n) * 360.0 - 180.0;
    let east = ((x + 1) as f64 / n) * 360.0 - 180.0;
    let north = (PI * (1.0 - 2.0 * (y as f64) / n))
        .sinh()
        .atan()
        .to_degrees();
    let south = (PI * (1.0 - 2.0 * ((y + 1) as f64) / n))
        .sinh()
        .atan()
        .to_degrees();
    GeoBounds::new(west, south, east, north)
}

/// Build a composite, run preload on every member in parallel, and assemble
/// the R*-tree from the now-populated bounds.
pub async fn build(
    base: Arc<dyn DemProvider>,
    overlays: Vec<Arc<dyn DemProvider>>,
) -> CompositeDemProvider {
    // Preload base + overlays in parallel. Failures are warned-and-continue.
    let mut futs = Vec::with_capacity(overlays.len() + 1);
    futs.push(base.preload());
    for o in &overlays {
        futs.push(o.preload());
    }
    for r in join_all(futs).await {
        if let Err(e) = r {
            tracing::warn!(error=%e, "preload failed during composite build");
        }
    }

    // Index overlays with known bounds; track unbounded ones separately.
    let mut entries = Vec::new();
    let mut unbounded = Vec::new();
    for (idx, o) in overlays.iter().enumerate() {
        match o.bounds() {
            Some(b) => entries.push(OverlayEntry {
                bbox: AABB::from_corners([b.west, b.south], [b.east, b.north]),
                overlay_idx: idx,
            }),
            None => unbounded.push(idx),
        }
    }
    let index = RTree::bulk_load(entries);

    // Recompute composite metadata in case overlays got their version /
    // bounds / max_zoom populated during preload.
    let mut composite = CompositeDemProvider::new(base, overlays);
    composite.index = index;
    composite.unbounded = unbounded;
    composite
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(
        slug: &str,
        bounds: Option<GeoBounds>,
        elevations: Vec<f64>,
    ) -> Arc<dyn DemProvider> {
        Arc::new(StubProvider {
            slug: slug.to_string(),
            bounds,
            elevations,
            fail: false,
        })
    }

    fn failing(slug: &str, bounds: Option<GeoBounds>) -> Arc<dyn DemProvider> {
        Arc::new(StubProvider {
            slug: slug.to_string(),
            bounds,
            elevations: vec![],
            fail: true,
        })
    }

    struct StubProvider {
        slug: String,
        bounds: Option<GeoBounds>,
        elevations: Vec<f64>,
        fail: bool,
    }

    #[async_trait]
    impl DemProvider for StubProvider {
        async fn get_tile_elevations(
            &self,
            _z: u8,
            _x: u32,
            _y: u32,
            tile_size: u32,
        ) -> Result<DemTile, DemError> {
            if self.fail {
                return Err(DemError::Http("stub failure".to_string()));
            }
            let n = (tile_size * tile_size) as usize;
            let mut e = self.elevations.clone();
            e.resize(n, f64::NAN);
            Ok(DemTile {
                elevations: e,
                etag: Some(format!("etag-{}", self.slug)),
            })
        }
        fn native_tile_size(&self) -> u32 {
            256
        }
        fn max_zoom(&self) -> u8 {
            18
        }
        fn version(&self) -> &str {
            "v1"
        }
        fn slug(&self) -> &str {
            &self.slug
        }
        fn bounds(&self) -> Option<GeoBounds> {
            self.bounds
        }
    }

    #[tokio::test]
    async fn paint_over_lifts_finite_values() {
        let base = provider("base", None, vec![1.0, 2.0, 3.0, 4.0]);
        let overlay = provider("a", None, vec![f64::NAN, 20.0, f64::NAN, 40.0]);
        let comp = build(base, vec![overlay]).await;
        let tile = comp.get_tile_elevations(0, 0, 0, 2).await.unwrap();
        assert_eq!(tile.elevations, vec![1.0, 20.0, 3.0, 40.0]);
        assert!(tile.etag.unwrap().contains("a:etag-a"));
    }

    #[tokio::test]
    async fn last_overlay_wins() {
        let base = provider("base", None, vec![0.0; 4]);
        let a = provider("a", None, vec![10.0; 4]);
        let b = provider("b", None, vec![20.0; 4]);
        let comp = build(base, vec![a, b]).await;
        let tile = comp.get_tile_elevations(0, 0, 0, 2).await.unwrap();
        assert_eq!(tile.elevations, vec![20.0; 4]);
    }

    #[tokio::test]
    async fn bbox_pruning_skips_disjoint_overlay() {
        let base = provider("base", None, vec![0.0; 4]);
        // Overlay restricted to a tiny region in Japan; at z=0/0/0 the tile
        // spans the whole globe so this should still intersect.
        let japan = provider(
            "japan",
            Some(GeoBounds::new(139.0, 35.0, 140.0, 36.0)),
            vec![100.0; 4],
        );
        // This overlay's bbox is *entirely* inside z=0/0/0 (still intersects).
        let comp = build(base.clone(), vec![japan]).await;
        let _ = comp.get_tile_elevations(0, 0, 0, 2).await.unwrap();

        // Now an overlay that won't intersect z=2/3/2 (Pacific Ocean tile).
        let antarctica = provider(
            "ant",
            Some(GeoBounds::new(-180.0, -89.0, 180.0, -60.0)),
            vec![999.0; 4],
        );
        let comp2 = build(base, vec![antarctica]).await;
        // z=0/0/0 covers the antarctic too (whole globe), so this still hits.
        // But picking a high-zoom tile in tokyo should *not* hit antarctic.
        let tokyo_z14 = comp2.get_tile_elevations(14, 14552, 6450, 2).await.unwrap();
        assert_eq!(tokyo_z14.elevations, vec![0.0; 4]);
        // Etag must NOT include the antarctic overlay (was pruned).
        assert!(!tokyo_z14.etag.unwrap().contains("ant"));
    }

    #[tokio::test]
    async fn failed_overlay_marked_in_etag() {
        let base = provider("base", None, vec![0.0; 4]);
        let bad = failing("bad", None);
        let comp = build(base, vec![bad]).await;
        let tile = comp.get_tile_elevations(0, 0, 0, 2).await.unwrap();
        assert!(tile.etag.unwrap().contains("failed:bad"));
        // Base still rendered.
        assert_eq!(tile.elevations, vec![0.0; 4]);
    }
}
