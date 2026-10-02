//! DEM tile resampling: resizing a decoded tile to the served tile size, and
//! upsampling a parent tile above the DEM's max zoom.
//!
//! Malformed input (zero sizes, a grid shorter than its stated size, a zero
//! factor) yields an all-NaN result of the requested size — or an empty one
//! when that size itself overflows — instead of a panic. Valid input takes
//! exactly the arithmetic it always did.

/// Version of the DEM-tile resampling below ([`fit_to_tile_size`],
/// [`extract_and_upsample`]). Tiles that went through a resample carry it in
/// their per-tile ETag fragment, so changing the resampling re-keys exactly the
/// tiles it can change and leaves the rest alone — the DEM counterpart of
/// `cog::COG_SAMPLING_VERSION`. Bump it whenever resampled values change.
///
/// `centre-v1`: sample between pixel **centres**. Before it,
/// `resample_bilinear` mapped corner pixel to corner pixel, so a 512 → 256
/// Mapterhorn reduction was stretched by one source pixel — off by ±¼ output
/// px at the tile edges — and the upsample above `DEM_MAX_ZOOM` was off by up
/// to (factor − 1)/2 output px, with a step at every child-tile seam.
pub const DEM_RESAMPLE_VERSION: &str = "centre-v1";

/// `w × h` as a length, or `None` when it does not fit `u32` (the index
/// arithmetic below is `u32`, as it always was).
fn area(w: u32, h: u32) -> Option<usize> {
    w.checked_mul(h).map(|n| n as usize)
}

/// All-NaN fallback for malformed input.
fn nan_grid(w: u32, h: u32) -> Vec<f64> {
    vec![f64::NAN; area(w, h).unwrap_or(0)]
}

/// Fit a decoded native DEM tile to the requested `tile_size`, returning the
/// elevations and the tile's ETag fragment.
///
/// A same-size tile passes through untouched with `etag` as given. Anything
/// else is resampled with [`resample_bilinear`] and its ETag gains
/// `resample:{DEM_RESAMPLE_VERSION}`, so only resampled tiles move key when
/// the resampling changes.
pub fn fit_to_tile_size(
    native: Vec<f64>,
    src_w: u32,
    src_h: u32,
    tile_size: u32,
    etag: Option<String>,
) -> (Vec<f64>, Option<String>) {
    if src_w == tile_size && src_h == tile_size {
        return (native, etag);
    }
    let marker = format!("resample:{DEM_RESAMPLE_VERSION}");
    let etag = Some(match etag {
        Some(e) => format!("{e}+{marker}"),
        None => marker,
    });
    (
        resample_bilinear(&native, src_w, src_h, tile_size, tile_size),
        etag,
    )
}

/// Serve a child tile at zoom `dem_max + zoom_diff` from its `parent` at the
/// upstream DEM's `max_zoom`: bilinear-upsample the child's
/// `(sub_x, sub_y)` cell of the parent to `tile_size × tile_size`.
///
/// Delegates to [`upsample_subregion`], which samples at child pixel centres
/// and reads neighbours from the whole parent (not just the child's cell), so
/// adjacent children agree at their shared edge.
pub fn extract_and_upsample(
    parent: &[f64],
    tile_size: u32,
    zoom_diff: u8,
    sub_x: u32,
    sub_y: u32,
) -> Vec<f64> {
    // A shift of 32 or more is malformed; factor 0 is refused downstream.
    let factor = 1u32.checked_shl(zoom_diff as u32).unwrap_or(0);
    upsample_subregion(parent, tile_size, factor, sub_x, sub_y)
}

