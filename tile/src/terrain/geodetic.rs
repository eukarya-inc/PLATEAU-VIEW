//! Geodetic TMS coordinate transformations and resampling
//!
//! This module handles the conversion between Global-geodetic TMS (used by Cesium)
//! and Web Mercator XYZ (used by elevation data sources).

use std::collections::HashMap;
use std::f64::consts::PI;
use std::time::Instant;

use super::dem::{DemError, DemProvider};

/// Output grid size for a Cesium terrain tile (65x65, matches heightmap-1.0).
pub const CESIUM_TILE_SIZE: u32 = 65;

/// Maximum latitude supported by Web Mercator projection.
/// Derived from: arctan(sinh(π)) ≈ 85.05112878°
/// This ensures the map is a square (y range equals x range in projected coordinates).
const WEB_MERCATOR_MAX_LAT: f64 = 85.05112878;

/// Geographic bounds for a geodetic TMS tile
#[derive(Debug, Clone, Copy)]
pub struct GeodeticBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// XYZ tile coordinate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XyzTile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

/// Timing information for geodetic tile fetch operation
#[derive(Debug, Clone, Default)]
pub struct GeodeticFetchTiming {
    /// Number of XYZ tiles fetched
    pub tiles_fetched: u32,
    /// Time spent fetching XYZ tiles (network I/O) in milliseconds
    pub xyz_fetch_ms: f64,
    /// Time spent resampling to geodetic grid in milliseconds
    pub resample_ms: f64,
}

/// Result of geodetic tile fetch including elevations, per-tile etags, and timing.
#[derive(Debug, Clone, Default)]
pub struct GeodeticFetchResult {
    /// Elevation data (65x65 grid)
    pub elevations: Vec<f64>,
    /// Upstream ETag fragments, sorted, for cache-key composition.
    pub source_etags: Vec<String>,
    /// Timing information
    pub timing: GeodeticFetchTiming,
}

/// Calculate the geographic bounds of a Global-geodetic TMS tile
///
/// Global-geodetic TMS:
/// - x range: 0 to 2^(z+1) - 1 (longitude: -180 to 180)
/// - y range: 0 to 2^z - 1 (latitude: -90 to 90, Y=0 is south)
pub fn geodetic_tms_bounds(z: u8, x: u32, y: u32) -> GeodeticBounds {
    let n_x = 1u32 << (z + 1); // 2^(z+1) tiles in X
    let n_y = 1u32 << z; // 2^z tiles in Y

    let west = (x as f64 / n_x as f64) * 360.0 - 180.0;
    let east = ((x + 1) as f64 / n_x as f64) * 360.0 - 180.0;
    let south = (y as f64 / n_y as f64) * 180.0 - 90.0;
    let north = ((y + 1) as f64 / n_y as f64) * 180.0 - 90.0;

    GeodeticBounds {
        west,
        south,
        east,
        north,
    }
}

