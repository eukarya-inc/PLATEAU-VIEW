//! Cesium quantized-mesh-1.0 terrain tile generation
//!
//! Thin wrapper around `terrain_codec::terrain` (the one-shot heightmap →
//! quantized-mesh pipeline added in terrain-codec 0.3.0): martini RTIN
//! meshing, quantisation, header (tight horizon-occlusion point), normals
//! and gzip encoding all happen inside the crate. What remains here is the
//! PLATEAU-specific input sanitisation — corrupted DEM samples (NaN / ±inf /
//! huge COG sentinels) are clamped to 0.0 before they can reach the mesh.

pub use terrain_codec::terrain::NormalMode;

use terrain_codec::quantized_mesh::TileBounds;
use terrain_codec::terrain::{TerrainOptions, encode_terrain_from_fn};

/// Grid size for mesh generation (must be 2^n + 1).
/// 65x65 matches Cesium's heightmap format for consistency.
pub const MESH_GRID_SIZE: u32 = 65;

/// Options for generating quantized mesh tiles.
#[derive(Debug, Clone, Default)]
pub struct QuantizedMeshOptions {
    /// Maximum error threshold for mesh simplification (meters).
    /// Lower values produce more detailed meshes with more triangles.
    pub max_error: f64,
    /// Vertex-normal strategy. Prefer [`NormalMode::BufferedGradient`] with a
    /// halo-extended grid — per-tile face normals are visibly discontinuous
    /// at tile edges.
    pub normals: NormalMode,
}

/// Generate a gzipped quantized-mesh terrain tile from elevation data.
///
/// `elevations` is a 65x65 grid (row-major, north to south); `bounds` are the
/// geographic bounds of the tile in degrees.
pub fn generate_quantized_mesh_tile(
    elevations: &[f64],
    bounds: &TileBounds,
    options: QuantizedMeshOptions,
) -> Vec<u8> {
    let grid_size = MESH_GRID_SIZE as usize;
    assert!(
        elevations.len() >= grid_size * grid_size,
        "Expected at least {} elevation values, got {}",
        grid_size * grid_size,
        elevations.len()
    );

    encode_terrain_from_fn(
        MESH_GRID_SIZE,
        bounds,
        |x, y| sanitize_height(elevations[y as usize * grid_size + x as usize]),
        &TerrainOptions {
            max_error: options.max_error,
            normals: options.normals,
            ..Default::default()
        },
    )
}

