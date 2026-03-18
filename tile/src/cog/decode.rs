//! Pixel data decoding utilities.

use async_tiff::tags::SampleFormat;

/// Decode raw bytes to RGBA pixel data.
///
/// Supports various band configurations:
/// - 1 band (grayscale) -> RGB with A=255
/// - 3 bands (RGB) -> RGBA with A=255
/// - 4 bands (RGBA) -> direct copy
pub fn decode_rgba(
    bytes: &[u8],
    width: u32,
    height: u32,
    samples_per_pixel: u16,
    bits_per_sample: u16,
) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut rgba = Vec::with_capacity(pixel_count * 4);

    match (samples_per_pixel, bits_per_sample) {
        (1, 8) => {
            // Grayscale 8-bit -> RGBA
            for &v in bytes.iter().take(pixel_count) {
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        (1, 16) => {
            // Grayscale 16-bit -> RGBA (scale to 8-bit)
            for chunk in bytes.chunks_exact(2).take(pixel_count) {
                let v = u16::from_le_bytes([chunk[0], chunk[1]]);
                let v8 = (v >> 8) as u8;
                rgba.extend_from_slice(&[v8, v8, v8, 255]);
            }
        }
        (3, 8) => {
            // RGB 8-bit -> RGBA
            for chunk in bytes.chunks_exact(3).take(pixel_count) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        (3, 16) => {
            // RGB 16-bit -> RGBA (scale to 8-bit)
            for chunk in bytes.chunks_exact(6).take(pixel_count) {
                let r = u16::from_le_bytes([chunk[0], chunk[1]]);
                let g = u16::from_le_bytes([chunk[2], chunk[3]]);
                let b = u16::from_le_bytes([chunk[4], chunk[5]]);
                rgba.extend_from_slice(&[(r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8, 255]);
            }
        }
        (4, 8) => {
            // RGBA 8-bit -> direct copy
            for chunk in bytes.chunks_exact(4).take(pixel_count) {
                rgba.extend_from_slice(chunk);
            }
        }
        (4, 16) => {
            // RGBA 16-bit -> RGBA (scale to 8-bit)
            for chunk in bytes.chunks_exact(8).take(pixel_count) {
                let r = u16::from_le_bytes([chunk[0], chunk[1]]);
                let g = u16::from_le_bytes([chunk[2], chunk[3]]);
                let b = u16::from_le_bytes([chunk[4], chunk[5]]);
                let a = u16::from_le_bytes([chunk[6], chunk[7]]);
                rgba.extend_from_slice(&[
                    (r >> 8) as u8,
                    (g >> 8) as u8,
                    (b >> 8) as u8,
                    (a >> 8) as u8,
                ]);
            }
        }
        _ => {
            // Fallback: assume grayscale 8-bit
            tracing::warn!(
                "Unknown band/bit configuration: {} bands, {} bits, assuming grayscale 8-bit",
                samples_per_pixel,
                bits_per_sample
            );
            for &v in bytes.iter().take(pixel_count) {
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
    }

    // Pad with transparent pixels if needed
    while rgba.len() < pixel_count * 4 {
        rgba.extend_from_slice(&[0, 0, 0, 0]);
    }

    rgba
}

/// Decode raw bytes to elevation (f64) data.
pub fn decode_elevation(
    bytes: &[u8],
    width: u32,
    height: u32,
    sample_format: SampleFormat,
    bits_per_sample: u16,
) -> Vec<f64> {
    let pixel_count = (width * height) as usize;
    let mut elevations = Vec::with_capacity(pixel_count);

    match (sample_format, bits_per_sample) {
        (SampleFormat::IEEEFP, 32) => {
            for chunk in bytes.chunks_exact(4) {
                let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                elevations.push(value as f64);
            }
        }
        (SampleFormat::IEEEFP, 64) => {
            for chunk in bytes.chunks_exact(8) {
                let value = f64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ]);
                elevations.push(value);
            }
        }
        (SampleFormat::Int, 16) => {
            for chunk in bytes.chunks_exact(2) {
                let value = i16::from_le_bytes([chunk[0], chunk[1]]);
                elevations.push(value as f64);
            }
        }
        (SampleFormat::Int, 32) => {
            for chunk in bytes.chunks_exact(4) {
                let value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                elevations.push(value as f64);
            }
        }
        (SampleFormat::Uint, 8) => {
            for &v in bytes.iter() {
                elevations.push(v as f64);
            }
        }
        (SampleFormat::Uint, 16) => {
            for chunk in bytes.chunks_exact(2) {
                let value = u16::from_le_bytes([chunk[0], chunk[1]]);
                elevations.push(value as f64);
            }
        }
        (SampleFormat::Uint, 32) => {
            for chunk in bytes.chunks_exact(4) {
                let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                elevations.push(value as f64);
            }
        }
        _ => {
            tracing::warn!(
                "Unknown sample format {:?} with {} bits, assuming 32-bit float",
                sample_format,
                bits_per_sample
            );
            for chunk in bytes.chunks_exact(4) {
                let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                elevations.push(value as f64);
            }
        }
    }

    // Pad with NaN if needed
    while elevations.len() < pixel_count {
        elevations.push(f64::NAN);
    }

    elevations
}

/// Get raw pixel values at a specific position for nodata checking.
/// Returns the raw band values as f64 for comparison with nodata config.
pub fn get_pixel_values(
    rgba: &[u8],
    width: usize,
    x: usize,
    y: usize,
    samples_per_pixel: u16,
) -> Vec<f64> {
    let idx = (y * width + x) * 4;
    if idx + 3 >= rgba.len() {
        return vec![];
    }

    match samples_per_pixel {
        1 => vec![rgba[idx] as f64], // Grayscale
        3 => vec![rgba[idx] as f64, rgba[idx + 1] as f64, rgba[idx + 2] as f64],
        4 => vec![
            rgba[idx] as f64,
            rgba[idx + 1] as f64,
            rgba[idx + 2] as f64,
            rgba[idx + 3] as f64,
        ],
        _ => vec![rgba[idx] as f64],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_rgba_grayscale() {
        let bytes = vec![128, 255, 0];
        let result = decode_rgba(&bytes, 3, 1, 1, 8);
        assert_eq!(result.len(), 12);
        assert_eq!(&result[0..4], &[128, 128, 128, 255]);
        assert_eq!(&result[4..8], &[255, 255, 255, 255]);
        assert_eq!(&result[8..12], &[0, 0, 0, 255]);
    }

    #[test]
    fn test_decode_rgba_rgb() {
        let bytes = vec![255, 0, 0, 0, 255, 0];
        let result = decode_rgba(&bytes, 2, 1, 3, 8);
        assert_eq!(result.len(), 8);
        assert_eq!(&result[0..4], &[255, 0, 0, 255]);
        assert_eq!(&result[4..8], &[0, 255, 0, 255]);
    }

    #[test]
    fn test_decode_rgba_rgba() {
        let bytes = vec![255, 0, 0, 128, 0, 255, 0, 64];
        let result = decode_rgba(&bytes, 2, 1, 4, 8);
        assert_eq!(result.len(), 8);
        assert_eq!(&result[0..4], &[255, 0, 0, 128]);
        assert_eq!(&result[4..8], &[0, 255, 0, 64]);
    }
}
