use image::{ImageBuffer, Rgb, RgbImage};

/// Encode elevation data to Mapbox Terrain-RGB v1 format.
///
/// Mapbox encoding formula:
/// - elevation = -10000 + ((R * 256 * 256 + G * 256 + B) * 0.1)
///
/// Inverse (encoding):
/// - value = (elevation + 10000) * 10
/// - R = floor(value / 65536)
/// - G = floor((value % 65536) / 256)
/// - B = floor(value % 256)
///
/// Range: -10000m to ~1677721.5m with 0.1m precision
pub fn encode_mapbox(elevations: &[f64], width: u32, height: u32) -> RgbImage {
    let mut img: RgbImage = ImageBuffer::new(width, height);

    for (i, elevation) in elevations.iter().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;

        let rgb = elevation_to_mapbox_rgb(*elevation);
        img.put_pixel(x, y, rgb);
    }

    img
}

/// Convert single elevation value to Mapbox Terrain-RGB
pub fn elevation_to_mapbox_rgb(elevation: f64) -> Rgb<u8> {
    // Clamp elevation to valid range
    let elevation = elevation.clamp(-10000.0, 1677721.5);

    let value = ((elevation + 10000.0) * 10.0).round() as u32;

    let r = (value / 65536) as u8;
    let g = ((value % 65536) / 256) as u8;
    let b = (value % 256) as u8;

    Rgb([r, g, b])
}

/// Decode Mapbox Terrain-RGB back to elevation (for testing)
pub fn mapbox_rgb_to_elevation(rgb: Rgb<u8>) -> f64 {
    let r = rgb.0[0] as f64;
    let g = rgb.0[1] as f64;
    let b = rgb.0[2] as f64;

    -10000.0 + ((r * 256.0 * 256.0 + g * 256.0 + b) * 0.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_roundtrip() {
        let test_values = [
            0.0, 100.0, -100.0, 8848.0,   // Everest
            -10000.0, // Minimum value
            1000.0, 5000.0,
        ];

        for &elevation in &test_values {
            let rgb = elevation_to_mapbox_rgb(elevation);
            let decoded = mapbox_rgb_to_elevation(rgb);
            // Allow for 0.1m precision
            assert!(
                (elevation - decoded).abs() < 0.15,
                "elevation={elevation}, decoded={decoded}"
            );
        }
    }

    #[test]
    fn test_zero_elevation() {
        // At elevation 0: value = 10000 * 10 = 100000
        // R = 100000 / 65536 = 1
        // G = (100000 % 65536) / 256 = 34464 / 256 = 134
        // B = 34464 % 256 = 160
        let rgb = elevation_to_mapbox_rgb(0.0);
        assert_eq!(rgb.0[0], 1);
        assert_eq!(rgb.0[1], 134);
        assert_eq!(rgb.0[2], 160);
    }

    #[test]
    fn test_sea_level() {
        // At -10000m (minimum): value = 0
        let rgb = elevation_to_mapbox_rgb(-10000.0);
        assert_eq!(rgb.0, [0, 0, 0]);
    }

    #[test]
    fn test_encode_mapbox_image() {
        let elevations = vec![0.0, 100.0, -100.0, 1000.0];
        let img = encode_mapbox(&elevations, 2, 2);

        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);

        // Check first pixel (elevation = 0)
        let pixel = img.get_pixel(0, 0);
        let decoded = mapbox_rgb_to_elevation(*pixel);
        assert!((decoded - 0.0).abs() < 0.15);
    }
}
