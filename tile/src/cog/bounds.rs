//! Geographic bounds handling.

use std::f64::consts::PI;

/// Semi-major axis of the WGS84 / Web Mercator sphere (meters).
const EARTH_RADIUS: f64 = 6_378_137.0;
/// Half the Web Mercator world extent (meters) — `PI * EARTH_RADIUS`.
pub const MERCATOR_HALF_WORLD: f64 = PI * EARTH_RADIUS;

/// Coordinate reference system of a COG, as far as this server cares about it.
///
/// The rendering pipeline samples linearly between the requested tile bounds and
/// the COG bounds, so all it needs to know is which *space* those numbers live
/// in. Two cases cover every COG we accept:
///
/// - [`CogCrs::Geographic`]: lon/lat in degrees. WGS84 (EPSG:4326) and JGD2011
///   geographic (EPSG:6668) are treated identically — the datum shift in Japan
///   is sub-meter, far below a raster pixel, so the error is negligible.
/// - [`CogCrs::WebMercator`]: easting/northing in meters (EPSG:3857). XYZ tiles
///   *are* Web Mercator, so a requested tile maps to a perfect square in this
///   space and the linear sampling becomes exact (no latitude distortion).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CogCrs {
    /// Geographic lon/lat degrees (EPSG:4326 WGS84, EPSG:6668 JGD2011).
    Geographic,
    /// Web Mercator meters (EPSG:3857 and legacy aliases).
    WebMercator,
}

impl CogCrs {
    /// Classify an EPSG code into a supported [`CogCrs`], or `None` if we can't
    /// render it (e.g. a JGD2011 *plane rectangular* CRS in meters, which is a
    /// completely different projection from the negligible-error geographic one).
    pub fn from_epsg(epsg: u16) -> Option<Self> {
        match epsg {
            // WGS84 geographic, JGD2011 geographic, JGD2000 geographic.
            4326 | 6668 | 4612 => Some(Self::Geographic),
            // Web Mercator and its legacy alias (the ESRI 90091x/10210x aliases
            // exceed u16 and can't appear in a GeoTIFF EPSG key, so they're moot).
            3857 | 3785 => Some(Self::WebMercator),
            _ => None,
        }
    }
}

/// Convert a Web Mercator easting (meters) to WGS84 longitude (degrees).
pub fn mercator_x_to_lon(x: f64) -> f64 {
    (x / EARTH_RADIUS).to_degrees()
}

/// Convert a Web Mercator northing (meters) to WGS84 latitude (degrees).
pub fn mercator_y_to_lat(y: f64) -> f64 {
    (2.0 * (y / EARTH_RADIUS).exp().atan() - PI / 2.0).to_degrees()
}

/// Geographic bounding box. Coordinates are degrees for [`CogCrs::Geographic`]
/// and meters for [`CogCrs::WebMercator`]; the field names still denote the
/// min/max edges along each axis.
#[derive(Debug, Clone, Copy)]
pub struct TileBounds {
    /// Western edge (longitude degrees, or Web Mercator easting meters)
    pub west: f64,
    /// Southern edge (latitude degrees, or Web Mercator northing meters)
    pub south: f64,
    /// Eastern edge (longitude degrees, or Web Mercator easting meters)
    pub east: f64,
    /// Northern edge (latitude degrees, or Web Mercator northing meters)
    pub north: f64,
}

impl TileBounds {
    /// Create a new TileBounds
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// Get the center point of the bounds
    pub fn center(&self) -> (f64, f64) {
        (
            (self.west + self.east) / 2.0,
            (self.south + self.north) / 2.0,
        )
    }

    /// Get the size of the bounds (width, height) in degrees
    pub fn size(&self) -> (f64, f64) {
        (self.east - self.west, self.north - self.south)
    }

    /// Check if this bounds intersects with another
    pub fn intersects(&self, other: &TileBounds) -> bool {
        self.west < other.east
            && self.east > other.west
            && self.south < other.north
            && self.north > other.south
    }

    /// Check if this bounds contains a point
    pub fn contains_point(&self, lon: f64, lat: f64) -> bool {
        lon >= self.west && lon <= self.east && lat >= self.south && lat <= self.north
    }

    /// Reproject Web Mercator meter bounds into WGS84 degrees.
    ///
    /// Web Mercator's axes are monotonic with lon/lat, so reprojecting the
    /// corners yields a correct degree bounding box. Used to report a COG's
    /// extent and to intersection-test against WGS84 XYZ tiles, while sampling
    /// stays in the COG's native meter space.
    pub fn mercator_to_wgs84(&self) -> TileBounds {
        TileBounds {
            west: mercator_x_to_lon(self.west),
            east: mercator_x_to_lon(self.east),
            north: mercator_y_to_lat(self.north),
            south: mercator_y_to_lat(self.south),
        }
    }
}

