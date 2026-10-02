//! Vertical-surface composition (orthometric DEM, geoid, or their sum).
//!
//! For each pixel in the output grid we sample the geoid height at that pixel's
//! (lng, lat) and combine it with the orthometric DEM height according to the
//! requested [`HeightMode`]. Samples where the model has no value fall back to
//! a geoid of 0 — partial-coverage tiles still render, with the uncovered area
//! at the orthometric height. That fill is an interim policy; each pass also
//! reports how much of the grid it hit (see [`GeoidCoverage`]), which the
//! handlers serve as `X-Geoid-Coverage`.
//!
//! The grid traversal is shared by all three modes and by both projections, so
//! the geoid-only surface is sampled at exactly the same points as the
//! ellipsoidal one, and the coverage-only functions (used on cache hits) count
//! exactly the points the rewrite did.

use super::geodetic::{CESIUM_TILE_SIZE, GeodeticBounds};
use super::geoid::{CoverageTally, Geoid, GeoidCoverage, HeightMode};

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

/// Visit every sample of a `grid_size × grid_size` grid spanning `bounds`
/// corner to corner, row-major and north first, as `(index, lng, lat)`.
///
/// The one traversal behind both the rewrite and the coverage-only pass, so
/// `X-Geoid-Coverage` describes exactly the points the geoid was applied at.
#[inline]
fn for_each_geodetic_sample(
    bounds: &GeodeticBounds,
    grid_size: usize,
    mut f: impl FnMut(usize, f64, f64),
) {
    for dst_y in 0..grid_size {
        let t_y = dst_y as f64 / (grid_size - 1) as f64;
        let lat = bounds.north - t_y * (bounds.north - bounds.south);
        for dst_x in 0..grid_size {
            let t_x = dst_x as f64 / (grid_size - 1) as f64;
            let lng = bounds.west + t_x * (bounds.east - bounds.west);
            f(dst_y * grid_size + dst_x, lng, lat);
        }
    }
}

/// Visit every pixel centre of a `tile_size × tile_size` Web Mercator XYZ
/// tile, row-major and north first, as `(index, lng, lat)`. Latitude is
/// mercator-Y uniform (not lat uniform), so it is recomputed per row.
#[inline]
fn for_each_xyz_sample(z: u8, x: u32, y: u32, tile_size: u32, mut f: impl FnMut(usize, f64, f64)) {
    let n = tile_size as usize;
    // Pre-compute per-column longitudes (lon is column-linear).
    let lons: Vec<f64> = (0..tile_size)
        .map(|px| xyz_pixel_lon(z, x, tile_size, px))
        .collect();
    for py in 0..tile_size {
        let lat = xyz_pixel_lat(z, y, tile_size, py);
        let row_off = (py as usize) * n;
        for (px, &lng) in lons.iter().enumerate() {
            f(row_off + px, lng, lat);
        }
    }
}

/// Rewrite one sample onto the `mode` surface, recording whether the geoid
/// model had a value there. Where it has none the geoid is taken as 0 (the
/// interim policy documented on [`GeoidCoverage`]).
///
/// The geoid is evaluated even where an ellipsoidal sample stays a NaN hole,
/// so the tally depends only on the grid's positions — a cache hit can then
/// recompute it without the DEM.
#[inline]
fn apply_at(
    grid: &mut [f64],
    idx: usize,
    lng: f64,
    lat: f64,
    geoid: &Geoid,
    mode: HeightMode,
    tally: &mut CoverageTally,
) {
    let geoid_height = geoid.height(lng, lat);
    let covered = geoid_height.is_finite();
    tally.record(covered);
    let ortho = grid[idx];
    if ortho.is_nan() && mode == HeightMode::Ellipsoidal {
        return;
    }
    grid[idx] = combine(mode, ortho, if covered { geoid_height } else { 0.0 });
}

/// Rewrite an orthometric elevation grid (in-place) into the surface selected
/// by `mode`, returning how much of the grid the geoid model covered (`None`
/// for [`HeightMode::Orthometric`], which uses no geoid).
///
/// The grid is 65×65 row-major, north-first (matching
/// `fetch_geodetic_tile_elevations_with_halo`).
pub fn apply_height_mode_to_grid(
    bounds: &GeodeticBounds,
    grid: &mut [f64],
    geoid: &Geoid,
    mode: HeightMode,
) -> Option<GeoidCoverage> {
    apply_height_mode_to_grid_sized(bounds, grid, geoid, CESIUM_TILE_SIZE as usize, mode)
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
) -> Option<GeoidCoverage> {
    debug_assert_eq!(grid.len(), grid_size * grid_size);

    // Orthometric is the DEM as-is: no geoid sampling at all.
    if mode == HeightMode::Orthometric {
        return None;
    }

    let mut tally = CoverageTally::default();
    for_each_geodetic_sample(bounds, grid_size, |idx, lng, lat| {
        apply_at(grid, idx, lng, lat, geoid, mode, &mut tally);
    });
    Some(tally.coverage())
}

