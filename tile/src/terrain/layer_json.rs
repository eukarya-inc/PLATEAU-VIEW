//! Cesium layer.json generation
//!
//! Generates layer.json metadata files for Cesium terrain tilesets.

use serde::{Deserialize, Serialize};

/// Configuration for layer.json generation
#[derive(Debug, Clone)]
pub struct LayerJsonConfig {
    /// Tile URL template (e.g., "{z}/{x}/{y}.terrain")
    pub tiles_template: String,
    /// Version string for cache busting
    pub version: String,
    /// Attribution text
    pub attribution: Option<String>,
    /// Available tiles by zoom level
    pub available: Vec<Vec<TileAvailability>>,
    /// Minimum zoom level
    pub min_zoom: Option<u8>,
    /// Maximum zoom level
    pub max_zoom: Option<u8>,
    /// Tiling scheme ("tms" or "xyz")
    pub scheme: String,
    /// Geographic bounds [west, south, east, north]
    pub bounds: Option<[f64; 4]>,
    /// Enabled extensions (e.g., "octvertexnormals", "watermask", "metadata")
    pub extensions: Vec<String>,
    /// Terrain format ("heightmap-1.0" or "quantized-mesh-1.0")
    pub format: String,
    /// Metadata availability level (enables dynamic tile availability discovery)
    /// When set, tiles include metadata extension with child tile availability
    pub metadata_availability: Option<u8>,
}

impl Default for LayerJsonConfig {
    fn default() -> Self {
        Self {
            tiles_template: "{z}/{x}/{y}.terrain".to_string(),
            version: "1.0.0".to_string(),
            attribution: None,
            available: Vec::new(),
            min_zoom: None,
            max_zoom: None,
            scheme: "tms".to_string(),
            bounds: None,
            extensions: Vec::new(),
            format: "heightmap-1.0".to_string(),
            metadata_availability: None,
        }
    }
}

/// Defines a range of available tiles at a zoom level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileAvailability {
    /// Starting X coordinate (inclusive)
    pub start_x: u32,
    /// Starting Y coordinate (inclusive)
    pub start_y: u32,
    /// Ending X coordinate (inclusive)
    pub end_x: u32,
    /// Ending Y coordinate (inclusive)
    pub end_y: u32,
}

impl TileAvailability {
    /// Create a new tile availability range
    pub fn new(start_x: u32, start_y: u32, end_x: u32, end_y: u32) -> Self {
        Self {
            start_x,
            start_y,
            end_x,
            end_y,
        }
    }

    /// Create availability for the entire zoom level in global-geodetic TMS scheme
    ///
    /// Global-geodetic: 2^(z+1) tiles in X, 2^z tiles in Y
    pub fn full_level_geodetic(zoom: u8) -> Self {
        let max_x = (1u32 << (zoom + 1)) - 1; // 2^(z+1) - 1
        let max_y = (1u32 << zoom) - 1; // 2^z - 1
        Self {
            start_x: 0,
            start_y: 0,
            end_x: max_x,
            end_y: max_y,
        }
    }

    /// Create availability for the entire zoom level in TMS scheme (global-geodetic)
    #[deprecated(note = "Use full_level_geodetic instead")]
    pub fn full_level(zoom: u8) -> Self {
        Self::full_level_geodetic(zoom)
    }

    /// Create availability for the entire zoom level in XYZ scheme (Web Mercator)
    pub fn full_level_xyz(zoom: u8) -> Self {
        let max_tile = (1u32 << zoom) - 1;
        Self {
            start_x: 0,
            start_y: 0,
            end_x: max_tile,
            end_y: max_tile,
        }
    }

    /// Create availability from geographic bounds (TMS scheme)
    ///
    /// # Arguments
    /// * `zoom` - Zoom level
    /// * `west` - Western longitude in degrees
    /// * `south` - Southern latitude in degrees
    /// * `east` - Eastern longitude in degrees
    /// * `north` - Northern latitude in degrees
    pub fn from_bounds(zoom: u8, west: f64, south: f64, east: f64, north: f64) -> Self {
        // TMS Y coordinate increases from south to north
        let (start_x, _) = lonlat_to_tms_tile(west, south, zoom);
        let (end_x, _) = lonlat_to_tms_tile(east, north, zoom);
        let (_, start_y) = lonlat_to_tms_tile(west, south, zoom);
        let (_, end_y) = lonlat_to_tms_tile(east, north, zoom);

        Self {
            start_x,
            start_y,
            end_x,
            end_y,
        }
    }