/// Web Mercator XYZ tile → bounding box in **meters** (EPSG:3857).
///
/// Unlike the WGS84 [`crate::tile::xyz_to_bounds`], the result is a perfect
/// square in projected space, so sampling a Web Mercator COG with these bounds
/// is exact rather than an approximation.
pub fn mercator_tile_bounds(z: u32, x: u32, y: u32) -> TileBounds {
    let n = 2.0_f64.powi(z as i32);
    let tile_size = (2.0 * MERCATOR_HALF_WORLD) / n;

    let west = -MERCATOR_HALF_WORLD + x as f64 * tile_size;
    let east = west + tile_size;
    let north = MERCATOR_HALF_WORLD - y as f64 * tile_size;
    let south = north - tile_size;

    TileBounds {
        west,
        south,
        east,
        north,
    }
}

/// Convert geographic longitude to pixel X coordinate
pub fn geo_to_pixel_x(lon: f64, bounds: &TileBounds, width: u32) -> f64 {
    let ratio = (lon - bounds.west) / (bounds.east - bounds.west);
    ratio * width as f64
}

/// Convert geographic latitude to pixel Y coordinate
pub fn geo_to_pixel_y(lat: f64, bounds: &TileBounds, height: u32) -> f64 {
    let ratio = (bounds.north - lat) / (bounds.north - bounds.south);
    ratio * height as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersects() {
        let a = TileBounds::new(0.0, 0.0, 10.0, 10.0);
        let b = TileBounds::new(5.0, 5.0, 15.0, 15.0);
        let c = TileBounds::new(20.0, 20.0, 30.0, 30.0);

        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_geo_to_pixel() {
        let bounds = TileBounds::new(0.0, 0.0, 10.0, 10.0);

        assert!((geo_to_pixel_x(0.0, &bounds, 100) - 0.0).abs() < 1e-6);
        assert!((geo_to_pixel_x(5.0, &bounds, 100) - 50.0).abs() < 1e-6);
        assert!((geo_to_pixel_x(10.0, &bounds, 100) - 100.0).abs() < 1e-6);

        assert!((geo_to_pixel_y(10.0, &bounds, 100) - 0.0).abs() < 1e-6);
        assert!((geo_to_pixel_y(5.0, &bounds, 100) - 50.0).abs() < 1e-6);
        assert!((geo_to_pixel_y(0.0, &bounds, 100) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_crs_from_epsg() {
        assert_eq!(CogCrs::from_epsg(4326), Some(CogCrs::Geographic));
        assert_eq!(CogCrs::from_epsg(6668), Some(CogCrs::Geographic)); // JGD2011 geographic
        assert_eq!(CogCrs::from_epsg(3857), Some(CogCrs::WebMercator));
        assert_eq!(CogCrs::from_epsg(3785), Some(CogCrs::WebMercator)); // legacy mercator alias
        assert_eq!(CogCrs::from_epsg(6677), None); // JGD2011 plane rectangular (meters) - unsupported
        assert_eq!(CogCrs::from_epsg(32654), None); // UTM zone 54N - unsupported
    }

    #[test]
    fn test_mercator_tile_bounds_z0() {
        let b = mercator_tile_bounds(0, 0, 0);
        assert!((b.west + MERCATOR_HALF_WORLD).abs() < 1e-6);
        assert!((b.east - MERCATOR_HALF_WORLD).abs() < 1e-6);
        assert!((b.north - MERCATOR_HALF_WORLD).abs() < 1e-6);
        assert!((b.south + MERCATOR_HALF_WORLD).abs() < 1e-6);
    }

    #[test]
    fn test_mercator_roundtrip_to_wgs84() {
        // The whole-world mercator tile maps to the full ±180° / ±85.0511°.
        let wgs = mercator_tile_bounds(0, 0, 0).mercator_to_wgs84();
        assert!((wgs.west + 180.0).abs() < 1e-6);
        assert!((wgs.east - 180.0).abs() < 1e-6);
        assert!((wgs.north - 85.051_128_78).abs() < 1e-4);
        assert!((wgs.south + 85.051_128_78).abs() < 1e-4);
    }

    #[test]
    fn test_mercator_to_wgs84_tokyo() {
        // Tokyo ~ (139.6917, 35.6895) round-trips through mercator meters.
        let x = 139.6917_f64.to_radians() * EARTH_RADIUS;
        let y = (PI / 4.0 + 35.6895_f64.to_radians() / 2.0).tan().ln() * EARTH_RADIUS;
        assert!((mercator_x_to_lon(x) - 139.6917).abs() < 1e-6);
        assert!((mercator_y_to_lat(y) - 35.6895).abs() < 1e-6);
    }
}
