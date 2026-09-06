//! Vertical-surface composition (orthometric DEM, geoid, or their sum).
//!
//! For each pixel in the output grid we sample the geoid height at that pixel's
//! (lng, lat) and combine it with the orthometric DEM height according to the
//! requested [`HeightMode`]. Out-of-coverage geoid pixels fall back to 0 (see
//! user requirement: partial-coverage tiles still render, the out-of-coverage
//! area is treated as zero-offset).
//!
//! The grid traversal is shared by all three modes and by both projections, so
//! the geoid-only surface is sampled at exactly the same points as the
//! ellipsoidal one.

use super::geodetic::{CESIUM_TILE_SIZE, GeodeticBounds};
use super::geoid::{Geoid, HeightMode};

use super::webmercator::{xyz_pixel_lat, xyz_pixel_lon};

/// Combine one orthometric sample with one geoid sample per `mode`.
///
/// `Ellipsoidal` preserves NaN DEM samples (a failed/absent DEM pixel stays a
/// hole); `GeoidOnly` ignores the DEM entirely, so a NaN DEM sample still
/// yields a defined geoid height.
#[inline]
fn combine(mode: HeightMode, ortho: f64, geoid_height: f64) -> f64 {
    match mode {
        HeightMode::Orthometric => ortho,
        HeightMode::GeoidOnly => geoid_height,
        HeightMode::Ellipsoidal => {
            if ortho.is_nan() {
                ortho
            } else {
                ortho + geoid_height
            }
        }
    }
}

/// Rewrite an orthometric elevation grid (in-place) into the surface selected
/// by `mode`.
///
/// The grid is 65×65 row-major, north-first (matching
/// `fetch_geodetic_tile_elevations_with_halo`).
pub fn apply_height_mode_to_grid(
    bounds: &GeodeticBounds,
    grid: &mut [f64],
    geoid: &Geoid,
    mode: HeightMode,
) {
    apply_height_mode_to_grid_sized(bounds, grid, geoid, CESIUM_TILE_SIZE as usize, mode);
}

/// Like [`apply_height_mode_to_grid`] but for an arbitrarily-sized grid covering
/// exactly `bounds`. Used for the halo-extended elevation grid that drives
/// gradient-based normals — the halo cells must land on the same surface as the
/// tile interior so the gradient is computed in one vertical datum.
pub fn apply_height_mode_to_grid_sized(
    bounds: &GeodeticBounds,
    grid: &mut [f64],
    geoid: &Geoid,
    grid_size: usize,
    mode: HeightMode,
) {
    debug_assert_eq!(grid.len(), grid_size * grid_size);

    // Orthometric is the DEM as-is: no geoid sampling at all.
    if mode == HeightMode::Orthometric {
        return;
    }

    for dst_y in 0..grid_size {
        let t_y = dst_y as f64 / (grid_size - 1) as f64;
        let lat = bounds.north - t_y * (bounds.north - bounds.south);
        for dst_x in 0..grid_size {
            let t_x = dst_x as f64 / (grid_size - 1) as f64;
            let lng = bounds.west + t_x * (bounds.east - bounds.west);
            let idx = dst_y * grid_size + dst_x;
            let ortho = grid[idx];
            if ortho.is_nan() && mode == HeightMode::Ellipsoidal {
                continue;
            }
            grid[idx] = combine(mode, ortho, geoid.height_or_zero(lng, lat));
        }
    }
}

