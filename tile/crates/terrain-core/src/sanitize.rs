//! Invalid-elevation policy.

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

/// Replace NaN, infinite, or physically-impossible elevations with 0.0 so
/// they don't propagate into the mesh as garbage vertex heights. A single
/// corrupted pixel (e.g. a bilinear-resampled `f32::MIN` nodata fringe from
/// a huge-sentinel COG) would otherwise drag the height range to ~−10³⁷,
/// blow up the bounding sphere and horizon occlusion in the quantized-mesh
/// header, and Cesium would false-cull the entire tile.
///
/// This is the invalid-sample policy for every terrain encoding: the
/// Terrarium / Mapbox raster endpoints apply the same function before
/// encoding, so an invalid sample becomes 0 m there too rather than the
/// format's minimum code (−32768 m / −10000 m). Valid heights are still
/// subject to each format's own range and quantisation.
#[inline]
pub fn sanitize_height(h: f64) -> f64 {
    if h.is_finite() && h.abs() <= MAX_PHYSICAL_ELEVATION_M {
        h
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_height() {
        assert_eq!(sanitize_height(123.4), 123.4);
        assert_eq!(sanitize_height(f64::NAN), 0.0);
        assert_eq!(sanitize_height(f64::INFINITY), 0.0);
        assert_eq!(sanitize_height(-2.7e+37), 0.0);
    }
}
