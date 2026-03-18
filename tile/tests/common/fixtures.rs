//! Test fixture generation utilities.

use image::{ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;
use std::path::PathBuf;

/// Create a solid color PNG image.
pub fn create_solid_png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let img = RgbaImage::from_fn(width, height, |_, _| Rgba(color));
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("Failed to encode PNG");
    bytes
}

/// Create a red PNG tile (256x256).
pub fn create_red_tile() -> Vec<u8> {
    create_solid_png(256, 256, [255, 0, 0, 255])
}

/// Create a green PNG tile (256x256).
#[allow(dead_code)]
pub fn create_green_tile() -> Vec<u8> {
    create_solid_png(256, 256, [0, 255, 0, 255])
}

/// Create a blue PNG tile (256x256).
#[allow(dead_code)]
pub fn create_blue_tile() -> Vec<u8> {
    create_solid_png(256, 256, [0, 0, 255, 255])
}

/// Create a transparent PNG tile (256x256).
#[allow(dead_code)]
pub fn create_transparent_tile() -> Vec<u8> {
    create_solid_png(256, 256, [0, 0, 0, 0])
}

/// Create a checkerboard pattern PNG tile.
#[allow(dead_code)]
pub fn create_checkerboard_tile(color1: [u8; 4], color2: [u8; 4]) -> Vec<u8> {
    let img = RgbaImage::from_fn(256, 256, |x, y| {
        if (x / 32 + y / 32) % 2 == 0 {
            Rgba(color1)
        } else {
            Rgba(color2)
        }
    });
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("Failed to encode PNG");
    bytes
}

/// Get the path to the test fixtures directory.
#[allow(dead_code)]
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Get the path to a test COG file.
/// Note: The COG file must be created using the provided script.
#[allow(dead_code)]
pub fn test_cog_path() -> PathBuf {
    fixtures_dir().join("test.tif")
}

/// Check if the test COG file exists.
#[allow(dead_code)]
pub fn test_cog_exists() -> bool {
    test_cog_path().exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_solid_png() {
        let png = create_solid_png(16, 16, [255, 0, 0, 255]);
        assert!(!png.is_empty());
        // Verify it's a valid PNG
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn test_create_red_tile() {
        let png = create_red_tile();
        let img = image::load_from_memory(&png).expect("Failed to load PNG");
        assert_eq!(img.width(), 256);
        assert_eq!(img.height(), 256);
    }
}
