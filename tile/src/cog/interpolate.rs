//! Bilinear interpolation utilities.

/// Bilinear interpolation for f64 elevation data.
/// Returns NaN if any of the four surrounding pixels is NaN.
pub fn bilinear_f64(buffer: &[f64], width: usize, height: usize, x: f64, y: f64) -> f64 {
    if x < 0.0 || y < 0.0 || x >= width as f64 || y >= height as f64 {
        return f64::NAN;
    }

    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let get_pixel = |px: usize, py: usize| -> Option<f64> {
        let idx = py * width + px;
        if idx < buffer.len() {
            let v = buffer[idx];
            if v.is_nan() {
                None
            } else {
                Some(v)
            }
        } else {
            None
        }
    };

    // Get all four surrounding pixels
    let v00 = match get_pixel(x0, y0) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let v10 = match get_pixel(x1, y0) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let v01 = match get_pixel(x0, y1) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let v11 = match get_pixel(x1, y1) {
        Some(v) => v,
        None => return f64::NAN,
    };

    // Bilinear interpolation
    let v0 = v00 * (1.0 - fx) + v10 * fx;
    let v1 = v01 * (1.0 - fx) + v11 * fx;
    v0 * (1.0 - fy) + v1 * fy
}

/// Bilinear interpolation for RGBA data.
/// Uses nearest neighbor if any surrounding pixel is transparent (alpha=0).
pub fn bilinear_rgba(buffer: &[u8], width: usize, height: usize, x: f64, y: f64) -> [u8; 4] {
    if x < 0.0 || y < 0.0 || x >= width as f64 || y >= height as f64 {
        return [0, 0, 0, 0];
    }

    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

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
        // Nearest neighbor
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
        assert!((bilinear_f64(&buffer, 2, 2, 0.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((bilinear_f64(&buffer, 2, 2, 1.0, 0.0) - 10.0).abs() < 1e-6);
        assert!((bilinear_f64(&buffer, 2, 2, 0.0, 1.0) - 20.0).abs() < 1e-6);
        assert!((bilinear_f64(&buffer, 2, 2, 1.0, 1.0) - 30.0).abs() < 1e-6);

        // Center
        assert!((bilinear_f64(&buffer, 2, 2, 0.5, 0.5) - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_bilinear_f64_nan() {
        let buffer = vec![0.0, f64::NAN, 20.0, 30.0];
        assert!(bilinear_f64(&buffer, 2, 2, 0.5, 0.5).is_nan());
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

        let result = bilinear_rgba(&buffer, 2, 2, 0.5, 0.5);
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
        let result = bilinear_rgba(&buffer, 2, 2, 0.3, 0.3);
        assert_eq!(result, [255, 0, 0, 255]); // Nearest to (0,0) = red
    }
}
