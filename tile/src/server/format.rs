//! Tile image format handling.

use image::ImageFormat;

/// Supported tile image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileFormat {
    Png,
    WebP,
    Avif,
}

impl TileFormat {
    /// Parse format from file extension (e.g., "png", "webp", "avif").
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "png" => Some(Self::Png),
            "webp" => Some(Self::WebP),
            "avif" => Some(Self::Avif),
            _ => None,
        }
    }

    /// Get the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
        }
    }

    /// Get the MIME content type for this format.
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
        }
    }

    /// Convert to image crate's ImageFormat.
    pub fn image_format(&self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::WebP => ImageFormat::WebP,
            Self::Avif => ImageFormat::Avif,
        }
    }
}

/// Parse "123.png" into (123, TileFormat::Png).
pub fn parse_y_and_format(y_ext: &str) -> Option<(u32, TileFormat)> {
    let (y_str, ext) = y_ext.rsplit_once('.')?;
    let y: u32 = y_str.parse().ok()?;
    let format = TileFormat::from_extension(ext)?;
    Some((y, format))
}

/// Encode an image to the specified format.
pub fn encode_image(
    img: &image::RgbaImage,
    format: TileFormat,
) -> Result<Vec<u8>, image::ImageError> {
    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    img.write_to(&mut cursor, format.image_format())?;
    Ok(bytes)
}