    /// Create availability from geographic bounds (XYZ scheme - Web Mercator)
    ///
    /// # Arguments
    /// * `zoom` - Zoom level
    /// * `west` - Western longitude in degrees
    /// * `south` - Southern latitude in degrees
    /// * `east` - Eastern longitude in degrees
    /// * `north` - Northern latitude in degrees
    pub fn from_bounds_xyz(zoom: u8, west: f64, south: f64, east: f64, north: f64) -> Self {
        // XYZ Y coordinate increases from north to south
        let (start_x, end_y) = lonlat_to_xyz_tile(west, south, zoom);
        let (end_x, start_y) = lonlat_to_xyz_tile(east, north, zoom);

        Self {
            start_x,
            start_y,
            end_x,
            end_y,
        }
    }

    /// Create availability from geographic bounds (TMS scheme - Y flipped from XYZ)
    ///
    /// TMS is the same as XYZ but Y is flipped (Y=0 is south instead of north)
    ///
    /// # Arguments
    /// * `zoom` - Zoom level
    /// * `west` - Western longitude in degrees
    /// * `south` - Southern latitude in degrees
    /// * `east` - Eastern longitude in degrees
    /// * `north` - Northern latitude in degrees
    pub fn from_bounds_tms(zoom: u8, west: f64, south: f64, east: f64, north: f64) -> Self {
        // First get XYZ coordinates
        let (start_x, xyz_end_y) = lonlat_to_xyz_tile(west, south, zoom);
        let (end_x, xyz_start_y) = lonlat_to_xyz_tile(east, north, zoom);

        // Flip Y for TMS (tms_y = max_y - xyz_y)
        let max_y = (1u32 << zoom) - 1;
        let tms_start_y = max_y.saturating_sub(xyz_end_y);
        let tms_end_y = max_y.saturating_sub(xyz_start_y);

        Self {
            start_x,
            start_y: tms_start_y,
            end_x,
            end_y: tms_end_y,
        }
    }

    /// Create availability from geographic bounds (global-geodetic TMS scheme)
    ///
    /// Global-geodetic TMS has 2^(z+1) tiles in X and 2^z tiles in Y.
    /// Y=0 is at the south pole.
    ///
    /// # Arguments
    /// * `zoom` - Zoom level
    /// * `west` - Western longitude in degrees
    /// * `south` - Southern latitude in degrees
    /// * `east` - Eastern longitude in degrees
    /// * `north` - Northern latitude in degrees
    pub fn from_bounds_geodetic(zoom: u8, west: f64, south: f64, east: f64, north: f64) -> Self {
        let (start_x, start_y) = lonlat_to_geodetic_tms_tile(west, south, zoom);
        let (end_x, end_y) = lonlat_to_geodetic_tms_tile(east, north, zoom);

        Self {
            start_x,
            start_y,
            end_x,
            end_y,
        }
    }
}

/// Convert longitude/latitude to global-geodetic TMS tile coordinates
///
/// Global-geodetic TMS uses equirectangular projection where:
/// - X ranges from 0 (west) to 2^(z+1) - 1 (east)
/// - Y ranges from 0 (south) to 2^z - 1 (north)
fn lonlat_to_geodetic_tms_tile(lon: f64, lat: f64, zoom: u8) -> (u32, u32) {
    // Clamp to valid range
    let lon_clamped = lon.clamp(-180.0, 180.0);
    let lat_clamped = lat.clamp(-90.0, 90.0);

    // Calculate tile coordinates
    let n_tiles_x = 1u32 << (zoom + 1); // 2^(z+1) tiles in X
    let n_tiles_y = 1u32 << zoom; // 2^z tiles in Y

    // Normalize to 0-1 range
    let lon_normalized = (lon_clamped + 180.0) / 360.0;
    let lat_normalized = (lat_clamped + 90.0) / 180.0;

    let x = ((lon_normalized * n_tiles_x as f64).floor() as u32).min(n_tiles_x - 1);
    let y = ((lat_normalized * n_tiles_y as f64).floor() as u32).min(n_tiles_y - 1);

    (x, y)
}

