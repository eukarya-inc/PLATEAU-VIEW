//! Terrain generation and serving.
//!
//! Provides DEM-backed Cesium quantized-mesh-1.0 tiles and Terrarium raster tiles.
//! Heights are served as ellipsoidal (orthometric DEM + geoid via `japan-geoid`).
//!
//! Modules ported from <https://github.com/MIERUNE/stralift> (quantized-mesh,
//! martini, cesium/geodetic, cesium/layer_json, format/terrarium).

#![allow(dead_code)]

pub mod cog_dem;
pub mod composite;
pub mod dem;
pub mod ellipsoid;
pub mod geodetic;
pub mod geoid;
pub mod layer_json;
pub mod mapbox;
pub mod mapterhorn;
pub mod martini;
pub mod mesh_gen;
pub mod pmtiles;
pub mod quantized_mesh;
pub mod settings;
pub mod terrarium;
pub mod webmercator;
pub mod xyz_dem;

pub use cog_dem::CogDemSource;
pub use composite::{CompositeDemProvider, build as build_composite_dem};
pub use dem::{DemError, DemProvider, DemTile, GeoBounds};
pub use geoid::{Geoid, GeoidModel, UnknownGeoidModel};
pub use mapterhorn::MapterhornSource;
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
/// Per stralift's per-source upsampling. The interpolation is independent
/// per child tile, so adjacent children may differ by sub-pixel amounts
/// at their shared edge — acceptable for terrain rendering.
pub(crate) fn extract_and_upsample(
    parent: &[f64],
    tile_size: u32,
    zoom_diff: u8,
    sub_x: u32,
    sub_y: u32,
) -> Vec<f64> {
    let factor = 1u32 << zoom_diff;
    let sub_size = (tile_size / factor).max(1);
    let off_x = sub_x * sub_size;
    let off_y = sub_y * sub_size;

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
}
