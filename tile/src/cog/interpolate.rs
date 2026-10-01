//! Bilinear interpolation utilities.
//!
//! Coordinates are **centre-based**: `(x, y) = (k, m)` is the centre of
//! buffer pixel `(k, m)`, so a pixel's own value comes back exactly at its
//! centre and the mean of two neighbours at their shared edge. Callers that
//! start from an edge-based coordinate (e.g. [`super::bounds::geo_to_pixel_x`],
//! where 0 is the west edge of pixel 0) must subtract 0.5 first — see
//! `resample::resample_to_tile`.
//!
//! `valid_w × valid_h` is the part of the buffer that holds real raster pixels
//! (the buffer is chunk-aligned and may extend past the image's east/south
//! edge). Within half a pixel outside that extent — the outer half of the
//! raster's edge pixels — neighbours are clamped to the edge pixel, so the
//! raster's own footprint is covered right to its border with no NaN /
//! transparent fringe; beyond it the result is NaN / transparent.

/// Neighbour indices and weight along one axis, or `None` when `v` lies
/// outside the raster (more than half a pixel beyond the valid extent).
#[inline]
fn axis(v: f64, valid: usize) -> Option<(usize, usize, f64)> {
    if valid == 0 || !(v >= -0.5 && v < valid as f64 - 0.5) {
        return None;
    }
    let f0 = v.floor();
    let t = v - f0;
    let last = (valid - 1) as i64;
    let i0 = (f0 as i64).clamp(0, last) as usize;
    let i1 = (f0 as i64 + 1).clamp(0, last) as usize;
    Some((i0, i1, t))
}

/// Bilinear interpolation for f64 elevation data (centre-based coordinates).
/// Returns NaN outside the raster or if any of the four neighbours is NaN.
pub fn bilinear_f64(
    buffer: &[f64],
    width: usize,
    valid_w: usize,
    valid_h: usize,
    x: f64,
    y: f64,
) -> f64 {
    let (Some((x0, x1, fx)), Some((y0, y1, fy))) = (axis(x, valid_w), axis(y, valid_h)) else {
        return f64::NAN;
    };

    let get_pixel = |px: usize, py: usize| -> Option<f64> {
        let v = *buffer.get(py * width + px)?;
        if v.is_nan() { None } else { Some(v) }
    };

    let (Some(v00), Some(v10), Some(v01), Some(v11)) = (
        get_pixel(x0, y0),
        get_pixel(x1, y0),
        get_pixel(x0, y1),
        get_pixel(x1, y1),
    ) else {
        return f64::NAN;
    };

    let v0 = v00 * (1.0 - fx) + v10 * fx;
    let v1 = v01 * (1.0 - fx) + v11 * fx;
    v0 * (1.0 - fy) + v1 * fy
}