/// Calculate XYZ tiles that intersect with geodetic bounds
///
/// Returns a list of XYZ tiles at zoom level `z` that cover the given bounds.
/// The bounds are clamped to Web Mercator's valid latitude range.
///
/// If `include_adjacent` is true, the function also includes adjacent tiles at
/// boundaries to ensure seamless tiling. This is needed when geodetic tile boundaries
/// align exactly with XYZ tile boundaries.
pub fn xyz_tiles_for_bounds(
    z: u8,
    bounds: &GeodeticBounds,
    include_adjacent: bool,
) -> Vec<XyzTile> {
    let n = 1u32 << z;

    // Clamp latitude to Web Mercator range
    let south = bounds
        .south
        .clamp(-WEB_MERCATOR_MAX_LAT, WEB_MERCATOR_MAX_LAT);
    let north = bounds
        .north
        .clamp(-WEB_MERCATOR_MAX_LAT, WEB_MERCATOR_MAX_LAT);

    // If bounds are entirely outside Web Mercator range, return empty
    if south >= WEB_MERCATOR_MAX_LAT || north <= -WEB_MERCATOR_MAX_LAT {
        return Vec::new();
    }

    // Calculate X tile range (as floats for boundary detection)
    let x_min_f = ((bounds.west + 180.0) / 360.0) * n as f64;
    let x_max_f = ((bounds.east + 180.0) / 360.0) * n as f64;
    let x_min = x_min_f.floor() as i32;
    let x_max = x_max_f.ceil() as i32 - 1;

    // Calculate Y tile range (note: Y increases southward in XYZ)
    let y_north = lat_to_xyz_y(north, n);
    let y_south = lat_to_xyz_y(south, n);
    let y_min = y_north.min(y_south);
    let y_max = y_north.max(y_south);

    // Smart adjacent tile inclusion: only extend in directions where the geodetic
    // boundary aligns with an XYZ tile boundary (within tolerance)
    let (x_min_final, x_max_final, y_min_final, y_max_final) = if include_adjacent {
        const BOUNDARY_TOLERANCE: f64 = 0.001;

        // Check if west boundary aligns with XYZ tile boundary
        let west_on_boundary = x_min_f.fract().abs() < BOUNDARY_TOLERANCE;
        // Check if east boundary aligns with XYZ tile boundary
        let east_on_boundary = x_max_f.fract().abs() < BOUNDARY_TOLERANCE
            || (1.0 - x_max_f.fract()).abs() < BOUNDARY_TOLERANCE;

        // For Y, we need to check in projected coordinates
        // Use the actual tile Y values to detect boundaries
        let y_north_f = lat_to_xyz_y_float(north, n);
        let y_south_f = lat_to_xyz_y_float(south, n);
        let north_on_boundary = y_north_f.fract().abs() < BOUNDARY_TOLERANCE;
        let south_on_boundary = y_south_f.fract().abs() < BOUNDARY_TOLERANCE
            || (1.0 - y_south_f.fract()).abs() < BOUNDARY_TOLERANCE;

        (
            if west_on_boundary {
                (x_min - 1).max(0)
            } else {
                x_min.max(0)
            },
            if east_on_boundary {
                (x_max + 1).min(n as i32 - 1)
            } else {
                x_max.min(n as i32 - 1)
            },
            if north_on_boundary {
                y_min.saturating_sub(1)
            } else {
                y_min
            },
            if south_on_boundary {
                (y_max + 1).min(n - 1)
            } else {
                y_max
            },
        )
    } else {
        (x_min.max(0), x_max.min(n as i32 - 1), y_min, y_max)
    };

    let mut tiles = Vec::new();
    for x in x_min_final..=x_max_final {
        for y in y_min_final..=y_max_final {
            if y < n {
                tiles.push(XyzTile { z, x: x as u32, y });
            }
        }
    }

    tiles
}

/// Convert latitude to XYZ Y tile coordinate (float version for boundary detection)
fn lat_to_xyz_y_float(lat: f64, n: u32) -> f64 {
    let lat_rad = lat.to_radians();
    let y = (1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n as f64;
    y.clamp(0.0, (n - 1) as f64)
}

/// Convert latitude to XYZ Y tile coordinate
fn lat_to_xyz_y(lat: f64, n: u32) -> u32 {
    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n as f64).floor() as u32;
    y.min(n.saturating_sub(1))
}

/// Convert XYZ tile Y coordinate to latitude
fn xyz_y_to_lat(y: f64, n: u32) -> f64 {
    let lat_rad = (PI * (1.0 - 2.0 * y / n as f64)).sinh().atan();
    lat_rad.to_degrees()
}

