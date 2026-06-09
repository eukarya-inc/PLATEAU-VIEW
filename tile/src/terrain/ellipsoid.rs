//! Ellipsoidal height composition (orthometric DEM + geoid).
//!
//! For each pixel in the 65×65 geodetic output grid, we add the geoid height
//! at that pixel's (lng, lat) to the orthometric DEM height. Out-of-coverage
//! geoid pixels fall back to 0 (see user requirement: partial-coverage tiles
//! still render, the out-of-coverage area is treated as zero-offset).

use super::geodetic::{CESIUM_TILE_SIZE, GeodeticBounds};
use super::geoid::Geoid;
use super::webmercator::{xyz_pixel_lat, xyz_pixel_lon};

/// Apply the geoid to an orthometric elevation grid (in-place), producing
/// ellipsoidal heights.
///
/// The grid is 65×65 row-major, north-first (matching
/// `fetch_geodetic_tile_elevations_with_halo`). NaN elevations are preserved.
pub fn apply_geoid_to_grid(bounds: &GeodeticBounds, grid: &mut [f64], geoid: &Geoid) {
    apply_geoid_to_grid_sized(bounds, grid, geoid, CESIUM_TILE_SIZE as usize);
}

/// Like [`apply_geoid_to_grid`] but for an arbitrarily-sized grid covering
/// exactly `bounds`. Used for the halo-extended elevation grid that drives
/// gradient-based normals — the geoid needs to be added to the halo cells
/// too so the gradient is computed in the same vertical datum as the mesh.
pub fn apply_geoid_to_grid_sized(
    bounds: &GeodeticBounds,
    grid: &mut [f64],
    geoid: &Geoid,
    grid_size: usize,
) {
    debug_assert_eq!(grid.len(), grid_size * grid_size);

    for dst_y in 0..grid_size {
        let t_y = dst_y as f64 / (grid_size - 1) as f64;
        let lat = bounds.north - t_y * (bounds.north - bounds.south);
        for dst_x in 0..grid_size {
            let t_x = dst_x as f64 / (grid_size - 1) as f64;
            let lng = bounds.west + t_x * (bounds.east - bounds.west);
            let idx = dst_y * grid_size + dst_x;
            let ortho = grid[idx];
            if ortho.is_nan() {
                continue;
            }
            grid[idx] = ortho + geoid.height_or_zero(lng, lat);
        }
    }
}

/// Apply the geoid to an orthometric elevation grid (in-place) for a
/// `tile_size × tile_size` Web Mercator XYZ tile, producing ellipsoidal
/// heights.
///
/// The grid is row-major with row 0 at the tile's north edge and column 0 at
/// the west edge (matching the layout returned by `DemProvider`). Latitude is
/// mercator-Y uniform (not lat uniform), so we recompute lat per row.
/// NaN elevations are preserved; out-of-coverage geoid samples fall back to 0.
pub fn apply_geoid_to_xyz_grid(
    z: u8,
    x: u32,
    y: u32,
    tile_size: u32,
    grid: &mut [f64],
    geoid: &Geoid,
) {
    let n = tile_size as usize;
    debug_assert_eq!(grid.len(), n * n);

    // Pre-compute per-column longitudes (lon is column-linear).
    let lons: Vec<f64> = (0..tile_size)
        .map(|px| xyz_pixel_lon(z, x, tile_size, px))
        .collect();

    for py in 0..tile_size {
        let lat = xyz_pixel_lat(z, y, tile_size, py);
        let row_off = (py as usize) * n;
        for (px, &lng) in lons.iter().enumerate() {
            let idx = row_off + px;
            let ortho = grid[idx];
            if ortho.is_nan() {
                continue;
            }
            grid[idx] = ortho + geoid.height_or_zero(lng, lat);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::geoid::{Geoid, GeoidModel};
    use super::*;

    #[test]
    fn applies_geoid_over_tokyo() {
        let bounds = GeodeticBounds {
            west: 139.0,
            south: 35.0,
            east: 140.0,
            north: 36.0,
        };
        let n = CESIUM_TILE_SIZE as usize;
        let mut grid = vec![100.0f64; n * n];
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        apply_geoid_to_grid(&bounds, &mut grid, &geoid);
        // All pixels should have been shifted by a finite positive geoid offset
        // (Japan's geoid height is roughly 30-40m).
        assert!(grid.iter().all(|&h| h > 120.0 && h < 160.0));
    }

    #[test]
    fn xyz_grid_applies_geoid_over_tokyo() {
        // Tokyo z=10 tile (≈139.7E, 35.7N).
        let size = 32u32;
        let mut grid = vec![100.0f64; (size * size) as usize];
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        apply_geoid_to_xyz_grid(10, 909, 403, size, &mut grid, &geoid);
        assert!(grid.iter().all(|&h| h > 120.0 && h < 160.0));
    }

    #[test]
    fn xyz_grid_out_of_coverage_keeps_orthometric() {
        // Mid-Pacific: well outside GSIGEO2011 coverage.
        let size = 8u32;
        let mut grid = vec![42.0f64; (size * size) as usize];
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        apply_geoid_to_xyz_grid(4, 2, 7, size, &mut grid, &geoid);
        assert!(grid.iter().all(|&h| (h - 42.0).abs() < 1e-9));
    }

    #[test]
    fn out_of_coverage_keeps_orthometric() {
        let bounds = GeodeticBounds {
            west: -160.0,
            south: 5.0,
            east: -150.0,
            north: 15.0,
        };
        let n = CESIUM_TILE_SIZE as usize;
        let mut grid = vec![42.0f64; n * n];
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        apply_geoid_to_grid(&bounds, &mut grid, &geoid);
        // Outside Japan the geoid returns NaN → fallback 0 → ellipsoidal == orthometric.
        assert!(grid.iter().all(|&h| (h - 42.0).abs() < 1e-9));
    }
}
