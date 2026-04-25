//! Web Mercator XYZ tile geometry helpers.
//!
//! Used by the `/mapbox/{z}/{x}/{y}` and `/terrarium/{z}/{x}/{y}` raster
//! endpoints, which serve Mapbox Terrain-RGB and Mapzen Terrarium tiles in
//! standard Web Mercator XYZ. (The `/terrain/{z}/{x}/{y}.terrain`
//! quantized-mesh endpoint uses TMS Geodetic instead — see `geodetic.rs`.)
//!
//! Math ported from
//! <https://github.com/eukarya-inc/stralift> (`crates/stralift/src/tile.rs`).

use std::f64::consts::PI;

use super::geodetic::GeodeticBounds;

/// Geographic bounds (lat/lon, EPSG:4326) of a Web Mercator XYZ tile.
pub fn xyz_tile_bounds(z: u8, x: u32, y: u32) -> GeodeticBounds {
    let n = (1u64 << z) as f64;

    let west = (x as f64 / n) * 360.0 - 180.0;
    let east = ((x + 1) as f64 / n) * 360.0 - 180.0;
    // XYZ Y grows southward, so y=0 is the north edge.
    let north = tile_y_to_lat(y as f64, n);
    let south = tile_y_to_lat((y + 1) as f64, n);

    GeodeticBounds {
        west,
        south,
        east,
        north,
    }
}

/// XYZ tile-Y (in Mercator pixel space) to latitude in degrees.
fn tile_y_to_lat(y: f64, n: f64) -> f64 {
    let lat_rad = (PI * (1.0 - 2.0 * y / n)).sinh().atan();
    lat_rad.to_degrees()
}

/// Pixel-center longitude for column `px` of a `tile_size`-wide XYZ tile.
pub fn xyz_pixel_lon(z: u8, x: u32, tile_size: u32, px: u32) -> f64 {
    let n = (1u64 << z) as f64;
    let tx = x as f64 + (px as f64 + 0.5) / tile_size as f64;
    (tx / n) * 360.0 - 180.0
}

/// Pixel-center latitude for row `py` of a `tile_size`-tall XYZ tile.
pub fn xyz_pixel_lat(z: u8, y: u32, tile_size: u32, py: u32) -> f64 {
    let n = (1u64 << z) as f64;
    let ty = y as f64 + (py as f64 + 0.5) / tile_size as f64;
    tile_y_to_lat(ty, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z0_covers_world() {
        let b = xyz_tile_bounds(0, 0, 0);
        assert!((b.west - (-180.0)).abs() < 1e-10);
        assert!((b.east - 180.0).abs() < 1e-10);
        assert!((b.north - 85.0511).abs() < 0.001);
        assert!((b.south - (-85.0511)).abs() < 0.001);
    }

    #[test]
    fn z3_x7_y3_covers_japan() {
        // The bug-report tile: z=3, x=7, y=3 is northern hemisphere east-Asia
        // in Web Mercator XYZ — it overlaps the GSIGEO2011 coverage box.
        // z=3 → 8×8 grid; x=7 covers lon 135°–180°, y=3 covers ≈0°–40°N.
        let b = xyz_tile_bounds(3, 7, 3);
        assert!(b.west <= 135.0 && b.east >= 154.0);
        assert!(b.south < 30.0 && b.north > 35.0);
    }

    #[test]
    fn tokyo_z10() {
        // Tokyo ≈ (139.7E, 35.7N) → z=10, x=909, y=403.
        let b = xyz_tile_bounds(10, 909, 403);
        assert!(b.west < 139.7 && b.east > 139.7);
        assert!(b.south < 35.7 && b.north > 35.7);
    }

    #[test]
    fn pixel_centers_round_trip() {
        let z = 5;
        let x = 28;
        let y = 12;
        let size = 256;
        let b = xyz_tile_bounds(z, x, y);
        // Top-left pixel center should sit inside the tile near its NW corner.
        let lon0 = xyz_pixel_lon(z, x, size, 0);
        let lat0 = xyz_pixel_lat(z, y, size, 0);
        assert!(lon0 > b.west && lon0 < b.east);
        assert!(lat0 < b.north && lat0 > b.south);
        // Bottom-right pixel center near SE corner.
        let lon1 = xyz_pixel_lon(z, x, size, size - 1);
        let lat1 = xyz_pixel_lat(z, y, size, size - 1);
        assert!(lon1 > lon0 && lon1 < b.east);
        assert!(lat1 < lat0 && lat1 > b.south);
    }
}
