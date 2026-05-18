//! Cesium quantized-mesh-1.0 terrain tile generation
//!
//! Generates binary .terrain files compatible with Cesium's quantized-mesh-1.0
//! format using the Martini RTIN algorithm for mesh simplification. Encoding,
//! the Martini implementation, and vertex-normal computation are provided by
//! the `terrain-codec` crate.

use terrain_codec::martini::Martini;
use terrain_codec::normals::{BufferedElevations, buffered_gradient_normals, face_normals};
use terrain_codec::quantized_mesh::{
    EdgeIndices, EncodeOptions, QUANTIZED_MAX, QuantizedMeshEncoder, QuantizedMeshHeader,
    QuantizedVertices, TileBounds, TileMetadata, WaterMask,
};

/// Grid size for mesh generation (must be 2^n + 1).
/// 65x65 matches Cesium's heightmap format for consistency.
pub const MESH_GRID_SIZE: u32 = 65;

/// Options for generating quantized mesh tiles.
#[derive(Debug, Clone)]
pub struct QuantizedMeshOptions {
    /// Maximum error threshold for mesh simplification (meters).
    /// Lower values produce more detailed meshes with more triangles.
    pub max_error: f64,
    /// Include oct-encoded vertex normals for lighting.
    pub include_normals: bool,
    /// Include water mask extension.
    pub include_water_mask: bool,
    /// Water mask data (if include_water_mask is true).
    pub water_mask: Option<WaterMask>,
    /// Include metadata extension with tile availability.
    pub include_metadata: bool,
    /// Tile X coordinate (used for metadata generation).
    pub tile_x: Option<u32>,
    /// Tile Y coordinate (used for metadata generation).
    pub tile_y: Option<u32>,
    /// Current zoom level of the tile (used for metadata generation).
    pub current_zoom: Option<u8>,
    /// Maximum zoom level (used for metadata generation).
    pub max_zoom: Option<u8>,
    /// Gzip compression level (0-9, default 6).
    pub compression_level: u32,
    /// When `Some`, normals are computed from the DEM gradient using this
    /// buffer-extended grid, which keeps lighting continuous across tile
    /// boundaries. When `None`, normals fall back to per-tile face-normal
    /// accumulation, which is visibly discontinuous at tile edges.
    pub buffered_elevations: Option<BufferedElevations>,
}

impl Default for QuantizedMeshOptions {
    fn default() -> Self {
        Self {
            max_error: 5.0,
            include_normals: false,
            include_water_mask: false,
            water_mask: None,
            include_metadata: false,
            tile_x: None,
            tile_y: None,
            current_zoom: None,
            max_zoom: None,
            compression_level: 6,
            buffered_elevations: None,
        }
    }
}

/// A generated quantized mesh terrain tile.
#[derive(Debug)]
pub struct QuantizedMeshTile {
    /// Gzipped binary data ready to serve.
    pub data: Vec<u8>,
    /// Number of vertices in the mesh.
    pub vertex_count: usize,
    /// Number of triangles in the mesh.
    pub triangle_count: usize,
}