/// Rewrite an orthometric elevation grid (in-place) into the surface selected
/// by `mode`, for a `tile_size × tile_size` Web Mercator XYZ tile, returning
/// the geoid coverage of its pixel centres (`None` for orthometric).
///
/// The grid is row-major with row 0 at the tile's north edge and column 0 at
/// the west edge (matching the layout returned by `DemProvider`).
/// Out-of-coverage geoid samples fall back to 0.
pub fn apply_height_mode_to_xyz_grid(
    z: u8,
    x: u32,
    y: u32,
    tile_size: u32,
    grid: &mut [f64],
    geoid: &Geoid,
    mode: HeightMode,
) -> Option<GeoidCoverage> {
    debug_assert_eq!(grid.len(), (tile_size as usize) * (tile_size as usize));

    if mode == HeightMode::Orthometric {
        return None;
    }

    let mut tally = CoverageTally::default();
    for_each_xyz_sample(z, x, y, tile_size, |idx, lng, lat| {
        apply_at(grid, idx, lng, lat, geoid, mode, &mut tally);
    });
    Some(tally.coverage())
}

/// Geoid coverage of the 65×65 quantized-mesh grid over `bounds`, without
/// touching any elevations — what [`apply_height_mode_to_grid`] reports, for
/// a response served from cache.
pub fn geoid_coverage_of_grid(bounds: &GeodeticBounds, geoid: &Geoid) -> GeoidCoverage {
    let mut tally = CoverageTally::default();
    for_each_geodetic_sample(bounds, CESIUM_TILE_SIZE as usize, |_, lng, lat| {
        tally.record(geoid.height(lng, lat).is_finite());
    });
    tally.coverage()
}

/// Geoid coverage of an XYZ tile's pixel centres, without touching any
/// elevations — what [`apply_height_mode_to_xyz_grid`] reports, for a
/// response served from cache.
pub fn geoid_coverage_of_xyz_grid(
    z: u8,
    x: u32,
    y: u32,
    tile_size: u32,
    geoid: &Geoid,
) -> GeoidCoverage {
    let mut tally = CoverageTally::default();
    for_each_xyz_sample(z, x, y, tile_size, |_, lng, lat| {
        tally.record(geoid.height(lng, lat).is_finite());
    });
    tally.coverage()
}

#[cfg(test)]
mod tests {
    use super::super::geodetic::geodetic_tms_bounds;
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

    // GSIGEO2011 fixtures, all inside the model's coverage bbox (so none of
    // them is a 404): inland Nagano, a z8 tile around Izu that is part land,
    // part open sea, and open sea south of Japan / Seoul, where the model has
    // no value at all.
    const XYZ_FULL: (u8, u32, u32) = (10, 905, 401);
    const XYZ_PARTIAL: (u8, u32, u32) = (8, 227, 102);
    const XYZ_NONE_SEA: (u8, u32, u32) = (10, 918, 422);
    const XYZ_NONE_KOREA: (u8, u32, u32) = (10, 873, 396);
    const MESH_FULL: (u8, u32, u32) = (9, 905, 358);
    const MESH_PARTIAL: (u8, u32, u32) = (9, 908, 352);
    const MESH_NONE: (u8, u32, u32) = (9, 918, 341);

    fn xyz_coverage(t: (u8, u32, u32), mode: HeightMode) -> Option<GeoidCoverage> {
        let size = 256u32;
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        let mut grid = vec![10.0f64; (size * size) as usize];
        apply_height_mode_to_xyz_grid(t.0, t.1, t.2, size, &mut grid, &geoid, mode)
    }

    fn mesh_coverage(t: (u8, u32, u32), mode: HeightMode) -> Option<GeoidCoverage> {
        let n = CESIUM_TILE_SIZE as usize;
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        let bounds = geodetic_tms_bounds(t.0, t.1, t.2);
        let mut grid = vec![10.0f64; n * n];
        apply_height_mode_to_grid(&bounds, &mut grid, &geoid, mode)
    }