/// Fetch elevation data for a geodetic TMS tile by reprojecting from XYZ tiles
///
/// This function:
/// 1. Calculates the geodetic bounds of the TMS tile
/// 2. Finds all XYZ tiles that intersect those bounds
/// 3. Fetches elevation data from all intersecting XYZ tiles in parallel
/// 4. Resamples the data to the Cesium 65x65 grid in geodetic coordinates
///
/// Returns a `GeodeticFetchResult` containing both the elevation data and timing information.
pub async fn fetch_geodetic_tile_elevations(
    provider: &dyn DemProvider,
    z: u8,
    geo_x: u32,
    geo_y: u32,
    xyz_tile_size: u32,
) -> Result<GeodeticFetchResult, DemError> {
    let bounds = geodetic_tms_bounds(z, geo_x, geo_y);
    // For zooms above the upstream DEM's max, fall back to parent XYZ
    // tiles at `dem_max` and let the bilinear sampler in
    // `resample_to_geodetic_grid` upsample the relevant sub-region (per
    // stralift's per-source upsampling). The geodetic bounds stay at the
    // requested output `z` so the output 65×65 grid still covers exactly
    // the requested geodetic tile.
    let fetch_z = z.min(provider.max_zoom());
    // Include adjacent tiles for seamless boundaries
    let xyz_tiles = xyz_tiles_for_bounds(fetch_z, &bounds, true);

    if xyz_tiles.is_empty() {
        // No XYZ tiles cover this geodetic tile (e.g., polar regions)
        return Ok(GeodeticFetchResult {
            elevations: vec![f64::NAN; (CESIUM_TILE_SIZE * CESIUM_TILE_SIZE) as usize],
            source_etags: Vec::new(),
            timing: GeodeticFetchTiming::default(),
        });
    }

    let tiles_fetched = xyz_tiles.len() as u32;

    // Fetch all XYZ tiles in parallel (already at `fetch_z` per the
    // upsample fallback above)
    let fetch_start = Instant::now();
    let fetch_futures: Vec<_> = xyz_tiles
        .iter()
        .map(|tile| async move {
            let data = provider
                .get_tile_elevations(tile.z, tile.x, tile.y, xyz_tile_size)
                .await;
            (*tile, data)
        })
        .collect();

    let results = futures::future::join_all(fetch_futures).await;
    let xyz_fetch_ms = fetch_start.elapsed().as_secs_f64() * 1000.0;

    // Collect successful results into a map, aggregate etags.
    let mut tile_data: HashMap<XyzTile, Vec<f64>> = HashMap::new();
    let mut source_etags: Vec<String> = Vec::new();
    for (tile, result) in results {
        if let Ok(data) = result {
            if let Some(etag) = data.etag {
                source_etags.push(etag);
            }
            tile_data.insert(tile, data.elevations);
        }
    }
    source_etags.sort();
    source_etags.dedup();

    // Resample to geodetic grid using `fetch_z` since the tile_data keys
    // and lookups are at that zoom (parent tiles when upsampling).
    let resample_start = Instant::now();
    let elevations = resample_to_geodetic_grid(&bounds, &tile_data, xyz_tile_size, fetch_z)?;
    let resample_ms = resample_start.elapsed().as_secs_f64() * 1000.0;

    Ok(GeodeticFetchResult {
        elevations,
        source_etags,
        timing: GeodeticFetchTiming {
            tiles_fetched,
            xyz_fetch_ms,
            resample_ms,
        },
    })
}

