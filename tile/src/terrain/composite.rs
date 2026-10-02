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

use super::dem::{DemError, DemProvider, DemTile, GeoBounds, PixelPositions, Upsampled};
use super::upsample_subregion;
use super::vertical::{DhMemo, SourceCorrection};

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
    /// Height correction for members whose vertical datum differs from the
    /// source's target datum (see [`super::vertical`]). `None` — every source
    /// that declares no datums — leaves the composite exactly as before.
    correction: Option<Arc<SourceCorrection>>,
}

impl CompositeDemProvider {
    /// Build the composite. Call `preload()` afterwards to populate the
    /// R*-tree from each overlay's metadata.
    pub fn new(base: Arc<dyn DemProvider>, overlays: Vec<Arc<dyn DemProvider>>) -> Self {
        Self::new_with_correction(base, overlays, None)
    }

    /// Like [`Self::new`], with a height correction. The correction's
    /// descriptor (product id + version, missing-ΔH policy, corrected
    /// members) is appended to `version`, which feeds every terrain cache key
    /// and ETag; without a correction `version` is unchanged.
    pub fn new_with_correction(
        base: Arc<dyn DemProvider>,
        overlays: Vec<Arc<dyn DemProvider>>,
        correction: Option<Arc<SourceCorrection>>,
    ) -> Self {
        if let Some(c) = &correction {
            assert_eq!(
                c.layers.len(),
                overlays.len(),
                "height correction must flag every overlay"
            );
        }
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
        let version = match &correction {
            Some(c) => format!("{version}|{}", c.descriptor()),
            None => version,
        };
        Self {
            base,
            overlays,
            index: RTree::new(),
            unbounded: Vec::new(),
            slug,
            version,
            max_zoom,
            correction,
        }
    }