/// Bilinear resample a row-major `src_w × src_h` grid to `dst_w × dst_h`,
/// treating both as covering the same extent and sampling at **pixel
/// centres**: output pixel `d` reads source position
/// `(d + 0.5) · src / dst − 0.5`. Positions beyond the outermost source
/// centres (only when upsampling) clamp to the edge sample — the same rule as
/// [`upsample_subregion`] and `MercatorDem::sample`. A NaN neighbour makes
/// the output NaN, as before.
///
/// For the exact 2:1 reduction (Mapterhorn 512 → 256) every output centre
/// lands midway between four source centres, so this is the plain mean of
/// each 2×2 block: no source pixel is dropped or double-counted, and a block
/// containing a NaN stays NaN rather than being filled from its neighbours.
pub fn resample_bilinear(src: &[f64], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<f64> {
    let (Some(src_len), Some(dst_len)) = (area(src_w, src_h), area(dst_w, dst_h)) else {
        return Vec::new();
    };
    if src_len == 0 || src.len() < src_len {
        return nan_grid(dst_w, dst_h);
    }
    let sx_scale = src_w as f64 / dst_w as f64;
    let sy_scale = src_h as f64 / dst_h as f64;
    let max_x = (src_w - 1) as f64;
    let max_y = (src_h - 1) as f64;
    let mut out = Vec::with_capacity(dst_len);
    for dy in 0..dst_h {
        let sy = ((dy as f64 + 0.5) * sy_scale - 0.5).clamp(0.0, max_y);
        let y0 = sy.floor() as u32;
        let y1 = (y0 + 1).min(src_h - 1);
        let fy = sy - y0 as f64;
        for dx in 0..dst_w {
            let sx = ((dx as f64 + 0.5) * sx_scale - 0.5).clamp(0.0, max_x);
            let x0 = sx.floor() as u32;
            let x1 = (x0 + 1).min(src_w - 1);
            let fx = sx - x0 as f64;
            let get = |x: u32, y: u32| src[(y * src_w + x) as usize];
            let v0 = get(x0, y0) * (1.0 - fx) + get(x1, y0) * fx;
            let v1 = get(x0, y1) * (1.0 - fx) + get(x1, y1) * fx;
            out.push(v0 * (1.0 - fy) + v1 * fy);
        }
    }
    out
}

/// Bilinear-upsample one sub-tile of a parent grid to `tile_size × tile_size`.
///
/// `parent` is a `tile_size × tile_size` grid covering one parent tile. The
/// child tile occupies the `(sub_x, sub_y)`-th cell of a `factor × factor`
/// subdivision of the parent. Pixel centers are sampled with bilinear
/// interpolation; NaN samples in the parent propagate as NaN.
///
/// A child pixel centre within half a parent pixel of the parent's border
/// lies outside the parent's outermost pixel centres, so one of its four
/// neighbours is off the grid. Those neighbours clamp to the nearest edge
/// sample — the same rule as `terrain_codec::mercator::MercatorDem::sample` —
/// rather than becoming NaN. The parent does have data there; treating the
/// half-pixel rim as missing used to put a NaN band (1 px at factor 2, 2 px
/// from factor 4) along every edge a child shares with its parent, which the
/// raster endpoints encoded as −32768 m (Terrarium) / −10000 m (Mapbox).
pub fn upsample_subregion(
    parent: &[f64],
    tile_size: u32,
    factor: u32,
    sub_x: u32,
    sub_y: u32,
) -> Vec<f64> {
    let Some(n) = area(tile_size, tile_size) else {
        return Vec::new();
    };
    if n == 0 || factor == 0 || parent.len() < n {
        return nan_grid(tile_size, tile_size);
    }
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

/// Bilinear sample of a `width × width` grid at fractional pixel `(x, y)`,
/// off-grid neighbours clamped to the edge sample (see
/// [`upsample_subregion`]). NaN when a neighbour is NaN, or when the grid is
/// empty or shorter than `width²`.
pub fn bilinear_at(grid: &[f64], width: u32, x: f64, y: f64) -> f64 {
    let w = width as i64;
    if w == 0 || area(width, width).is_none_or(|n| grid.len() < n) {
        return f64::NAN;
    }
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let dx = x - x0 as f64;
    let dy = y - y0 as f64;
    // Off-grid neighbours clamp to the edge sample (see `upsample_subregion`).
    let get = |xi: i64, yi: i64| -> f64 {
        let xi = xi.clamp(0, w - 1);
        let yi = yi.clamp(0, w - 1);
        grid[(yi * w + xi) as usize]
    };
    let v00 = get(x0, y0);
    let v10 = get(x0.saturating_add(1), y0);
    let v01 = get(x0, y0.saturating_add(1));
    let v11 = get(x0.saturating_add(1), y0.saturating_add(1));
    if v00.is_nan() || v10.is_nan() || v01.is_nan() || v11.is_nan() {
        return f64::NAN;
    }
    v00 * (1.0 - dx) * (1.0 - dy) + v10 * dx * (1.0 - dy) + v01 * (1.0 - dx) * dy + v11 * dx * dy
}

#[cfg(test)]
mod tests {
    use super::{
        DEM_RESAMPLE_VERSION, bilinear_at, extract_and_upsample, fit_to_tile_size,
        resample_bilinear, upsample_subregion,
    };

    /// `n × n` grid whose value is `100·row + col` — linear in both axes, so
    /// any centre-aligned bilinear sample can be predicted exactly.
    fn ramp(n: u32) -> Vec<f64> {
        (0..n * n)
            .map(|i| (i / n) as f64 * 100.0 + (i % n) as f64)
            .collect()
    }

    #[test]
    fn resample_identity() {
        let src: Vec<f64> = (0..9).map(|v| v as f64).collect();
        let out = resample_bilinear(&src, 3, 3, 3, 3);
        assert_eq!(out, src);
    }

    /// 512 → 256 must be the mean of each 2×2 block. The old corner-aligned
    /// mapping (`d · 511/255`) read single source pixels near the edges and
    /// stretched the tile by one source pixel.
    #[test]
    fn halving_is_the_two_by_two_block_mean() {
        let src: Vec<f64> = (0..512u32 * 512)
            .map(|i: u32| (i.wrapping_mul(2_654_435_761) % 9973) as f64)
            .collect();
        let out = resample_bilinear(&src, 512, 512, 256, 256);
        for dy in 0..256usize {
            for dx in 0..256usize {
                let at = |x: usize, y: usize| src[y * 512 + x];
                let mean = (at(2 * dx, 2 * dy)
                    + at(2 * dx + 1, 2 * dy)
                    + at(2 * dx, 2 * dy + 1)
                    + at(2 * dx + 1, 2 * dy + 1))
                    / 4.0;
                let got = out[dy * 256 + dx];
                assert!((got - mean).abs() < 1e-9, "({dx},{dy}): {got} vs {mean}");
            }
        }
    }

    /// On a ramp the output centre `d` must read source position `2d + 0.5`
    /// at both edges: no shift, no stretch.
    #[test]
    fn halving_keeps_pixel_centres_in_place() {
        let out = resample_bilinear(&ramp(512), 512, 512, 256, 256);
        for (dx, dy) in [(0usize, 0usize), (255, 0), (0, 255), (255, 255), (128, 77)] {
            let want = (2.0 * dy as f64 + 0.5) * 100.0 + (2.0 * dx as f64 + 0.5);
            assert!(
                (out[dy * 256 + dx] - want).abs() < 1e-9,
                "({dx},{dy}): {} vs {want}",
                out[dy * 256 + dx]
            );
        }
    }

    /// Upsampling reads `(d + 0.5)/2 − 0.5` and clamps the outer half pixel
    /// to the edge sample.
    #[test]
    fn doubling_samples_between_centres() {
        let out = resample_bilinear(&ramp(4), 4, 4, 8, 8);
        let at = |x: usize, y: usize| out[y * 8 + x];
        assert_eq!(at(0, 0), 0.0); // −0.25 clamps to the corner sample
        assert!((at(1, 0) - 0.25).abs() < 1e-12);
        assert!((at(7, 7) - 303.0).abs() < 1e-12); // 3.25 clamps to 3
        assert!((at(4, 2) - (75.0 + 1.75)).abs() < 1e-12);
    }

    #[test]
    fn resample_propagates_nan() {
        let mut src = ramp(4);
        src[5] = f64::NAN; // (1, 1)
        let out = resample_bilinear(&src, 4, 4, 2, 2);
        assert!(out[0].is_nan(), "the block holding the NaN stays a hole");
        assert!(out[1].is_finite() && out[2].is_finite() && out[3].is_finite());
    }

    /// Children of one parent, sampled at their own pixel centres, must
    /// reproduce the parent ramp — continuous across the seams between
    /// siblings, with the same step as inside a child.
    #[test]
    fn upsample_children_are_centre_aligned_and_seamless() {
        let n = 16u32;
        let parent = ramp(n);
        for zoom_diff in [1u8, 2, 3] {
            let f = 1u32 << zoom_diff;
            let step = 1.0 / f as f64;
            for sub in 0..f {
                let child = extract_and_upsample(&parent, n, zoom_diff, sub, sub);
                for c in [0u32, 1, n / 2, n - 2, n - 1] {
                    // Parent-pixel coordinate of child centre `c`, clamped
                    // to the parent's outermost centres.
                    let p = (sub as f64 * n as f64 / f as f64 + (c as f64 + 0.5) * step - 0.5)
                        .clamp(0.0, (n - 1) as f64);
                    let want = 100.0 * p + p;
                    let got = child[(c * n + c) as usize];
                    assert!(
                        (got - want).abs() < 1e-9,
                        "zoom_diff {zoom_diff} sub {sub} px {c}: {got} vs {want}"
                    );
                }
                if sub + 1 < f {
                    // Seam to the east sibling, away from the parent's border.
                    let east = extract_and_upsample(&parent, n, zoom_diff, sub + 1, sub);
                    let row = (n / 2) as usize * n as usize;
                    let seam = east[row] - child[row + n as usize - 1];
                    assert!(
                        (seam - step).abs() < 1e-9,
                        "zoom_diff {zoom_diff} sub {sub}: seam step {seam}, interior {step}"
                    );
                }
            }
        }
    }

    #[test]
    fn upsample_factor_one_is_passthrough() {
        let parent: Vec<f64> = (0..16).map(|v| v as f64).collect();
        let out = extract_and_upsample(&parent, 4, 0, 0, 0);
        assert_eq!(out, parent);
    }

    /// Only a tile that was actually resampled takes the resample version
    /// into its ETag, so a resampling change re-keys exactly those tiles.
    #[test]
    fn fit_marks_only_resampled_tiles() {
        let (same, etag) = fit_to_tile_size(ramp(4), 4, 4, 4, Some("abc".into()));
        assert_eq!(same, ramp(4));
        assert_eq!(etag.as_deref(), Some("abc"));

        let marker = format!("resample:{DEM_RESAMPLE_VERSION}");
        let (half, etag) = fit_to_tile_size(ramp(4), 4, 4, 2, Some("abc".into()));
        assert_eq!(half.len(), 4);
        assert_eq!(etag, Some(format!("abc+{marker}")));
        let (_, etag) = fit_to_tile_size(ramp(4), 4, 4, 8, None);
        assert_eq!(etag, Some(marker));
    }

    /// Malformed sizes and coordinates give NaN / empty grids, never a panic.
    #[test]
    fn malformed_input_does_not_panic() {
        let g = ramp(4);
        assert!(resample_bilinear(&g, 0, 4, 2, 2).iter().all(|v| v.is_nan()));
        assert!(resample_bilinear(&g, 8, 8, 2, 2).iter().all(|v| v.is_nan()));
        assert!(resample_bilinear(&g, 4, 4, 0, 0).is_empty());
        assert_eq!(resample_bilinear(&g, 4, 4, 3, 0).len(), 0);
        assert!(resample_bilinear(&g, u32::MAX, u32::MAX, 2, 2).is_empty());
        assert!(resample_bilinear(&[], 1, 1, u32::MAX, u32::MAX).is_empty());
        assert!(
            upsample_subregion(&g, 4, 0, 0, 0)
                .iter()
                .all(|v| v.is_nan())
        );
        assert!(
            upsample_subregion(&g, 5, 2, 0, 0)
                .iter()
                .all(|v| v.is_nan())
        );
        assert!(upsample_subregion(&g, 0, 2, 0, 0).is_empty());
        assert!(upsample_subregion(&g, u32::MAX, 2, 0, 0).is_empty());
        let far = upsample_subregion(&g, 4, 2, u32::MAX, u32::MAX);
        assert_eq!(far.len(), 16);
        assert!(
            extract_and_upsample(&g, 4, 40, 0, 0)
                .iter()
                .all(|v| v.is_nan())
        );
        for (x, y) in [
            (f64::NAN, 0.0),
            (f64::INFINITY, f64::NEG_INFINITY),
            (1e300, -1e300),
            (i64::MAX as f64, i64::MIN as f64),
        ] {
            let _ = bilinear_at(&g, 4, x, y);
        }
        assert!(bilinear_at(&g, 0, 0.0, 0.0).is_nan());
        assert!(bilinear_at(&g, 5, 0.0, 0.0).is_nan());
        let (v, _) = fit_to_tile_size(vec![], 0, 0, 2, None);
        assert_eq!(v.len(), 4);
    }
}
