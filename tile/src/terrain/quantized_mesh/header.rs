//! Header structures for quantized-mesh format.

use super::TileBounds;
use super::coords::{
    WGS84_SEMI_MAJOR_AXIS, ecef_to_ellipsoid_scaled, geodetic_to_ecef, vec3_distance,
    vec3_magnitude, vec3_normalize,
};

/// Quantized mesh header (88 bytes).
///
/// All coordinates are in Earth-Centered Earth-Fixed (ECEF) frame.
#[derive(Debug, Clone, Copy)]
pub struct QuantizedMeshHeader {
    /// Center of the tile in ECEF coordinates (meters)
    pub center: [f64; 3],
    /// Minimum height in the tile (meters)
    pub min_height: f32,
    /// Maximum height in the tile (meters)
    pub max_height: f32,
    /// Bounding sphere center in ECEF coordinates (meters)
    pub bounding_sphere_center: [f64; 3],
    /// Bounding sphere radius (meters)
    pub bounding_sphere_radius: f64,
    /// Horizon occlusion point in ellipsoid-scaled ECEF coordinates
    pub horizon_occlusion_point: [f64; 3],
}

impl Default for QuantizedMeshHeader {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0, WGS84_SEMI_MAJOR_AXIS],
            min_height: 0.0,
            max_height: 0.0,
            bounding_sphere_center: [0.0, 0.0, WGS84_SEMI_MAJOR_AXIS],
            bounding_sphere_radius: 0.0,
            horizon_occlusion_point: [0.0, 0.0, 1.0],
        }
    }
}

impl QuantizedMeshHeader {
    /// Create a header from tile bounds and height range.
    ///
    /// Computes ECEF coordinates, bounding sphere, and horizon occlusion point.
    pub fn from_bounds(bounds: &TileBounds, min_height: f32, max_height: f32) -> Self {
        // Compute tile center in geodetic coordinates
        let center_lon = bounds.center_lon();
        let center_lat = bounds.center_lat();
        let center_height = (min_height as f64 + max_height as f64) / 2.0;

        // Convert center to ECEF
        let center = geodetic_to_ecef(center_lon, center_lat, center_height);

        // Compute bounding sphere from corner points
        let (bounding_sphere_center, bounding_sphere_radius) =
            compute_bounding_sphere(bounds, min_height as f64, max_height as f64);

        // Compute horizon occlusion point
        let horizon_occlusion_point =
            compute_horizon_occlusion_point(&bounding_sphere_center, bounding_sphere_radius);

        Self {
            center,
            min_height,
            max_height,
            bounding_sphere_center,
            bounding_sphere_radius,
            horizon_occlusion_point,
        }
    }

    /// Serialize header to bytes (88 bytes, little-endian).
    pub fn to_bytes(&self) -> [u8; 88] {
        let mut bytes = [0u8; 88];
        let mut offset = 0;

        // Center (3 x f64 = 24 bytes)
        for &v in &self.center {
            bytes[offset..offset + 8].copy_from_slice(&v.to_le_bytes());
            offset += 8;
        }

        // Min/Max height (2 x f32 = 8 bytes)
        bytes[offset..offset + 4].copy_from_slice(&self.min_height.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.max_height.to_le_bytes());
        offset += 4;

        // Bounding sphere center (3 x f64 = 24 bytes)
        for &v in &self.bounding_sphere_center {
            bytes[offset..offset + 8].copy_from_slice(&v.to_le_bytes());
            offset += 8;
        }

        // Bounding sphere radius (f64 = 8 bytes)
        bytes[offset..offset + 8].copy_from_slice(&self.bounding_sphere_radius.to_le_bytes());
        offset += 8;

        // Horizon occlusion point (3 x f64 = 24 bytes)
        for &v in &self.horizon_occlusion_point {
            bytes[offset..offset + 8].copy_from_slice(&v.to_le_bytes());
            offset += 8;
        }

        debug_assert_eq!(offset, 88);
        bytes
    }

    /// Deserialize header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 88 {
            return None;
        }

        let mut offset = 0;

        let read_f64 = |bytes: &[u8], offset: &mut usize| -> f64 {
            let v = f64::from_le_bytes(bytes[*offset..*offset + 8].try_into().unwrap());
            *offset += 8;
            v
        };

        let read_f32 = |bytes: &[u8], offset: &mut usize| -> f32 {
            let v = f32::from_le_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
            *offset += 4;
            v
        };

        let center = [
            read_f64(bytes, &mut offset),
            read_f64(bytes, &mut offset),
            read_f64(bytes, &mut offset),
        ];

        let min_height = read_f32(bytes, &mut offset);
        let max_height = read_f32(bytes, &mut offset);

        let bounding_sphere_center = [
            read_f64(bytes, &mut offset),
            read_f64(bytes, &mut offset),
            read_f64(bytes, &mut offset),
        ];

        let bounding_sphere_radius = read_f64(bytes, &mut offset);

        let horizon_occlusion_point = [
            read_f64(bytes, &mut offset),
            read_f64(bytes, &mut offset),
            read_f64(bytes, &mut offset),
        ];

        Some(Self {
            center,
            min_height,
            max_height,
            bounding_sphere_center,
            bounding_sphere_radius,
            horizon_occlusion_point,
        })
    }
}