    /// Try to fetch the base at the requested zoom, falling back to a
    /// parent tile (with bilinear upsampling) when the base returns
    /// `OutOfRange` or `NotFound`. Walks down one zoom level at a time so
    /// partial-coverage upstreams (Mapterhorn over Japan tops out at
    /// different zooms in different areas) still feed the composite.
    async fn fetch_base_upsampled(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError> {
        let mut try_z = z.min(self.base.max_zoom());
        loop {
            let factor = 1u32 << (z - try_z);
            let parent_x = x / factor;
            let parent_y = y / factor;
            match self
                .base
                .get_tile_elevations(try_z, parent_x, parent_y, tile_size)
                .await
            {
                Ok(parent) => {
                    if factor == 1 {
                        return Ok(parent);
                    }
                    let elevations = upsample_subregion(
                        &parent.elevations,
                        tile_size,
                        factor,
                        x % factor,
                        y % factor,
                    );
                    // The parent comes straight from the base provider, so it
                    // is never itself an upsampled tile.
                    let parent_positions = parent
                        .positions
                        .unwrap_or_else(|| PixelPositions::centres(tile_size));
                    return Ok(DemTile {
                        elevations,
                        etag: parent.etag,
                        positions: Some(PixelPositions {
                            upsampled: Some(Upsampled {
                                factor,
                                sub_x: x % factor,
                                sub_y: y % factor,
                                tile_size,
                            }),
                            ..parent_positions
                        }),
                    });
                }
                Err(DemError::NotFound | DemError::OutOfRange) if try_z > 0 => {
                    try_z -= 1;
                }
                Err(e) => return Err(e),
            }
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
        // 1. Base, with parent-upsample fallback. We try the requested
        //    zoom first, then walk up parent tiles on `OutOfRange` (z
        //    exceeds advertised max_zoom) or `NotFound` (advertised
        //    max_zoom over-promises actual coverage — Mapterhorn for
        //    Japan claims z=15 but 404s on parts of Kisarazu, etc.) and
        //    bilinear-upsample the relevant sub-region. Without this,
        //    the COG overlays never reach the renderer at high zoom and
        //    Cesium ends up with all-zero tiles.
        let mut base_tile = self.fetch_base_upsampled(z, x, y, tile_size).await?;

        // Height correction happens per member, *before* painting, so the
        // composite ends up in one datum. Members sharing sample positions
        // share one ΔH evaluation via the memo.
        let mut dh_memo = DhMemo::default();
        if let Some(c) = &self.correction
            && c.base
        {
            c.apply(&mut base_tile, z, x, y, tile_size, &mut dh_memo);
        }

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
                (idx, z_clamped, result)
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
        for (idx, z_eval, result) in results {
            let provider = &self.overlays[idx];
            match result {
                Ok(mut overlay) => {
                    if let Some(c) = &self.correction
                        && c.layers[idx]
                    {
                        // ΔH at the points this member was evaluated at.
                        c.apply(&mut overlay, z_eval, x, y, tile_size, &mut dh_memo);
                    }
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
    build_with_correction(base, overlays, None).await
}

/// [`build`] with an optional height correction (one flag per overlay).
pub async fn build_with_correction(
    base: Arc<dyn DemProvider>,
    overlays: Vec<Arc<dyn DemProvider>>,
    correction: Option<Arc<SourceCorrection>>,
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
    let mut composite = CompositeDemProvider::new_with_correction(base, overlays, correction);
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
                positions: None,
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

    /// Every child of a parent, at every factor, must come out NaN-free from
    /// a NaN-free parent — including the rim pixels whose centres sit within
    /// half a parent pixel of the parent's border.
    #[test]
    fn upsample_subregion_has_no_nan_rim() {
        let size = 16u32;
        let parent: Vec<f64> = (0..size * size)
            .map(|i| (i / size) as f64 * 100.0 + (i % size) as f64)
            .collect();
        for factor in [2u32, 4, 8] {
            for sub_y in 0..factor {
                for sub_x in 0..factor {
                    let child = upsample_subregion(&parent, size, factor, sub_x, sub_y);
                    let nan = child.iter().filter(|v| v.is_nan()).count();
                    assert_eq!(
                        nan, 0,
                        "factor {factor} sub ({sub_x},{sub_y}): {nan} NaN px"
                    );
                }
            }
        }
        // The rim extends the edge sample: the top-left child's first pixel
        // is the parent's corner, the bottom-right child's last pixel the
        // opposite corner.
        let tl = upsample_subregion(&parent, size, 4, 0, 0);
        assert_eq!(tl[0], parent[0]);
        let br = upsample_subregion(&parent, size, 4, 3, 3);
        assert_eq!(br[br.len() - 1], parent[parent.len() - 1]);
    }

    /// Base that only serves zooms up to `max_zoom` (like Mapterhorn over
    /// open sea, which 404s above z6 around 145°E 30°N), so deeper requests
    /// go through `fetch_base_upsampled`.
    struct ShallowBase {
        max_zoom: u8,
    }

    #[async_trait]
    impl DemProvider for ShallowBase {
        async fn get_tile_elevations(
            &self,
            z: u8,
            _x: u32,
            _y: u32,
            tile_size: u32,
        ) -> Result<DemTile, DemError> {
            if z > self.max_zoom {
                return Err(DemError::NotFound);
            }
            Ok(DemTile {
                elevations: vec![0.0; (tile_size * tile_size) as usize],
                etag: None,
                positions: None,
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
            "shallow"
        }
    }

    /// Regression for the −32768 m stripes at sea: z8/231/105 is the
    /// right-hand child of its z6 parent, and its last two columns used to
    /// come back NaN.
    #[tokio::test]
    async fn base_parent_fallback_has_no_nan_edges() {
        let base: Arc<dyn DemProvider> = Arc::new(ShallowBase { max_zoom: 6 });
        let overlay = provider(
            "far",
            Some(GeoBounds::new(130.4, 32.8, 130.7, 33.0)),
            vec![],
        );
        let comp = build(base, vec![overlay]).await;
        for (x, y) in [(231, 105), (228, 104), (231, 107)] {
            let tile = comp.get_tile_elevations(8, x, y, 256).await.unwrap();
            let nan = tile.elevations.iter().filter(|v| v.is_nan()).count();
            assert_eq!(nan, 0, "z8/{x}/{y}: {nan} NaN px");
        }
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

    async fn v1_product() -> Arc<crate::terrain::vertical::DhProduct> {
        let url = url::Url::from_file_path(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/vertical/hyokorev-jgd2011-to-jgd2024/v1/manifest.json"),
        )
        .unwrap();
        crate::terrain::vertical::load_product(url.as_str())
            .await
            .unwrap()
    }

    /// Without a correction, the corrected-composite builder is the plain
    /// composite: same elevations, same etag, same version (= same cache keys).
    #[tokio::test]
    async fn no_correction_is_byte_identical() {
        let mk = || {
            (
                provider("base", None, vec![1.0, 2.0, 3.0, 4.0]),
                vec![
                    provider("a", None, vec![f64::NAN, 20.0, f64::NAN, 40.0]),
                    provider("b", None, vec![5.0, f64::NAN, f64::NAN, f64::NAN]),
                ],
            )
        };
        let (base, overlays) = mk();
        let plain = build(base, overlays).await;
        let (base, overlays) = mk();
        let none = build_with_correction(base, overlays, None).await;
        assert_eq!(plain.version(), none.version());
        assert_eq!(plain.slug(), none.slug());
        let (z, x, y) = (14, 14163, 6677);
        let a = plain.get_tile_elevations(z, x, y, 2).await.unwrap();
        let b = none.get_tile_elevations(z, x, y, 2).await.unwrap();
        assert_eq!(
            a.elevations.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            b.elevations.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(a.etag, b.etag);
    }

    /// ΔH goes onto the flagged members before painting; the base (here the
    /// datum-agnostic sea level) and unflagged members are left alone, and the
    /// correction is visible in `version`.
    #[tokio::test]
    async fn correction_applies_to_flagged_members_before_paint() {
        use crate::terrain::vertical::{BaseDatum, MissingDhPolicy, SourceCorrection};
        let product = v1_product().await;
        let n = 4u32;
        let len = (n * n) as usize;
        let base = provider("base", None, vec![0.0; len]);
        // `a` (jgd2011, corrected) covers the left half, `b` (already in the
        // target datum) the right half; `c` (corrected) paints over `b` in
        // the last column.
        let left: Vec<f64> = (0..len)
            .map(|k| if k % (n as usize) < 2 { 50.0 } else { f64::NAN })
            .collect();
        let right: Vec<f64> = (0..len)
            .map(|k| {
                if k % (n as usize) >= 2 {
                    70.0
                } else {
                    f64::NAN
                }
            })
            .collect();
        let last: Vec<f64> = (0..len)
            .map(|k| {
                if k % (n as usize) == 3 {
                    90.0
                } else {
                    f64::NAN
                }
            })
            .collect();
        let corr = Arc::new(SourceCorrection::new(
            product.clone(),
            MissingDhPolicy::Keep,
            false,
            BaseDatum::Agnostic,
            vec![true, false, true],
        ));
        let comp = build_with_correction(
            base,
            vec![
                provider("a", None, left),
                provider("b", None, right),
                provider("c", None, last),
            ],
            Some(corr.clone()),
        )
        .await;
        assert!(comp.version().ends_with(corr.descriptor()));
        assert!(corr.descriptor().contains("hyokorev-jgd2011-to-jgd2024@v1"));
        assert!(corr.descriptor().contains("missing=keep"));

        let (z, x, y) = (14u8, 14163u32, 6677u32);
        let tile = comp.get_tile_elevations(z, x, y, n).await.unwrap();
        let p = PixelPositions::centres(n);
        for j in 0..n {
            for i in 0..n {
                let (lon, lat) = p.lonlat(z, x, y, i, j);
                let dh = product.sample(lat, lon);
                assert!(dh.is_finite());
                let want = match i {
                    0 | 1 => 50.0 + dh,
                    2 => 70.0,
                    _ => 90.0 + dh,
                };
                assert_eq!(tile.elevations[(j * n + i) as usize], want, "({i},{j})");
            }
        }

        // Where the product has no ΔH (open sea), `nan` drops the corrected
        // member's pixel so the base shows through; `keep` serves it as-is.
        for (policy, want) in [(MissingDhPolicy::Keep, 50.0), (MissingDhPolicy::Nan, 0.0)] {
            let corr = Arc::new(SourceCorrection::new(
                product.clone(),
                policy,
                false,
                BaseDatum::Agnostic,
                vec![true],
            ));
            let comp = build_with_correction(
                provider("base", None, vec![0.0; len]),
                vec![provider("a", None, vec![50.0; len])],
                Some(corr),
            )
            .await;
            let tile = comp.get_tile_elevations(10, 924, 422, n).await.unwrap();
            assert!(tile.elevations.iter().all(|&v| v == want), "{policy:?}");
        }
    }
}