    #[test]
    fn coverage_full_partial_none() {
        for mode in [HeightMode::Ellipsoidal, HeightMode::GeoidOnly] {
            assert_eq!(xyz_coverage(XYZ_FULL, mode), Some(GeoidCoverage::Full));
            assert_eq!(
                xyz_coverage(XYZ_PARTIAL, mode),
                Some(GeoidCoverage::Partial)
            );
            assert_eq!(xyz_coverage(XYZ_NONE_SEA, mode), Some(GeoidCoverage::None));
            assert_eq!(
                xyz_coverage(XYZ_NONE_KOREA, mode),
                Some(GeoidCoverage::None)
            );
            assert_eq!(mesh_coverage(MESH_FULL, mode), Some(GeoidCoverage::Full));
            assert_eq!(
                mesh_coverage(MESH_PARTIAL, mode),
                Some(GeoidCoverage::Partial)
            );
            assert_eq!(mesh_coverage(MESH_NONE, mode), Some(GeoidCoverage::None));
        }
    }

    #[test]
    fn orthometric_reports_no_coverage() {
        for t in [XYZ_FULL, XYZ_PARTIAL, XYZ_NONE_SEA] {
            assert_eq!(xyz_coverage(t, HeightMode::Orthometric), None);
        }
        for t in [MESH_FULL, MESH_PARTIAL, MESH_NONE] {
            assert_eq!(mesh_coverage(t, HeightMode::Orthometric), None);
        }
    }

    /// The coverage-only pass a cache hit uses must agree with the pass that
    /// rendered the tile.
    #[test]
    fn coverage_only_pass_matches_the_rewrite() {
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        for t in [XYZ_FULL, XYZ_PARTIAL, XYZ_NONE_SEA, XYZ_NONE_KOREA] {
            assert_eq!(
                Some(geoid_coverage_of_xyz_grid(t.0, t.1, t.2, 256, &geoid)),
                xyz_coverage(t, HeightMode::Ellipsoidal),
                "{t:?}"
            );
        }
        for t in [MESH_FULL, MESH_PARTIAL, MESH_NONE] {
            assert_eq!(
                Some(geoid_coverage_of_grid(
                    &geodetic_tms_bounds(t.0, t.1, t.2),
                    &geoid
                )),
                mesh_coverage(t, HeightMode::Ellipsoidal),
                "{t:?}"
            );
        }
    }

    /// Coverage depends on positions only: a NaN DEM sample (left as a hole in
    /// ellipsoidal mode) still counts, so the cached-hit recompute, which has
    /// no DEM, gives the same answer.
    #[test]
    fn coverage_ignores_dem_holes() {
        let size = 64u32;
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        let (z, x, y) = XYZ_PARTIAL;
        let mut grid = vec![f64::NAN; (size * size) as usize];
        let c = apply_height_mode_to_xyz_grid(
            z,
            x,
            y,
            size,
            &mut grid,
            &geoid,
            HeightMode::Ellipsoidal,
        );
        assert!(grid.iter().all(|h| h.is_nan()));
        assert_eq!(c, Some(geoid_coverage_of_xyz_grid(z, x, y, size, &geoid)));
    }

    /// Reporting coverage must not move a single served height: every sample
    /// is still `ortho + (geoid, or 0 where the model has no value)`, NaN DEM
    /// samples stay holes in ellipsoidal mode, and geoid-only ignores the DEM.
    #[test]
    fn served_heights_are_unchanged_on_a_partial_tile() {
        let size = 256u32;
        let geoid = Geoid::load(GeoidModel::Gsigeo2011);
        let (z, x, y) = XYZ_PARTIAL;
        let dem: Vec<f64> = (0..size * size)
            .map(|i| {
                if i % 97 == 0 {
                    f64::NAN
                } else {
                    (i % 1000) as f64
                }
            })
            .collect();
        for mode in HeightMode::all() {
            let mut got = dem.clone();
            apply_height_mode_to_xyz_grid(z, x, y, size, &mut got, &geoid, *mode);
            for py in 0..size {
                let lat = xyz_pixel_lat(z, y, size, py);
                for px in 0..size {
                    let lng = xyz_pixel_lon(z, x, size, px);
                    let idx = (py * size + px) as usize;
                    let g = geoid.height(lng, lat);
                    let g = if g.is_finite() { g } else { 0.0 };
                    let want = match mode {
                        HeightMode::Orthometric => dem[idx],
                        HeightMode::GeoidOnly => g,
                        HeightMode::Ellipsoidal => dem[idx] + g,
                    };
                    assert!(
                        got[idx].to_bits() == want.to_bits()
                            || (want.is_nan() && got[idx].is_nan()),
                        "{mode} ({px},{py}): got {} want {want}",
                        got[idx]
                    );
                }
            }
        }
    }
}
