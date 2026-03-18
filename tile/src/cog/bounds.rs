//! Geographic bounds handling.

/// Geographic bounding box in WGS84 coordinates.
#[derive(Debug, Clone, Copy)]
pub struct TileBounds {
    /// Western longitude (degrees)
    pub west: f64,
    /// Southern latitude (degrees)
    pub south: f64,
    /// Eastern longitude (degrees)
    pub east: f64,
    /// Northern latitude (degrees)
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
}
