use image::{ImageBuffer, Rgb, RgbImage};

/// Encode elevation data to Terrarium format RGB image.
///
/// Terrarium encoding formula:
/// - elevation = (R * 256 + G + B / 256) - 32768
///
/// Inverse (encoding):
/// - value = elevation + 32768
/// - R = floor(value / 256)
/// - G = floor(value) % 256
/// - B = floor((value - floor(value)) * 256)
pub fn encode_terrarium(elevations: &[f64], width: u32, height: u32) -> RgbImage {
    let mut img: RgbImage = ImageBuffer::new(width, height);

    for (i, elevation) in elevations.iter().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;

        let rgb = elevation_to_rgb(*elevation);
        img.put_pixel(x, y, rgb);
    }

    img
}

/// Convert single elevation value to Terrarium RGB
pub fn elevation_to_rgb(elevation: f64) -> Rgb<u8> {
    let value = elevation + 32768.0;

    let r = (value / 256.0).floor() as u8;
    let g = (value.floor() as u32 % 256) as u8;
    let b = ((value.fract()) * 256.0).floor() as u8;

    Rgb([r, g, b])
}

/// Decode Terrarium RGB back to elevation (for testing)
pub fn rgb_to_elevation(rgb: Rgb<u8>) -> f64 {
    let r = rgb.0[0] as f64;
    let g = rgb.0[1] as f64;
    let b = rgb.0[2] as f64;

    (r * 256.0 + g + b / 256.0) - 32768.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_roundtrip() {
        let test_values = [
            0.0, 100.0, -100.0, 8848.0,   // Everest
            -10994.0, // Mariana Trench (approximate)
            32767.0, -32768.0,
        ];

        for &elevation in &test_values {
            let rgb = elevation_to_rgb(elevation);
            let decoded = rgb_to_elevation(rgb);
            // Allow for some precision loss due to 8-bit encoding
            assert!(
                (elevation - decoded).abs() < 1.0,
                "elevation={elevation}, decoded={decoded}"
            );
        }
    }

    #[test]
    fn test_encode_terrarium_image() {
        let elevations = vec![0.0, 100.0, -100.0, 1000.0];
        let img = encode_terrarium(&elevations, 2, 2);

        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);

        // Check first pixel (elevation = 0)
        let pixel = img.get_pixel(0, 0);
        let decoded = rgb_to_elevation(*pixel);
        assert!((decoded - 0.0).abs() < 1.0);
    }
}