/// Bilinear interpolation for RGBA data (centre-based coordinates).
/// Uses nearest neighbour if any surrounding pixel is transparent (alpha=0);
/// transparent outside the raster.
pub fn bilinear_rgba(
    buffer: &[u8],
    width: usize,
    valid_w: usize,
    valid_h: usize,
    x: f64,
    y: f64,
) -> [u8; 4] {
    let (Some((x0, x1, fx)), Some((y0, y1, fy))) = (axis(x, valid_w), axis(y, valid_h)) else {
        return [0, 0, 0, 0];
    };

    let get_pixel = |px: usize, py: usize| -> [u8; 4] {
        let idx = (py * width + px) * 4;
        if idx + 3 < buffer.len() {
            [
                buffer[idx],
                buffer[idx + 1],
                buffer[idx + 2],
                buffer[idx + 3],
            ]
        } else {
            [0, 0, 0, 0]
        }
    };

    let p00 = get_pixel(x0, y0);
    let p10 = get_pixel(x1, y0);
    let p01 = get_pixel(x0, y1);
    let p11 = get_pixel(x1, y1);

    // If any pixel is transparent, use nearest neighbor to preserve sharp edges
    let any_transparent = p00[3] == 0 || p10[3] == 0 || p01[3] == 0 || p11[3] == 0;

    if any_transparent {
        // Nearest neighbour: with centre-based coordinates the nearer centre.
        let near_x = if fx < 0.5 { x0 } else { x1 };
        let near_y = if fy < 0.5 { y0 } else { y1 };
        return get_pixel(near_x, near_y);
    }

    // Bilinear interpolation for each channel
    let interpolate_channel = |c: usize| -> u8 {
        let v00 = p00[c] as f64;
        let v10 = p10[c] as f64;
        let v01 = p01[c] as f64;
        let v11 = p11[c] as f64;

        let v0 = v00 * (1.0 - fx) + v10 * fx;
        let v1 = v01 * (1.0 - fx) + v11 * fx;
        let v = v0 * (1.0 - fy) + v1 * fy;

        v.round().clamp(0.0, 255.0) as u8
    };

    [
        interpolate_channel(0),
        interpolate_channel(1),
        interpolate_channel(2),
        interpolate_channel(3),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bilinear_f64() {
        // 2x2 grid: [0, 10, 20, 30]
        let buffer = vec![0.0, 10.0, 20.0, 30.0];

        // Corner values
        assert!((bilinear_f64(&buffer, 2, 2, 2, 0.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((bilinear_f64(&buffer, 2, 2, 2, 1.0, 0.0) - 10.0).abs() < 1e-6);
        assert!((bilinear_f64(&buffer, 2, 2, 2, 0.0, 1.0) - 20.0).abs() < 1e-6);
        assert!((bilinear_f64(&buffer, 2, 2, 2, 1.0, 1.0) - 30.0).abs() < 1e-6);

        // Center
        assert!((bilinear_f64(&buffer, 2, 2, 2, 0.5, 0.5) - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_bilinear_f64_nan() {
        let buffer = vec![0.0, f64::NAN, 20.0, 30.0];
        assert!(bilinear_f64(&buffer, 2, 2, 2, 0.5, 0.5).is_nan());
    }

    #[test]
    fn test_bilinear_rgba() {
        // 2x2 RGBA: red, green, blue, white
        let buffer = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 255, 255, // white
        ];

        let result = bilinear_rgba(&buffer, 2, 2, 2, 0.5, 0.5);
        // Should be interpolated mix
        assert!(result[3] == 255); // Alpha should be 255
    }

    #[test]
    fn test_bilinear_rgba_transparent() {
        // With transparent pixel
        let buffer = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 0, // green but transparent
            0, 0, 255, 255, // blue
            255, 255, 255, 255, // white
        ];

        // Should use nearest neighbor due to transparency
        let result = bilinear_rgba(&buffer, 2, 2, 2, 0.3, 0.3);
        assert_eq!(result, [255, 0, 0, 255]); // Nearest to (0,0) = red
    }

    /// The convention the COG reader relies on: a pixel centre returns that
    /// pixel exactly, a shared edge returns the mean of its two neighbours.
    #[test]
    fn centre_returns_pixel_edge_returns_mean() {
        // 3x2 grid, row-major.
        let b = vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
        for (k, m, v) in [
            (0, 0, 1.0),
            (1, 0, 2.0),
            (2, 0, 4.0),
            (0, 1, 8.0),
            (2, 1, 32.0),
        ] {
            assert_eq!(bilinear_f64(&b, 3, 3, 2, k as f64, m as f64), v);
        }
        // Edge between (0,0) and (1,0); edge between rows at column 2.
        assert_eq!(bilinear_f64(&b, 3, 3, 2, 0.5, 0.0), 1.5);
        assert_eq!(bilinear_f64(&b, 3, 3, 2, 2.0, 0.5), 18.0);
        // Corner shared by four pixels.
        assert_eq!(
            bilinear_f64(&b, 3, 3, 2, 0.5, 0.5),
            (1.0 + 2.0 + 8.0 + 16.0) / 4.0
        );
    }

    /// The outer half of the raster's edge pixels is covered by clamping (no
    /// fringe); beyond the raster it is NaN / transparent.
    #[test]
    fn raster_edges_clamp_then_end() {
        let b = vec![1.0, 2.0, 4.0, 8.0];
        assert_eq!(bilinear_f64(&b, 2, 2, 2, -0.5, 0.0), 1.0);
        assert_eq!(bilinear_f64(&b, 2, 2, 2, 1.49, 1.0), 8.0);
        assert!(bilinear_f64(&b, 2, 2, 2, -0.51, 0.0).is_nan());
        assert!(bilinear_f64(&b, 2, 2, 2, 1.5, 0.0).is_nan());
        // A chunk-padded buffer: width 4 but only 2 valid columns; the padding
        // (NaN) is never read, the edge clamps to column 1.
        let padded = vec![1.0, 2.0, f64::NAN, f64::NAN, 4.0, 8.0, f64::NAN, f64::NAN];
        assert_eq!(bilinear_f64(&padded, 4, 2, 2, 1.3, 0.0), 2.0);
        let rgba = vec![10u8, 0, 0, 255, 20, 0, 0, 255];
        assert_eq!(bilinear_rgba(&rgba, 2, 2, 1, -0.4, 0.0), [10, 0, 0, 255]);
        assert_eq!(bilinear_rgba(&rgba, 2, 2, 1, 2.0, 0.0), [0, 0, 0, 0]);
    }
}