/// Convert longitude/latitude to TMS tile coordinates
///
/// TMS uses a global-geodetic profile where:
/// - X ranges from 0 (west) to 2^(z+1) - 1 (east)
/// - Y ranges from 0 (south) to 2^z - 1 (north)
fn lonlat_to_tms_tile(lon: f64, lat: f64, zoom: u8) -> (u32, u32) {
    // Normalize longitude to 0-360 range
    let lon_normalized = (lon + 180.0) / 360.0;

    // Normalize latitude to 0-1 range (simple equirectangular)
    let lat_normalized = (lat + 90.0) / 180.0;

    // Calculate tile coordinates
    let n_tiles_x = 1u32 << (zoom + 1); // 2^(z+1) tiles in X
    let n_tiles_y = 1u32 << zoom; // 2^z tiles in Y

    let x = ((lon_normalized * n_tiles_x as f64).floor() as u32).min(n_tiles_x - 1);
    let y = ((lat_normalized * n_tiles_y as f64).floor() as u32).min(n_tiles_y - 1);

    (x, y)
}

/// Maximum latitude for Web Mercator projection (degrees)
///
/// Derived from: (2 * atan(e^π) - π/2) * 180/π ≈ 85.051129°
/// This is where the Mercator projection reaches y = ±π
const WEB_MERCATOR_MAX_LAT: f64 = 85.05112877980659;

/// Convert longitude/latitude to XYZ tile coordinates (Web Mercator)
///
/// XYZ uses Web Mercator projection where:
/// - X ranges from 0 (west) to 2^z - 1 (east)
/// - Y ranges from 0 (north) to 2^z - 1 (south)
fn lonlat_to_xyz_tile(lon: f64, lat: f64, zoom: u8) -> (u32, u32) {
    let n = 1u32 << zoom;

    // Clamp longitude to valid range
    let lon_clamped = lon.clamp(-180.0, 180.0);
    let x = (((lon_clamped + 180.0) / 360.0) * n as f64).floor() as u32;
    let x = x.min(n - 1);

    // Clamp latitude to Web Mercator valid range (avoids infinity at poles)
    let lat_clamped = lat.clamp(-WEB_MERCATOR_MAX_LAT, WEB_MERCATOR_MAX_LAT);
    let lat_rad = lat_clamped.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n as f64).floor() as u32;
    let y = y.min(n - 1);

    (x, y)
}

/// The layer.json structure for Cesium terrain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerJson {
    /// TileJSON version
    pub tilejson: String,
    /// Terrain format identifier
    pub format: String,
    /// Data version
    pub version: String,
    /// Tiling scheme ("tms" for heightmap-1.0)
    pub scheme: String,
    /// Tile URL templates
    pub tiles: Vec<String>,
    /// Available tiles by zoom level
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub available: Vec<Vec<TileAvailability>>,
    /// Attribution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    /// Minimum zoom level
    #[serde(skip_serializing_if = "Option::is_none", rename = "minzoom")]
    pub min_zoom: Option<u8>,
    /// Maximum zoom level
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxzoom")]
    pub max_zoom: Option<u8>,
    /// Geographic bounds [west, south, east, north]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[f64; 4]>,
    /// Enabled extensions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
    /// Metadata availability level (enables dynamic tile availability discovery)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_availability: Option<u8>,
}