/// Rewrite an orthometric elevation grid (in-place) into the surface selected
/// by `mode`, for a `tile_size × tile_size` Web Mercator XYZ tile.
///
/// The grid is row-major with row 0 at the tile's north edge and column 0 at
/// the west edge (matching the layout returned by `DemProvider`). Latitude is
/// mercator-Y uniform (not lat uniform), so we recompute lat per row.
/// Out-of-coverage geoid samples fall back to 0.
pub fn apply_height_mode_to_xyz_grid(
    z: u8,
    x: u32,
    y: u32,
    tile_size: u32,
    grid: &mut [f64],
    geoid: &Geoid,
    mode: HeightMode,
) {
    let n = tile_size as usize;
    debug_assert_eq!(grid.len(), n * n);

    if mode == HeightMode::Orthometric {
        return;
    }

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
            if ortho.is_nan() && mode == HeightMode::Ellipsoidal {
                continue;
            }
            grid[idx] = combine(mode, ortho, geoid.height_or_zero(lng, lat));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::geoid::{Geoid, GeoidModel};
    use super::*;

    const TOKYO: GeodeticBounds = GeodeticBounds {
        west: 139.0,
        south: 35.0,
        east: 140.0,
        north: 36.0,
    };

    #[test]
    fn applies_geoid_over_tokyo() {
        let n = CESIUM_TILE_SIZE as usize;
        let mut grid = vec![100.0f64; n * n];
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        apply_height_mode_to_grid(&TOKYO, &mut grid, &geoid, HeightMode::Ellipsoidal);
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
        apply_height_mode_to_xyz_grid(
            10,
            909,
            403,
            size,
            &mut grid,
            &geoid,
            HeightMode::Ellipsoidal,
        );
        assert!(grid.iter().all(|&h| h > 120.0 && h < 160.0));
    }

    #[test]
    fn xyz_grid_out_of_coverage_keeps_orthometric() {
        // Mid-Pacific: well outside GSIGEO2011 coverage.
        let size = 8u32;
        let mut grid = vec![42.0f64; (size * size) as usize];
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        apply_height_mode_to_xyz_grid(4, 2, 7, size, &mut grid, &geoid, HeightMode::Ellipsoidal);
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
        apply_height_mode_to_grid(&bounds, &mut grid, &geoid, HeightMode::Ellipsoidal);
        // Outside Japan the geoid returns NaN → fallback 0 → ellipsoidal == orthometric.
        assert!(grid.iter().all(|&h| (h - 42.0).abs() < 1e-9));
    }

    /// The three modes must produce three different surfaces from one DEM,
    /// and they must be consistent: ellipsoidal == orthometric + geoid-only.
    #[test]
    fn three_modes_produce_expected_surfaces() {
        let n = CESIUM_TILE_SIZE as usize;
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);

        let mut ortho = vec![100.0f64; n * n];
        apply_height_mode_to_grid(&TOKYO, &mut ortho, &geoid, HeightMode::Orthometric);
        assert!(ortho.iter().all(|&h| (h - 100.0).abs() < 1e-12));

        let mut only = vec![100.0f64; n * n];
        apply_height_mode_to_grid(&TOKYO, &mut only, &geoid, HeightMode::GeoidOnly);
        // Geoid surface over Japan: ~30-45 m, and independent of the DEM value.
        assert!(only.iter().all(|&h| h > 20.0 && h < 60.0));
        let mut only_from_other_dem = vec![-5000.0f64; n * n];
        apply_height_mode_to_grid(
            &TOKYO,
            &mut only_from_other_dem,
            &geoid,
            HeightMode::GeoidOnly,
        );
        assert_eq!(only, only_from_other_dem);

        let mut ellip = vec![100.0f64; n * n];
        apply_height_mode_to_grid(&TOKYO, &mut ellip, &geoid, HeightMode::Ellipsoidal);
        for i in 0..ellip.len() {
            assert!((ellip[i] - (ortho[i] + only[i])).abs() < 1e-12);
        }
    }

    #[test]
    fn xyz_three_modes_produce_expected_surfaces() {
        let size = 16u32;
        let len = (size * size) as usize;
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);

        let mut ortho = vec![100.0f64; len];
        apply_height_mode_to_xyz_grid(
            10,
            909,
            403,
            size,
            &mut ortho,
            &geoid,
            HeightMode::Orthometric,
        );
        assert!(ortho.iter().all(|&h| (h - 100.0).abs() < 1e-12));

        let mut only = vec![100.0f64; len];
        apply_height_mode_to_xyz_grid(10, 909, 403, size, &mut only, &geoid, HeightMode::GeoidOnly);
        assert!(only.iter().all(|&h| h > 20.0 && h < 60.0));

        let mut ellip = vec![100.0f64; len];
        apply_height_mode_to_xyz_grid(
            10,
            909,
            403,
            size,
            &mut ellip,
            &geoid,
            HeightMode::Ellipsoidal,
        );
        for i in 0..len {
            assert!((ellip[i] - (ortho[i] + only[i])).abs() < 1e-12);
        }
    }

    /// NaN DEM samples stay holes in ellipsoidal mode, but geoid-only ignores
    /// the DEM entirely so it still yields a defined surface there.
    #[test]
    fn nan_dem_handling_per_mode() {
        let n = CESIUM_TILE_SIZE as usize;
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);

        let mut ellip = vec![f64::NAN; n * n];
        apply_height_mode_to_grid(&TOKYO, &mut ellip, &geoid, HeightMode::Ellipsoidal);
        assert!(ellip.iter().all(|h| h.is_nan()));

        let mut only = vec![f64::NAN; n * n];
        apply_height_mode_to_grid(&TOKYO, &mut only, &geoid, HeightMode::GeoidOnly);
        assert!(only.iter().all(|h| h.is_finite()));
    }

    /// Outside the model's coverage the geoid-only surface is the same 0-fill
    /// that ellipsoidal mode uses — no fallback to another model.
    #[test]
    fn geoid_only_out_of_coverage_is_zero() {
        let bounds = GeodeticBounds {
            west: -160.0,
            south: 5.0,
            east: -150.0,
            north: 15.0,
        };
        let n = CESIUM_TILE_SIZE as usize;
        let mut grid = vec![42.0f64; n * n];
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        apply_height_mode_to_grid(&bounds, &mut grid, &geoid, HeightMode::GeoidOnly);
        assert!(grid.iter().all(|&h| h == 0.0));
    }
}
