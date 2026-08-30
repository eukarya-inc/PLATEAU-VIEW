//! Terrain generation and serving.
//!
//! Provides DEM-backed Cesium quantized-mesh-1.0 tiles and Terrarium raster tiles.
//! Heights are served as ellipsoidal (orthometric DEM + geoid via `japan-geoid`).

#![allow(dead_code)]

pub mod cached_dem;
pub mod cog_dem;
pub mod composite;
pub mod dem;
pub mod ellipsoid;
pub mod geodetic;
pub mod geoid;
pub mod mapterhorn;
pub mod mesh_gen;
pub mod mirror;
pub mod pmtiles;
pub mod sealevel;
pub mod settings;
pub mod webmercator;
pub mod xyz_dem;

pub use cached_dem::CachedDemProvider;
pub use cog_dem::CogDemSource;
pub use composite::{CompositeDemProvider, build as build_composite_dem};
pub use dem::{DemError, DemProvider, DemTile, GeoBounds};
pub use geoid::{Geoid, GeoidModel, UnknownGeoidModel};
pub use mapterhorn::MapterhornSource;
pub use mirror::MirrorSource;
pub use pmtiles::{PmtilesEncoding, PmtilesSource};
pub use settings::TerrainSettings;
pub use xyz_dem::{XyzDemEncoding, XyzDemSource};

/// Extract a `(tile_size / 2^zoom_diff)`-wide sub-region of `parent` and
/// bilinear-upsample it back to `tile_size × tile_size`. Used to serve
/// raster-DEM tiles at zoom levels above the upstream DEM's `max_zoom`:
/// the requested child `(z, x, y)` is mapped to a parent tile at
/// `dem_max_zoom`, the relevant quadrant is extracted, and the missing
/// detail is filled in by bilinear interpolation.
///
/// The interpolation is independent per child tile, so adjacent children
/// may differ by sub-pixel amounts at their shared edge — acceptable for
/// terrain rendering.
pub(crate) fn extract_and_upsample(
    parent: &[f64],
    tile_size: u32,
    zoom_diff: u8,
    sub_x: u32,
    sub_y: u32,
) -> Vec<f64> {
    // Defense in depth against a caller feeding a zoom_diff that would push
    // the sub-region out of `parent`. When `factor > tile_size` (i.e.
    // `zoom_diff > floor(log2(tile_size))`), `sub_size` saturates to 1 but
    // the raw `sub_x`/`sub_y` inputs can each reach `factor - 1`, so
    // `off_x + px` walks past `tile_size` and the always-on bounds check on
    // `parent` panics — an unauthenticated DoS. The primary guard lives in
    // the raster_tile handler (rejects `z > max_zoom`); this saturating
    // fallback keeps a stray call from panicking either way.
    //
    // Use `ilog2` (floor log2) rather than `trailing_zeros`: they agree on
    // power-of-two `tile_size` (the only shape production uses) but for a
    // non-power-of-two like 255 `trailing_zeros = 0` would clamp away every
    // legitimate `zoom_diff`. `ilog2` panics at 0, so guard that first.
    let max_zoom_diff = if tile_size == 0 {
        0
    } else {
        tile_size.ilog2() as u8
    };
    let zoom_diff = zoom_diff.min(max_zoom_diff);
    let factor = 1u32 << zoom_diff;
    let sub_size = (tile_size / factor).max(1);
    let clamped_sub_x = sub_x.min(factor.saturating_sub(1));
    let clamped_sub_y = sub_y.min(factor.saturating_sub(1));
    let off_x = clamped_sub_x * sub_size;
    let off_y = clamped_sub_y * sub_size;

    let mut sub = Vec::with_capacity((sub_size * sub_size) as usize);
    for py in 0..sub_size {
        let row = ((off_y + py) * tile_size) as usize;
        for px in 0..sub_size {
            sub.push(parent[row + (off_x + px) as usize]);
        }
    }

    resample_bilinear(&sub, sub_size, sub_size, tile_size, tile_size)
}

