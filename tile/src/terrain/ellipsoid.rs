//! Ellipsoidal height composition (orthometric DEM + geoid).
//!
//! For each pixel in the 65×65 geodetic output grid, we add the geoid height
//! at that pixel's (lng, lat) to the orthometric DEM height. Out-of-coverage
//! geoid pixels fall back to 0 (see user requirement: partial-coverage tiles
//! still render, the out-of-coverage area is treated as zero-offset).

use super::geodetic::{CESIUM_TILE_SIZE, GeodeticBounds};
use super::geoid::Geoid;

/// Apply the geoid to an orthometric elevation grid (in-place), producing
/// ellipsoidal heights.
///
/// The grid is 65×65 row-major, north-first (matching
/// `fetch_geodetic_tile_elevations`). NaN elevations are preserved.
pub fn apply_geoid_to_grid(bounds: &GeodeticBounds, grid: &mut [f64], geoid: &Geoid) {
    let n = CESIUM_TILE_SIZE as usize;
    debug_assert_eq!(grid.len(), n * n);

    for dst_y in 0..n {
        let t_y = dst_y as f64 / (n - 1) as f64;
        let lat = bounds.north - t_y * (bounds.north - bounds.south);
        for dst_x in 0..n {
            let t_x = dst_x as f64 / (n - 1) as f64;
            let lng = bounds.west + t_x * (bounds.east - bounds.west);
            let idx = dst_y * n + dst_x;
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