/// Replace NaN, infinite, or physically-impossible elevations with 0.0 so
/// they don't propagate into the mesh as garbage vertex heights. A single
/// corrupted pixel (e.g. a bilinear-resampled `f32::MIN` nodata fringe from
/// a huge-sentinel COG) would otherwise drag the height range to ~−10³⁷,
/// blow up the bounding sphere and horizon occlusion in the quantized-mesh
/// header, and Cesium would false-cull the entire tile.
#[inline]
fn sanitize_height(h: f64) -> f64 {
    if h.is_finite() && h.abs() <= crate::cog::MAX_PHYSICAL_ELEVATION_M {
        h
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terrain_codec::quantized_mesh::{DecodedMesh, QuantizedMeshHeader};

    /// Tiles are always gzipped; decode helpers want the raw bytes.
    fn gunzip(data: &[u8]) -> Vec<u8> {
        use std::io::Read;
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(data)
            .read_to_end(&mut out)
            .expect("gunzip");
        out
    }

    #[test]
    fn test_sanitize_height() {
        assert_eq!(sanitize_height(123.4), 123.4);
        assert_eq!(sanitize_height(f64::NAN), 0.0);
        assert_eq!(sanitize_height(f64::INFINITY), 0.0);
        assert_eq!(sanitize_height(-2.7e+37), 0.0);
    }

    /// Regression for the western-Japan blackout: a single corrupted
    /// elevation (e.g. `-2.7e+37` from a bilinear-resampled `f32::MIN`
    /// nodata fringe in a COG overlay) used to drag the height range to
    /// ~−10³⁷, which then collapsed the quantized-mesh header's bounding
    /// sphere and horizon occlusion into garbage and false-culled the
    /// whole tile in Cesium.
    #[test]
    fn test_corrupt_elevation_does_not_break_mesh_header() {
        let grid_size = MESH_GRID_SIZE as usize;
        let mut elevations = vec![100.0; grid_size * grid_size];
        // Drop the same `f32::MIN`-derived fringe value the production
        // bug produced into the middle of an otherwise flat grid.
        elevations[grid_size * grid_size / 2] = -2.748191709909388e+37;

        let bounds = TileBounds::new(123.75, 33.75, 135.0, 45.0);
        let data = generate_quantized_mesh_tile(
            &elevations,
            &bounds,
            QuantizedMeshOptions {
                max_error: 0.0,
                ..Default::default()
            },
        );

        let header = QuantizedMeshHeader::from_bytes(&gunzip(&data)).expect("header");
        // Heights stay in the physical range.
        assert!(header.min_height.is_finite() && header.min_height.abs() < 50_000.0);
        assert!(header.max_height.is_finite() && header.max_height.abs() < 50_000.0);
        // Bounding sphere center is on Earth (not 10³⁶ metres out).
        let bsc_mag = (header.bounding_sphere_center[0].powi(2)
            + header.bounding_sphere_center[1].powi(2)
            + header.bounding_sphere_center[2].powi(2))
        .sqrt();
        assert!(bsc_mag < 1e8, "bounding sphere center off Earth: {bsc_mag}");
        // Horizon occlusion isn't trapped at |P|=1 (the catastrophic
        // fallback that culled the tile). For a non-hemisphere tile we
        // expect |P| > 1 from the Cesium occluder formula.
        let p_mag = (header.horizon_occlusion_point[0].powi(2)
            + header.horizon_occlusion_point[1].powi(2)
            + header.horizon_occlusion_point[2].powi(2))
        .sqrt();
        assert!(
            p_mag > 1.0,
            "horizon occluder collapsed to unit sphere: {p_mag}"
        );
    }

    #[test]
    fn test_generate_flat_terrain() {
        let grid_size = MESH_GRID_SIZE as usize;
        let elevations = vec![100.0; grid_size * grid_size];

        let bounds = TileBounds::new(139.0, 35.0, 140.0, 36.0);
        let data = generate_quantized_mesh_tile(
            &elevations,
            &bounds,
            QuantizedMeshOptions {
                max_error: 0.0,
                ..Default::default()
            },
        );

        assert_eq!(&data[0..2], &[0x1f, 0x8b]); // gzipped by default
    }

    #[test]
    fn test_generate_with_normals() {
        let grid_size = MESH_GRID_SIZE as usize;
        let elevations: Vec<f64> = (0..grid_size * grid_size)
            .map(|i| ((i % grid_size) as f64 / 8.0).sin() * 50.0)
            .collect();

        let bounds = TileBounds::new(139.0, 35.0, 140.0, 36.0);
        let opts = QuantizedMeshOptions {
            max_error: 0.0,
            normals: NormalMode::FaceNormals,
        };
        let with_normals = generate_quantized_mesh_tile(&elevations, &bounds, opts);
        let without_normals = generate_quantized_mesh_tile(
            &elevations,
            &bounds,
            QuantizedMeshOptions {
                max_error: 0.0,
                ..Default::default()
            },
        );

        assert!(with_normals.len() > without_normals.len());
    }

    #[test]
    fn test_mesh_decodes_roundtrip() {
        let grid_size = MESH_GRID_SIZE as usize;
        let elevations: Vec<f64> = (0..grid_size * grid_size)
            .map(|i| {
                let x = (i % grid_size) as f64;
                let y = (i / grid_size) as f64;
                (x / 8.0).sin() * 50.0 + (y / 8.0).cos() * 30.0
            })
            .collect();

        let bounds = TileBounds::new(139.0, 35.0, 140.0, 36.0);
        let data = generate_quantized_mesh_tile(
            &elevations,
            &bounds,
            QuantizedMeshOptions {
                max_error: 1.0,
                ..Default::default()
            },
        );

        let mesh = DecodedMesh::decode(&gunzip(&data)).expect("decode");
        assert!(mesh.vertices.len() >= 4);
        assert!(mesh.indices.len() >= 6);
    }
}
