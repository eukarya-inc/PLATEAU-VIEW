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
pub mod xyz_dem;

pub use cog_dem::CogDemSource;
pub use composite::{CompositeDemProvider, build as build_composite_dem};
pub use dem::{DemError, DemProvider, DemTile, GeoBounds};
pub use geoid::{Geoid, GeoidModel, UnknownGeoidModel};
pub use mapterhorn::MapterhornSource;
pub use pmtiles::{PmtilesEncoding, PmtilesSource};
pub use settings::TerrainSettings;
pub use xyz_dem::{XyzDemEncoding, XyzDemSource};

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
    use super::resample_bilinear;

    #[test]
    fn resample_identity() {
        let src: Vec<f64> = (0..9).map(|v| v as f64).collect();
        let out = resample_bilinear(&src, 3, 3, 3, 3);
        assert_eq!(out, src);
    }
}
