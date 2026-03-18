//! XYZ tile coordinate utilities.

use std::f64::consts::PI;

use crate::cog::TileBounds;

/// XYZ tile coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

impl TileCoord {
    pub fn new(z: u32, x: u32, y: u32) -> Self {
        Self { z, x, y }
    }
}

/// Convert XYZ tile coordinates to WGS84 bounding box.
///
/// Uses Web Mercator projection formula to convert tile coordinates to geographic bounds.
pub fn xyz_to_bounds(z: u32, x: u32, y: u32) -> TileBounds {
    let n = 2.0_f64.powi(z as i32);

    let west = x as f64 / n * 360.0 - 180.0;
    let east = (x + 1) as f64 / n * 360.0 - 180.0;

    let north_rad = (PI * (1.0 - 2.0 * y as f64 / n)).sinh().atan();
    let south_rad = (PI * (1.0 - 2.0 * (y + 1) as f64 / n)).sinh().atan();

    let north = north_rad.to_degrees();
    let south = south_rad.to_degrees();

    TileBounds {
        west,
        south,
        east,
        north,
    }
}

/// Convert WGS84 coordinates to XYZ tile coordinate at a specific zoom level.
#[allow(dead_code)]
pub fn latlon_to_tile(lat: f64, lon: f64, z: u32) -> TileCoord {
    let n = 2.0_f64.powi(z as i32);

    let x = ((lon + 180.0) / 360.0 * n).floor() as u32;

    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n).floor() as u32;

    TileCoord {
        z,
        x: x.min((n as u32).saturating_sub(1)),
        y: y.min((n as u32).saturating_sub(1)),
    }
}

/// Get all tiles that intersect with a bounding box at a specific zoom level.
#[allow(dead_code)]
pub fn bounds_to_tiles(bounds: &TileBounds, z: u32) -> Vec<TileCoord> {
    let min_tile = latlon_to_tile(bounds.north, bounds.west, z);
    let max_tile = latlon_to_tile(bounds.south, bounds.east, z);

    let mut tiles = Vec::new();
    for y in min_tile.y..=max_tile.y {
        for x in min_tile.x..=max_tile.x {
            tiles.push(TileCoord::new(z, x, y));
        }
    }
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xyz_to_bounds_z0() {
        let bounds = xyz_to_bounds(0, 0, 0);
        assert!((bounds.west - (-180.0)).abs() < 1e-6);
        assert!((bounds.east - 180.0).abs() < 1e-6);
        assert!(bounds.north > 80.0);
        assert!(bounds.south < -80.0);
    }

    #[test]
    fn test_xyz_to_bounds_z1() {
        // Top-left tile at z=1
        let bounds = xyz_to_bounds(1, 0, 0);
        assert!((bounds.west - (-180.0)).abs() < 1e-6);
        assert!((bounds.east - 0.0).abs() < 1e-6);
        assert!(bounds.north > 80.0);
        assert!((bounds.south - 0.0).abs() < 1.0);
    }

    #[test]
    fn test_latlon_to_tile() {
        // Tokyo area at z=10
        let tile = latlon_to_tile(35.6762, 139.6503, 10);
        assert_eq!(tile.z, 10);
        // Should be around x=909, y=403
        assert!(tile.x > 900 && tile.x < 920);
        assert!(tile.y > 395 && tile.y < 415);
    }

    #[test]
    fn test_roundtrip() {
        // Convert tile to bounds and back
        let original = TileCoord::new(10, 909, 403);
        let bounds = xyz_to_bounds(original.z, original.x, original.y);
        let center_lat = (bounds.north + bounds.south) / 2.0;
        let center_lon = (bounds.west + bounds.east) / 2.0;
        let recovered = latlon_to_tile(center_lat, center_lon, original.z);

        assert_eq!(original, recovered);
    }
}