/// Compute bounding sphere for a tile.
///
/// Returns (center, radius) in ECEF coordinates.
///
/// The center is the geographic center of the tile at the average height,
/// and the radius is the maximum distance from this center to any corner.
fn compute_bounding_sphere(
    bounds: &TileBounds,
    min_height: f64,
    max_height: f64,
) -> ([f64; 3], f64) {
    // Use the geographic center of the tile at average height as the bounding sphere center
    // This gives a tighter bounding sphere than using the centroid of corner points,
    // especially for large tiles like level 0 which span half the globe.
    let avg_height = (min_height + max_height) / 2.0;
    let center = geodetic_to_ecef(bounds.center_lon(), bounds.center_lat(), avg_height);

    // Sample corner and edge points at both height extremes
    let points = [
        // Corners at min height
        geodetic_to_ecef(bounds.west, bounds.south, min_height),
        geodetic_to_ecef(bounds.east, bounds.south, min_height),
        geodetic_to_ecef(bounds.west, bounds.north, min_height),
        geodetic_to_ecef(bounds.east, bounds.north, min_height),
        // Corners at max height
        geodetic_to_ecef(bounds.west, bounds.south, max_height),
        geodetic_to_ecef(bounds.east, bounds.south, max_height),
        geodetic_to_ecef(bounds.west, bounds.north, max_height),
        geodetic_to_ecef(bounds.east, bounds.north, max_height),
        // Edge midpoints at max height (important for large tiles)
        geodetic_to_ecef(bounds.west, bounds.center_lat(), max_height),
        geodetic_to_ecef(bounds.east, bounds.center_lat(), max_height),
        geodetic_to_ecef(bounds.center_lon(), bounds.south, max_height),
        geodetic_to_ecef(bounds.center_lon(), bounds.north, max_height),
    ];

    // Compute radius as max distance from center to any sampled point
    let mut radius = 0.0f64;
    for p in &points {
        let dist = vec3_distance(&center, p);
        radius = radius.max(dist);
    }

    (center, radius)
}

/// Compute horizon occlusion point in ellipsoid-scaled coordinates.
///
/// This point is used for efficient horizon culling. It's computed by
/// scaling the bounding sphere center to unit ellipsoid and pushing
/// it outward along the surface normal.
fn compute_horizon_occlusion_point(
    bounding_sphere_center: &[f64; 3],
    bounding_sphere_radius: f64,
) -> [f64; 3] {
    // Scale to unit ellipsoid
    let scaled = ecef_to_ellipsoid_scaled(bounding_sphere_center);

    // Magnitude in scaled space
    let mag = vec3_magnitude(&scaled);

    if mag < 1e-10 {
        return [0.0, 0.0, 1.0];
    }

    // Direction (normalized)
    let dir = vec3_normalize(&scaled);

    // Push outward by scaled radius
    let scaled_radius = bounding_sphere_radius / WGS84_SEMI_MAJOR_AXIS;
    let occlusion_scale = mag + scaled_radius;

    [
        dir[0] * occlusion_scale,
        dir[1] * occlusion_scale,
        dir[2] * occlusion_scale,
    ]
}

#[cfg(test)]
mod tests {
    use super::super::coords::WGS84_SEMI_MINOR_AXIS;
    use super::*;

    #[test]
    fn test_header_serialization_roundtrip() {
        let header = QuantizedMeshHeader {
            center: [1.0, 2.0, 3.0],
            min_height: 100.0,
            max_height: 200.0,
            bounding_sphere_center: [4.0, 5.0, 6.0],
            bounding_sphere_radius: 1000.0,
            horizon_occlusion_point: [0.1, 0.2, 0.3],
        };

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 88);

        let parsed = QuantizedMeshHeader::from_bytes(&bytes).unwrap();

        assert_eq!(header.center, parsed.center);
        assert_eq!(header.min_height, parsed.min_height);
        assert_eq!(header.max_height, parsed.max_height);
        assert_eq!(header.bounding_sphere_center, parsed.bounding_sphere_center);
        assert_eq!(header.bounding_sphere_radius, parsed.bounding_sphere_radius);
        assert_eq!(
            header.horizon_occlusion_point,
            parsed.horizon_occlusion_point
        );
    }

    #[test]
    fn test_header_from_bounds() {
        let bounds = TileBounds::new(-1.0, -1.0, 1.0, 1.0);
        let header = QuantizedMeshHeader::from_bounds(&bounds, 0.0, 100.0);

        // Center should be near equator/prime meridian
        assert!(header.center[0] > 0.0); // X should be positive (facing prime meridian)
        assert!(header.center[1].abs() < 1000.0); // Y should be near zero
        assert!(header.center[2].abs() < 1000.0); // Z should be near zero

        assert_eq!(header.min_height, 0.0);
        assert_eq!(header.max_height, 100.0);
        assert!(header.bounding_sphere_radius > 0.0);
    }

    #[test]
    fn test_header_default() {
        let header = QuantizedMeshHeader::default();

        assert_eq!(header.center[0], 0.0);
        assert_eq!(header.center[1], 0.0);
        assert!((header.center[2] - WGS84_SEMI_MAJOR_AXIS).abs() < 1.0);
    }

    #[test]
    fn test_bounding_sphere_at_pole() {
        let bounds = TileBounds::new(-10.0, 80.0, 10.0, 90.0);
        let header = QuantizedMeshHeader::from_bounds(&bounds, 0.0, 1000.0);

        // Near north pole, Z should be close to semi-minor axis
        assert!(header.center[2] > WGS84_SEMI_MINOR_AXIS * 0.9);
        assert!(header.bounding_sphere_radius > 0.0);
    }
}