/// Resample XYZ tile data to a geodetic grid (65x65)
///
/// For each pixel in the output grid:
/// 1. Calculate its geographic coordinates (lng, lat)
/// 2. Find which XYZ tile contains that point
/// 3. Sample the elevation from that tile
///
/// IMPORTANT: At tile boundaries, we need to ensure adjacent geodetic tiles sample
/// the SAME elevation value at shared edges. The approach:
/// - West edge (dst_x=0): This is also the east edge of the tile to the west.
///   We nudge the coordinate slightly WEST so both tiles sample from the western XYZ tile.
/// - North edge (dst_y=0): This is also the south edge of the tile to the north.
///   We nudge the coordinate slightly NORTH so both tiles sample from the northern XYZ tile.
/// - East/South edges are not adjusted, they will naturally sample correctly.
fn resample_to_geodetic_grid(
    bounds: &GeodeticBounds,
    tile_data: &HashMap<XyzTile, Vec<f64>>,
    xyz_tile_size: u32,
    z: u8,
) -> Result<Vec<f64>, DemError> {
    let output_size = CESIUM_TILE_SIZE as usize;
    let mut result = Vec::with_capacity(output_size * output_size);
    let n = 1u32 << z;

    // Small epsilon to nudge boundary coordinates
    // This ensures adjacent tiles sample from the same XYZ tile at boundaries
    let lng_epsilon = (bounds.east - bounds.west) * 1e-9;
    let lat_epsilon = (bounds.north - bounds.south) * 1e-9;

    for dst_y in 0..output_size {
        for dst_x in 0..output_size {
            // Calculate geographic coordinates for this pixel
            // Note: Cesium expects data ordered from north to south
            let t_x = dst_x as f64 / (output_size - 1) as f64;
            let t_y = dst_y as f64 / (output_size - 1) as f64;

            let mut lng = bounds.west + t_x * (bounds.east - bounds.west);
            let mut lat = bounds.north - t_y * (bounds.north - bounds.south); // north to south

            // BOUNDARY SAMPLING STRATEGY:
            // When geodetic tile boundaries align with XYZ tile boundaries, we must ensure
            // that adjacent geodetic tiles sample from the SAME XYZ tile at the SAME position.
            //
            // Convention: Always nudge boundary coordinates WEST and NORTH.
            // This ensures tile A's east edge and tile B's west edge both sample from
            // the same position in the western XYZ tile.
            if dst_x == 0 || dst_x == output_size - 1 {
                lng -= lng_epsilon;
            }
            if dst_y == 0 || dst_y == output_size - 1 {
                lat += lat_epsilon;
            }

            // Check if lat is within Web Mercator range
            if !(-WEB_MERCATOR_MAX_LAT..=WEB_MERCATOR_MAX_LAT).contains(&lat) {
                result.push(f64::NAN);
                continue;
            }

            // Find which XYZ tile contains this point
            let xyz_x = (((lng + 180.0) / 360.0) * n as f64).floor() as u32;
            let xyz_x = xyz_x.min(n.saturating_sub(1));

            let lat_rad = lat.to_radians();
            let xyz_y = ((1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n as f64).floor() as u32;
            let xyz_y = xyz_y.min(n.saturating_sub(1));

            let xyz_tile = XyzTile {
                z,
                x: xyz_x,
                y: xyz_y,
            };

            // Get elevation from this tile using bilinear interpolation
            let elevation = if let Some(data) = tile_data.get(&xyz_tile) {
                sample_from_xyz_tile(data, xyz_tile_size, &xyz_tile, lng, lat, n)
            } else {
                f64::NAN
            };

            result.push(elevation);
        }
    }

    Ok(result)
}

/// Sample elevation from an XYZ tile at a given geographic coordinate using bilinear interpolation
fn sample_from_xyz_tile(
    data: &[f64],
    tile_size: u32,
    tile: &XyzTile,
    lng: f64,
    lat: f64,
    n: u32,
) -> f64 {
    // Calculate the tile's geographic bounds
    let tile_west = (tile.x as f64 / n as f64) * 360.0 - 180.0;
    let tile_east = ((tile.x + 1) as f64 / n as f64) * 360.0 - 180.0;
    let tile_north = xyz_y_to_lat(tile.y as f64, n);
    let tile_south = xyz_y_to_lat((tile.y + 1) as f64, n);

    // Calculate normalized position within the tile (0.0 to 1.0)
    let px = (lng - tile_west) / (tile_east - tile_west);
    let py = (tile_north - lat) / (tile_north - tile_south); // Y increases southward

    // Convert to pixel coordinates
    let src_x = px * (tile_size - 1) as f64;
    let src_y = py * (tile_size - 1) as f64;

    bilinear_interpolate(data, tile_size, src_x, src_y)
}

/// Bilinear interpolation for elevation sampling
fn bilinear_interpolate(elevations: &[f64], width: u32, x: f64, y: f64) -> f64 {
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(width - 1);

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let get_elevation = |px: u32, py: u32| -> f64 {
        let idx = (py * width + px) as usize;
        elevations.get(idx).copied().unwrap_or(f64::NAN)
    };

    let v00 = get_elevation(x0, y0);
    let v10 = get_elevation(x1, y0);
    let v01 = get_elevation(x0, y1);
    let v11 = get_elevation(x1, y1);

    // Handle NaN values
    let values = [v00, v10, v01, v11];
    let valid_values: Vec<f64> = values.iter().filter(|v| !v.is_nan()).copied().collect();

    if valid_values.is_empty() {
        return f64::NAN;
    }

    if valid_values.len() < 4 {
        // If some values are NaN, use average of valid values
        return valid_values.iter().sum::<f64>() / valid_values.len() as f64;
    }

    // Standard bilinear interpolation
    let v0 = v00 * (1.0 - fx) + v10 * fx;
    let v1 = v01 * (1.0 - fx) + v11 * fx;

    v0 * (1.0 - fy) + v1 * fy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuji_adjacent_tiles_z12() {
        // Mt. Fuji is around 138.7E, 35.4N
        // At zoom 12, geodetic TMS has 8192 x-tiles, 4096 y-tiles
        // x = (138.7 + 180) / 360 * 8192 ≈ 7253
        // y = (35.4 + 90) / 180 * 4096 ≈ 2853

        let z = 12u8;
        let n_x = 1u32 << (z + 1); // 8192
        let n_y = 1u32 << z; // 4096
        let x = ((138.7 + 180.0) / 360.0 * n_x as f64).floor() as u32;
        let y = ((35.4 + 90.0) / 180.0 * n_y as f64).floor() as u32;

        println!("z={z}, geodetic TMS tile: x={x}, y={y}");

        let bounds1 = geodetic_tms_bounds(z, x, y);
        let bounds2 = geodetic_tms_bounds(z, x + 1, y);

        println!(
            "Tile {}: west={}, east={}, south={}, north={}",
            x, bounds1.west, bounds1.east, bounds1.south, bounds1.north
        );
        println!(
            "Tile {}: west={}, east={}, south={}, north={}",
            x + 1,
            bounds2.west,
            bounds2.east,
            bounds2.south,
            bounds2.north
        );

        let xyz1 = xyz_tiles_for_bounds(z, &bounds1, false);
        let xyz2 = xyz_tiles_for_bounds(z, &bounds2, false);

        println!("XYZ tiles for {x}: {xyz1:?}");
        println!("XYZ tiles for {}: {:?}", x + 1, xyz2);

        // At zoom 12, geodetic tiles are half the width of XYZ tiles
        // So two adjacent geodetic tiles might share the same XYZ tile
        // But they should sample different parts of that tile
    }

    #[test]
    fn test_tile_boundary_consistency() {
        // Test that adjacent tiles share the same edge values
        // Using the problematic tiles: 12/7251/1243 and 12/7252/1243
        // Note: These are TMS Y coordinates (from screenshot), need to verify

        let z = 12u8;
        let geo_x1 = 7251u32;
        let geo_x2 = 7252u32;
        let geo_y = 1243u32;

        let bounds1 = geodetic_tms_bounds(z, geo_x1, geo_y);
        let bounds2 = geodetic_tms_bounds(z, geo_x2, geo_y);

        println!(
            "Left tile (7251): west={:.6}, east={:.6}",
            bounds1.west, bounds1.east
        );
        println!(
            "Right tile (7252): west={:.6}, east={:.6}",
            bounds2.west, bounds2.east
        );
        println!("Boundary should be at: {:.6}", bounds1.east);

        // The right edge of bounds1 should equal the left edge of bounds2
        let boundary_match = (bounds1.east - bounds2.west).abs() < 1e-10;
        println!(
            "Bounds match: {} (diff={})",
            boundary_match,
            bounds1.east - bounds2.west
        );
        assert!(boundary_match, "Tile bounds should be continuous");

        // Check which XYZ tiles are used
        let xyz1 = xyz_tiles_for_bounds(z, &bounds1, false);
        let xyz2 = xyz_tiles_for_bounds(z, &bounds2, false);
        println!("XYZ tiles for 7251: {xyz1:?}");
        println!("XYZ tiles for 7252: {xyz2:?}");

        // Check if they share any XYZ tiles (they should at the boundary)
        let shared: Vec<_> = xyz1.iter().filter(|t| xyz2.contains(t)).collect();
        println!("Shared XYZ tiles: {shared:?}");

        // KEY INSIGHT: The boundary longitude (138.691406) is exactly on an XYZ tile boundary!
        // Left tile samples from XYZ 3625's right edge, right tile samples from XYZ 3626's left edge
        // These are DIFFERENT source tiles, so they may have slightly different values!

        let n = 1u32 << z;
        let boundary_lng = bounds1.east;
        let xyz_x_float = (boundary_lng + 180.0) / 360.0 * n as f64;
        println!(
            "XYZ X for boundary: {} (fractional part: {})",
            xyz_x_float,
            xyz_x_float.fract()
        );

        // If fractional part is close to 0 or 1, we're on an XYZ tile boundary
        let frac = xyz_x_float.fract();
        let on_xyz_boundary = !(0.001..=0.999).contains(&frac);
        println!("On XYZ tile boundary: {on_xyz_boundary}");

        // Simulate what happens when sampling
        // For left tile (7251), at east edge (dst_x=64):
        let left_east_lng = bounds1.east; // 138.691406
        let left_xyz_x = ((left_east_lng + 180.0) / 360.0 * n as f64).floor() as u32;
        println!("Left tile east edge -> XYZ x={left_xyz_x}");

        // For right tile (7252), at west edge (dst_x=0):
        let right_west_lng = bounds2.west; // 138.691406
        let right_xyz_x = ((right_west_lng + 180.0) / 360.0 * n as f64).floor() as u32;
        println!("Right tile west edge -> XYZ x={right_xyz_x}");

        // With nudging (epsilon):
        let epsilon = (bounds1.east - bounds1.west) * 1e-9;
        let left_nudged = left_east_lng - epsilon;
        let left_xyz_x_nudged = ((left_nudged + 180.0) / 360.0 * n as f64).floor() as u32;
        println!("Left tile east edge (nudged) -> XYZ x={left_xyz_x_nudged}");
    }

    #[test]
    fn test_geodetic_tms_bounds_z0() {
        // z=0: 2 tiles in X, 1 tile in Y
        let bounds = geodetic_tms_bounds(0, 0, 0);
        assert!((bounds.west - (-180.0)).abs() < 1e-10);
        assert!((bounds.east - 0.0).abs() < 1e-10);
        assert!((bounds.south - (-90.0)).abs() < 1e-10);
        assert!((bounds.north - 90.0).abs() < 1e-10);

        let bounds = geodetic_tms_bounds(0, 1, 0);
        assert!((bounds.west - 0.0).abs() < 1e-10);
        assert!((bounds.east - 180.0).abs() < 1e-10);
    }

    #[test]
    fn test_geodetic_tms_bounds_z1() {
        // z=1: 4 tiles in X, 2 tiles in Y
        let bounds = geodetic_tms_bounds(1, 0, 0);
        assert!((bounds.west - (-180.0)).abs() < 1e-10);
        assert!((bounds.east - (-90.0)).abs() < 1e-10);
        assert!((bounds.south - (-90.0)).abs() < 1e-10);
        assert!((bounds.north - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_xyz_tiles_for_bounds() {
        // A small bounds around Tokyo
        let bounds = GeodeticBounds {
            west: 139.0,
            east: 140.0,
            south: 35.0,
            north: 36.0,
        };

        let tiles = xyz_tiles_for_bounds(8, &bounds, false);
        assert!(!tiles.is_empty());

        // All tiles should have z=8
        for tile in &tiles {
            assert_eq!(tile.z, 8);
        }
    }

    #[test]
    fn test_xyz_tiles_for_bounds_include_adjacent() {
        // Test that include_adjacent=true only adds tiles when boundaries align with XYZ tiles
        let bounds = GeodeticBounds {
            west: 139.0,
            east: 140.0,
            south: 35.0,
            north: 36.0,
        };

        let tiles_without = xyz_tiles_for_bounds(8, &bounds, false);
        let tiles_with = xyz_tiles_for_bounds(8, &bounds, true);

        // All original tiles should be included
        for tile in &tiles_without {
            assert!(tiles_with.contains(tile));
        }

        // With smart adjacent logic, tiles_with >= tiles_without
        // (more tiles only when boundaries align with XYZ tile boundaries)
        assert!(tiles_with.len() >= tiles_without.len());
    }

    #[test]
    fn test_xyz_tiles_for_bounds_boundary_aligned() {
        // Test with bounds that align with XYZ tile boundaries
        // At z=8, each tile is 360/256 = 1.40625 degrees wide in longitude
        // Tile 0 covers -180 to -178.59375, Tile 1 covers -178.59375 to -177.1875, etc.
        // We want a boundary that aligns exactly with a tile edge

        // West boundary at exactly -180 + 1.40625 * 100 = -39.375 (tile boundary)
        let z = 8u8;
        let n = 1u32 << z; // 256
        let tile_width = 360.0 / n as f64;

        // Create bounds where west edge is on a tile boundary
        let west = -180.0 + tile_width * 100.0; // Exactly on tile boundary
        let bounds = GeodeticBounds {
            west,
            east: west + 0.5, // Small width, doesn't align
            south: 35.0,
            north: 36.0,
        };

        let tiles_without = xyz_tiles_for_bounds(z, &bounds, false);
        let tiles_with = xyz_tiles_for_bounds(z, &bounds, true);

        // West edge aligns, so should have extra tile on west side
        assert!(
            tiles_with.len() > tiles_without.len(),
            "Should add adjacent tile when boundary aligns. without={}, with={}",
            tiles_without.len(),
            tiles_with.len()
        );
    }

    #[test]
    fn test_xyz_tiles_for_polar_region() {
        // Bounds entirely outside Web Mercator range
        let bounds = GeodeticBounds {
            west: 0.0,
            east: 10.0,
            south: 86.0,
            north: 90.0,
        };

        let tiles = xyz_tiles_for_bounds(8, &bounds, false);
        assert!(tiles.is_empty());
    }

    #[test]
    fn test_adjacent_geodetic_tiles_share_xyz_tiles() {
        // Test that adjacent geodetic tiles share XYZ tiles when include_adjacent=true
        // Using the problematic tiles from the boundary issue: 12/7251/1243 and 12/7252/1243
        let z = 12u8;
        let bounds1 = geodetic_tms_bounds(z, 7251, 1243);
        let bounds2 = geodetic_tms_bounds(z, 7252, 1243);

        // Without include_adjacent, they don't share XYZ tiles
        let xyz1_without = xyz_tiles_for_bounds(z, &bounds1, false);
        let xyz2_without = xyz_tiles_for_bounds(z, &bounds2, false);
        let shared_without: Vec<_> = xyz1_without
            .iter()
            .filter(|t| xyz2_without.contains(t))
            .collect();
        assert!(
            shared_without.is_empty(),
            "Without adjacent, no shared tiles"
        );

        // With include_adjacent, they share XYZ tiles at the boundary
        let xyz1_with = xyz_tiles_for_bounds(z, &bounds1, true);
        let xyz2_with = xyz_tiles_for_bounds(z, &bounds2, true);
        let shared_with: Vec<_> = xyz1_with.iter().filter(|t| xyz2_with.contains(t)).collect();
        assert!(!shared_with.is_empty(), "With adjacent, should share tiles");
    }

    #[test]
    fn test_bilinear_interpolate() {
        // Simple 2x2 grid
        let elevations = vec![0.0, 100.0, 100.0, 200.0];

        // Center should be average
        let center = bilinear_interpolate(&elevations, 2, 0.5, 0.5);
        assert!((center - 100.0).abs() < 0.001);

        // Corners should be exact
        let tl = bilinear_interpolate(&elevations, 2, 0.0, 0.0);
        assert!((tl - 0.0).abs() < 0.001);

        let br = bilinear_interpolate(&elevations, 2, 1.0, 1.0);
        assert!((br - 200.0).abs() < 0.001);
    }
}