/// Generate a layer.json structure
pub fn generate_layer_json(config: &LayerJsonConfig) -> LayerJson {
    LayerJson {
        tilejson: "2.1.0".to_string(),
        format: config.format.clone(),
        version: config.version.clone(),
        scheme: config.scheme.clone(),
        tiles: vec![config.tiles_template.clone()],
        available: config.available.clone(),
        attribution: config.attribution.clone(),
        min_zoom: config.min_zoom,
        max_zoom: config.max_zoom,
        bounds: config.bounds,
        extensions: config.extensions.clone(),
        metadata_availability: config.metadata_availability,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lonlat_to_tms_tile() {
        // Center of the world at zoom 0
        let (x, y) = lonlat_to_tms_tile(0.0, 0.0, 0);
        assert_eq!(x, 1); // East of center
        assert_eq!(y, 0); // South half at zoom 0

        // Northwest corner at zoom 1
        let (x, y) = lonlat_to_tms_tile(-180.0, 90.0, 1);
        assert_eq!(x, 0);
        assert_eq!(y, 1); // TMS Y increases northward
    }

    #[test]
    fn test_tile_availability_from_bounds() {
        // Japan roughly
        let avail = TileAvailability::from_bounds(4, 122.0, 20.0, 154.0, 46.0);

        assert!(avail.start_x <= avail.end_x);
        assert!(avail.start_y <= avail.end_y);
    }

    #[test]
    fn test_generate_layer_json() {
        let config = LayerJsonConfig {
            tiles_template: "{z}/{x}/{y}.terrain".to_string(),
            version: "1.0.0".to_string(),
            attribution: Some("Test".to_string()),
            available: vec![vec![TileAvailability::full_level_geodetic(0)]],
            min_zoom: Some(0),
            max_zoom: Some(10),
            scheme: "tms".to_string(),
            bounds: None,
            extensions: vec!["watermask".to_string()],
            format: "heightmap-1.0".to_string(),
            metadata_availability: None,
        };

        let json = generate_layer_json(&config);

        assert_eq!(json.tilejson, "2.1.0");
        assert_eq!(json.format, "heightmap-1.0");
        assert_eq!(json.scheme, "tms");
        assert!(json.extensions.contains(&"watermask".to_string()));
    }

    #[test]
    fn test_layer_json_serialization() {
        let config = LayerJsonConfig::default();
        let json = generate_layer_json(&config);

        let serialized = serde_json::to_string_pretty(&json).unwrap();
        assert!(serialized.contains("heightmap-1.0"));
        assert!(serialized.contains("tms"));
    }

    #[test]
    fn test_from_bounds_tms_full_world() {
        // Full world bounds
        let avail = TileAvailability::from_bounds_tms(1, -180.0, -90.0, 180.0, 90.0);
        println!(
            "TMS z1 full world: startX={}, startY={}, endX={}, endY={}",
            avail.start_x, avail.start_y, avail.end_x, avail.end_y
        );
        // In TMS, Y=0 is south, so full world should be startY=0, endY=1
        assert_eq!(avail.start_y, 0, "startY should be 0 (south)");
        assert_eq!(avail.end_y, 1, "endY should be 1 (north)");
    }

    #[test]
    fn test_lonlat_to_xyz_tile_bounds() {
        // South pole area
        let (x_sw, y_sw) = lonlat_to_xyz_tile(-180.0, -85.0, 1);
        println!("XYZ z1 SW corner (-180, -85): x={x_sw}, y={y_sw}");

        // North pole area
        let (x_ne, y_ne) = lonlat_to_xyz_tile(180.0, 85.0, 1);
        println!("XYZ z1 NE corner (180, 85): x={x_ne}, y={y_ne}");

        // In XYZ, Y=0 is north, Y=1 is south at zoom 1
        assert_eq!(y_sw, 1, "South should have Y=1 in XYZ");
        assert_eq!(y_ne, 0, "North should have Y=0 in XYZ");
    }

    #[test]
    fn test_from_bounds_tms_with_overflow() {
        // Actual bounds from geoid provider (slightly beyond poles)
        let avail = TileAvailability::from_bounds_tms(
            1,
            -180.02083333333334,
            -90.02083333333334,
            179.97916666666666,
            90.02083333333331,
        );
        println!(
            "TMS z1 with overflow: startX={}, startY={}, endX={}, endY={}",
            avail.start_x, avail.start_y, avail.end_x, avail.end_y
        );
        // Should still be valid
        assert!(
            avail.start_y <= avail.end_y,
            "startY ({}) should be <= endY ({})",
            avail.start_y,
            avail.end_y
        );
    }
}