/// Generate a quantized mesh terrain tile from elevation data.
///
/// # Arguments
///
/// * `elevations` - Elevation data in 65x65 grid (row-major, north to south).
/// * `bounds` - Geographic bounds of the tile (degrees).
/// * `options` - Mesh generation options.
///
/// # Returns
///
/// A quantized mesh tile ready to serve to Cesium.
pub fn generate_quantized_mesh_tile(
    elevations: &[f64],
    bounds: &TileBounds,
    options: &QuantizedMeshOptions,
) -> QuantizedMeshTile {
    let grid_size = MESH_GRID_SIZE as usize;
    assert!(
        elevations.len() >= grid_size * grid_size,
        "Expected at least {} elevation values, got {}",
        grid_size * grid_size,
        elevations.len()
    );

    let (min_height, max_height) = find_height_range(elevations);

    let mut martini = Martini::new(MESH_GRID_SIZE);

    let tile = martini.create_terrain(|x, y| {
        let idx = y * grid_size + x;
        let h = elevations.get(idx).copied().unwrap_or(0.0);
        sanitize_height(h)
    });

    let (vertices_flat, indices, _uvs) =
        tile.construct_mesh(&mut martini, options.max_error, &mut |(u, v)| {
            let lon = bounds.west + u * (bounds.east - bounds.west);
            let lat = bounds.south + v * (bounds.north - bounds.south);

            let px = (u * (grid_size - 1) as f64).round() as usize;
            let py = ((1.0 - v) * (grid_size - 1) as f64).round() as usize;
            let idx = py.min(grid_size - 1) * grid_size + px.min(grid_size - 1);
            let height = sanitize_height(elevations.get(idx).copied().unwrap_or(0.0));

            (lon, lat, height)
        });

    let vertex_count = vertices_flat.len() / 3;
    let mut vertices = QuantizedVertices::with_capacity(vertex_count);

    for i in 0..vertex_count {
        let lon = vertices_flat[i * 3] as f64;
        let lat = vertices_flat[i * 3 + 1] as f64;
        let height = vertices_flat[i * 3 + 2] as f64;

        let u = quantize_coordinate(lon, bounds.west, bounds.east);
        let v = quantize_coordinate(lat, bounds.south, bounds.north);
        let h = quantize_height(height, min_height, max_height);

        vertices.push(u, v, h);
    }

    let edge_indices = EdgeIndices::from_vertices(&vertices);

    // Create header — pass the mesh vertices so the horizon-occlusion point
    // is tight enough that Cesium doesn't false-cull tiles near the bounding
    // sphere's "equator" (e.g. anywhere in the eastern hemisphere with a small
    // ECEF Y component, like Geneva or Amsterdam). Stream the flat `Vec<f32>`
    // directly through the iterator API so no intermediate vertex buffer is
    // allocated.
    let header = QuantizedMeshHeader::from_bounds_with_vertices_iter(
        bounds,
        min_height as f32,
        max_height as f32,
        vertices_flat
            .chunks_exact(3)
            .map(|c| [c[0] as f64, c[1] as f64, c[2] as f64]),
    );

    // Prefer the DEM-gradient path when a buffered grid is available — face-
    // normal accumulation only sees triangles inside the current tile, so the
    // same physical edge is shaded inconsistently from adjacent tiles.
    let normals = if options.include_normals {
        if let Some(buffered) = &options.buffered_elevations {
            Some(buffered_gradient_normals(&vertices, bounds, buffered))
        } else {
            Some(face_normals(
                &vertices, &indices, bounds, min_height, max_height,
            ))
        }
    } else {
        None
    };

    let metadata = if options.include_metadata {
        let tile_x = options.tile_x.unwrap_or(0);
        let tile_y = options.tile_y.unwrap_or(0);
        let current_zoom = options.current_zoom.unwrap_or(0);
        let max_zoom = options.max_zoom.unwrap_or(15);
        Some(TileMetadata::for_tile(
            tile_x,
            tile_y,
            current_zoom,
            max_zoom,
        ))
    } else {
        None
    };

    let encoder = QuantizedMeshEncoder::new(header, vertices, indices.clone(), edge_indices);

    let encode_options = EncodeOptions {
        include_normals: options.include_normals,
        normals,
        include_water_mask: options.include_water_mask,
        water_mask: options.water_mask.clone(),
        include_metadata: options.include_metadata,
        metadata,
        compression_level: options.compression_level,
    };

    let data = encoder.encode_with_options(&encode_options);

    QuantizedMeshTile {
        data,
        vertex_count,
        triangle_count: indices.len() / 3,
    }
}

/// Replace NaN, infinite, or physically-impossible elevations with 0.0 so
/// they don't propagate into the mesh as garbage vertex heights. The same
/// `MAX_PHYSICAL_ELEVATION_M` threshold used by `find_height_range` is
/// applied here so a value rejected from the range computation also can't
/// sneak into a mesh vertex via the martini sample path.
#[inline]
fn sanitize_height(h: f64) -> f64 {
    if h.is_finite() && h.abs() <= crate::cog::MAX_PHYSICAL_ELEVATION_M {
        h
    } else {
        0.0
    }
}

/// Find the height range in elevation data, excluding NaN values.
///
/// Also rejects any value outside `[-MAX_PHYSICAL_ELEVATION_M,
/// MAX_PHYSICAL_ELEVATION_M]` as defense-in-depth: a single corrupted pixel
/// (e.g. a fringe value from a huge-sentinel COG that slipped earlier guards)
/// would otherwise drag `min_height` to ~−10³⁷, blow up the bounding-sphere
/// and horizon occlusion in the quantized-mesh header, and Cesium would
/// false-cull the entire tile.
fn find_height_range(elevations: &[f64]) -> (f64, f64) {
    let mut min_height = f64::MAX;
    let mut max_height = f64::MIN;

    for &h in elevations {
        if h.is_finite() && h.abs() <= crate::cog::MAX_PHYSICAL_ELEVATION_M {
            min_height = min_height.min(h);
            max_height = max_height.max(h);
        }
    }

    if min_height > max_height {
        min_height = 0.0;
        max_height = 0.0;
    }

    if (max_height - min_height).abs() < 1e-6 {
        max_height = min_height + 1.0;
    }

    (min_height, max_height)
}

#[inline]
fn quantize_coordinate(value: f64, min: f64, max: f64) -> u16 {
    let t = (value - min) / (max - min);
    (t.clamp(0.0, 1.0) * QUANTIZED_MAX as f64).round() as u16
}