/// Bilinear resample a row-major grid. Shared between DEM sources.
pub(crate) fn resample_bilinear(
    src: &[f64],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Vec<f64> {
    let mut out = Vec::with_capacity((dst_w * dst_h) as usize);
    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let sx = (dx as f64) * (src_w - 1) as f64 / (dst_w - 1).max(1) as f64;
            let sy = (dy as f64) * (src_h - 1) as f64 / (dst_h - 1).max(1) as f64;
            let x0 = sx.floor() as u32;
            let y0 = sy.floor() as u32;
            let x1 = (x0 + 1).min(src_w - 1);
            let y1 = (y0 + 1).min(src_h - 1);
            let fx = sx - x0 as f64;
            let fy = sy - y0 as f64;
            let get = |x: u32, y: u32| src[(y * src_w + x) as usize];
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

#[cfg(test)]
mod tests {
    use super::{extract_and_upsample, resample_bilinear};

    #[test]
    fn resample_identity() {
        let src: Vec<f64> = (0..9).map(|v| v as f64).collect();
        let out = resample_bilinear(&src, 3, 3, 3, 3);
        assert_eq!(out, src);
    }

    #[test]
    fn upsample_extracts_correct_quadrant() {
        // 4x4 parent: each row 100*y + x.
        let parent: Vec<f64> = (0..16)
            .map(|i| {
                let y = (i / 4) as f64;
                let x = (i % 4) as f64;
                y * 100.0 + x
            })
            .collect();
        // zoom_diff=1, so each child is 2x2 within a 4x4 parent.
        // Top-left child (sub=0,0) should cover parent[0..2, 0..2] = [0,1,100,101].
        let tl = extract_and_upsample(&parent, 4, 1, 0, 0);
        assert_eq!(tl.len(), 16);
        assert!((tl[0] - 0.0).abs() < 1e-9);
        assert!((tl[15] - 101.0).abs() < 1e-9);
        // Bottom-right child (sub=1,1) should cover parent[2..4, 2..4] = [202,203,302,303].
        let br = extract_and_upsample(&parent, 4, 1, 1, 1);
        assert!((br[0] - 202.0).abs() < 1e-9);
        assert!((br[15] - 303.0).abs() < 1e-9);
    }

    #[test]
    fn upsample_factor_one_is_passthrough() {
        let parent: Vec<f64> = (0..16).map(|v| v as f64).collect();
        let out = extract_and_upsample(&parent, 4, 0, 0, 0);
        assert_eq!(out, parent);
    }

    // Regression: an out-of-range zoom_diff (factor > tile_size) with wide
    // sub_x/sub_y used to walk off the end of `parent` and panic on the
    // bounds check. Defense-in-depth clamping must return a valid tile.
    #[test]
    fn upsample_zoom_diff_beyond_tile_size_does_not_panic() {
        let tile_size = 4u32;
        let parent: Vec<f64> = (0..(tile_size * tile_size) as usize)
            .map(|v| v as f64)
            .collect();
        // Original bug: for zoom_diff=10, factor=1024 >> tile_size=4, and
        // sub_x/sub_y up to factor-1 index parent[~] out of bounds.
        let out = extract_and_upsample(&parent, tile_size, 10, 1023, 1023);
        assert_eq!(out.len(), (tile_size * tile_size) as usize);
    }

    #[test]
    fn upsample_clamps_sub_x_y_at_boundary() {
        let tile_size = 4u32;
        let parent: Vec<f64> = (0..(tile_size * tile_size) as usize)
            .map(|v| v as f64)
            .collect();
        // With zoom_diff=2 and tile_size=4, factor=4 → valid sub_x/sub_y are
        // [0,3]. Feed both the last-in-range case (3,3) and an out-of-range
        // case (4,4) and require they produce the SAME output — that's the
        // clamping we're testing (sub_x/y are min'd with factor-1).
        let in_range = extract_and_upsample(&parent, tile_size, 2, 3, 3);
        let out_of_range = extract_and_upsample(&parent, tile_size, 2, 4, 4);
        assert_eq!(in_range.len(), (tile_size * tile_size) as usize);
        assert_eq!(out_of_range, in_range);
    }

    // Non-power-of-two tile_size: verify max_zoom_diff uses ilog2 (floor),
    // not trailing_zeros. With trailing_zeros a size like 6 would clamp
    // zoom_diff to 1 (the trailing 0 bit of 0b110); ilog2 correctly gives 2,
    // matching the intended `factor <= tile_size` invariant.
    #[test]
    fn upsample_non_power_of_two_tile_size() {
        let tile_size = 6u32;
        let parent: Vec<f64> = (0..(tile_size * tile_size) as usize)
            .map(|v| v as f64)
            .collect();
        // zoom_diff=2 → factor=4 <= 6, so it should NOT be clamped and the
        // caller's sub_x=3, sub_y=3 should be honored (still within factor-1).
        let out = extract_and_upsample(&parent, tile_size, 2, 3, 3);
        assert_eq!(out.len(), (tile_size * tile_size) as usize);
    }
}
