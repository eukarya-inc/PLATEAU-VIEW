//! Pixel data decoding utilities.

use async_tiff::tags::SampleFormat;

/// Maximum plausible terrain elevation in metres. Anything beyond this magnitude
/// is not a real Earth elevation (Mt. Everest is ~8.85 km, Mariana Trench
/// ~−10.9 km), so we treat such samples as nodata regardless of the COG's
/// declared sentinel.
///
/// This guard catches resampling-blended fringe values that a strict nodata
/// equality check misses — most commonly when a COG was built with a huge
/// sentinel like `f32::MIN` (≈ −3.4 × 10³⁸) and a non-nearest resampler
/// (`-r bilinear`, `cubic`, …) blends real elevations with the sentinel at
/// every mask boundary, leaving values like `−2.7 × 10³⁷` that pass through
/// any reasonable tolerance check. Even one such pixel in a quantized-mesh
/// tile collapses the header's `min_height` / bounding-sphere / horizon
/// occlusion into garbage and false-culls the whole tile in Cesium.
pub const MAX_PHYSICAL_ELEVATION_M: f64 = 50_000.0;

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

    // Defense-in-depth: drop any value outside the physically plausible
    // elevation range. See `MAX_PHYSICAL_ELEVATION_M` for the rationale.
    for v in elevations.iter_mut() {
        if !v.is_finite() || v.abs() > MAX_PHYSICAL_ELEVATION_M {
            *v = f64::NAN;
        }
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

    /// `f32::MIN` and bilinear-fringe values near it must be sanitised to
    /// NaN at decode time so they never reach the mesh generator. See
    /// `MAX_PHYSICAL_ELEVATION_M` for the rationale.
    #[test]
    fn test_decode_elevation_rejects_huge_sentinel_and_fringes() {
        let values: [f32; 4] = [
            100.0,
            f32::MIN,   // canonical f32::MIN nodata sentinel
            -2.748e+37, // bilinear-blended fringe value seen in production
            200.0,
        ];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let result = decode_elevation(&bytes, 4, 1, SampleFormat::IEEEFP, 32);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], 100.0);
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert_eq!(result[3], 200.0);
    }

    #[test]
    fn test_decode_elevation_rejects_infinity() {
        let values: [f32; 3] = [42.0, f32::INFINITY, f32::NEG_INFINITY];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let result = decode_elevation(&bytes, 3, 1, SampleFormat::IEEEFP, 32);
        assert_eq!(result[0], 42.0);
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }
}