#[inline]
fn quantize_height(height: f64, min_height: f64, max_height: f64) -> u16 {
    let t = (height - min_height) / (max_height - min_height);
    (t.clamp(0.0, 1.0) * QUANTIZED_MAX as f64).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_height_range() {
        let elevations = vec![10.0, 20.0, 30.0, 40.0];
        let (min, max) = find_height_range(&elevations);
        assert_eq!(min, 10.0);
        assert_eq!(max, 40.0);
    }

    #[test]
    fn test_find_height_range_with_nan() {
        let elevations = vec![10.0, f64::NAN, 30.0, f64::NAN];
        let (min, max) = find_height_range(&elevations);
        assert_eq!(min, 10.0);
        assert_eq!(max, 30.0);
    }

    #[test]
    fn test_find_height_range_all_nan() {
        let elevations = vec![f64::NAN, f64::NAN];
        let (min, max) = find_height_range(&elevations);
        assert_eq!(min, 0.0);
        assert!(max > min);
    }

    /// Regression for the western-Japan blackout: a single corrupted
    /// elevation (e.g. `-2.7e+37` from a bilinear-resampled `f32::MIN`
    /// nodata fringe in a COG overlay) used to drag `min_height` to
    /// ~−10³⁷, which then collapsed the quantized-mesh header's bounding
    /// sphere and horizon occlusion into garbage and false-culled the
    /// whole tile in Cesium.
    #[test]
    fn test_find_height_range_rejects_corrupt_outliers() {
        let elevations = vec![
            10.0,
            20.0,
            -2.7e+37, // fringe value from a huge-sentinel COG
            30.0,
            f64::INFINITY,
            40.0,
            f64::NAN,
        ];
        let (min, max) = find_height_range(&elevations);
        assert_eq!(min, 10.0);
        assert_eq!(max, 40.0);
    }

    #[test]
    fn test_corrupt_elevation_does_not_break_mesh_header() {
        let grid_size = MESH_GRID_SIZE as usize;
        let mut elevations = vec![100.0; grid_size * grid_size];
        // Drop the same `f32::MIN`-derived fringe value the production
        // bug produced into the middle of an otherwise flat grid.
        elevations[grid_size * grid_size / 2] = -2.748191709909388e+37;

        let bounds = TileBounds::new(123.75, 33.75, 135.0, 45.0);
        let options = QuantizedMeshOptions {
            max_error: 0.0,
            compression_level: 0,
            ..Default::default()
        };

        let tile = generate_quantized_mesh_tile(&elevations, &bounds, &options);

        let header = QuantizedMeshHeader::from_bytes(&tile.data).expect("header");
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
    fn test_quantize_coordinate() {
        assert_eq!(quantize_coordinate(0.0, 0.0, 100.0), 0);
        assert_eq!(quantize_coordinate(100.0, 0.0, 100.0), QUANTIZED_MAX);
        let mid = quantize_coordinate(50.0, 0.0, 100.0);
        assert!(mid == QUANTIZED_MAX / 2 || mid == QUANTIZED_MAX / 2 + 1);
    }

    #[test]
    fn test_generate_flat_terrain() {
        let grid_size = MESH_GRID_SIZE as usize;
        let elevations = vec![100.0; grid_size * grid_size];

        let bounds = TileBounds::new(139.0, 35.0, 140.0, 36.0);
        let options = QuantizedMeshOptions {
            max_error: 0.0,
            compression_level: 0,
            ..Default::default()
        };

        let tile = generate_quantized_mesh_tile(&elevations, &bounds, &options);

        assert!(tile.triangle_count > 0);
        assert!(tile.vertex_count > 0);
        assert!(!tile.data.is_empty());
    }

    #[test]
    fn test_generate_with_normals() {
        let grid_size = MESH_GRID_SIZE as usize;
        let elevations = vec![100.0; grid_size * grid_size];

        let bounds = TileBounds::new(139.0, 35.0, 140.0, 36.0);
        let options = QuantizedMeshOptions {
            max_error: 0.0,
            include_normals: true,
            compression_level: 0,
            ..Default::default()
        };

        let tile = generate_quantized_mesh_tile(&elevations, &bounds, &options);

        let options_no_normals = QuantizedMeshOptions {
            include_normals: false,
            ..options.clone()
        };
        let tile_no_normals =
            generate_quantized_mesh_tile(&elevations, &bounds, &options_no_normals);

        assert!(tile.data.len() > tile_no_normals.data.len());
    }

    #[test]
    fn test_generate_with_compression() {
        let grid_size = MESH_GRID_SIZE as usize;
        let elevations = vec![100.0; grid_size * grid_size];

        let bounds = TileBounds::new(139.0, 35.0, 140.0, 36.0);

        let options_compressed = QuantizedMeshOptions {
            compression_level: 6,
            ..Default::default()
        };
        let tile_compressed =
            generate_quantized_mesh_tile(&elevations, &bounds, &options_compressed);

        assert_eq!(&tile_compressed.data[0..2], &[0x1f, 0x8b]); // gzip magic
    }
}
