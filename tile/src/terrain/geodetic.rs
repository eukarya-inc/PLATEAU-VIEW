//! Geodetic TMS tile elevation fetching.
//!
//! Cesium quantized-mesh tiles live on the Global-geodetic TMS grid while
//! elevation sources serve Web Mercator XYZ tiles. The projection math —
//! tile bounds, covering-tile computation, and the actual warp (bilinear
//! resampling onto the geodetic grid) — is provided by
//! `terrain_codec::mercator::MercatorDem` (terrain-codec 0.3.0); this module
//! supplies the async side: fetching the covering XYZ tiles from a
//! [`DemProvider`] in parallel, NaN-filling failed tiles, and aggregating
//! upstream ETags and timing.

use std::time::Instant;

use terrain_codec::mercator::MercatorDem;
use terrain_codec::normals::BufferedElevations;
use terrain_codec::quantized_mesh::TileBounds;
use terrain_codec::tile_coords::{geodetic_tms, web_mercator};

use super::dem::{DemError, DemProvider};

/// Output grid size for a Cesium terrain tile (65x65, matches heightmap-1.0).
pub const CESIUM_TILE_SIZE: u32 = 65;

/// Geographic bounds for a geodetic TMS tile
#[derive(Debug, Clone, Copy)]
pub struct GeodeticBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// Timing information for geodetic tile fetch operation
#[derive(Debug, Clone, Default)]
pub struct GeodeticFetchTiming {
    /// Number of XYZ tiles fetched
    pub tiles_fetched: u32,
    /// Time spent fetching XYZ tiles (network I/O) in milliseconds
    pub xyz_fetch_ms: f64,
    /// Time spent resampling to geodetic grid in milliseconds
    pub resample_ms: f64,
}

/// Calculate the geographic bounds of a Global-geodetic TMS tile
///
/// Global-geodetic TMS:
/// - x range: 0 to 2^(z+1) - 1 (longitude: -180 to 180)
/// - y range: 0 to 2^z - 1 (latitude: -90 to 90, Y=0 is south)
pub fn geodetic_tms_bounds(z: u8, x: u32, y: u32) -> GeodeticBounds {
    let (west, south, east, north) = geodetic_tms::tile_to_bounds(z, x, y);
    GeodeticBounds {
        west,
        south,
        east,
        north,
    }
}

/// Result of a halo-extended fetch — see [`fetch_geodetic_tile_elevations_with_halo`].
#[derive(Debug)]
pub struct GeodeticFetchWithHaloResult {
    /// Tile-sized elevation grid (`CESIUM_TILE_SIZE × CESIUM_TILE_SIZE`),
    /// row-major north → south.
    pub elevations: Vec<f64>,
    /// Elevation grid extended by `halo_cells` cells on every side. Layout is
    /// `(CESIUM_TILE_SIZE + 2*halo_cells)²`, north → south, row major. The
    /// inner `CESIUM_TILE_SIZE × CESIUM_TILE_SIZE` block equals `elevations`.
    pub elevations_with_halo: Vec<f64>,
    pub halo_cells: u32,
    pub source_etags: Vec<String>,
    pub timing: GeodeticFetchTiming,
}

/// Fetch elevation data for a geodetic TMS tile by reprojecting from XYZ
/// tiles, additionally returning a grid that extends `halo_cells` cells
/// beyond the tile on every side. The halo is used by terrain mesh
/// generation to compute DEM-gradient normals that stay continuous across
/// tile boundaries: the halo cells are drawn from the same underlying XYZ
/// tiles as the interior, so adjacent geodetic tiles see identical halo
/// samples at any shared physical position.
///
/// For zooms above the upstream DEM's max, the covering XYZ tiles are
/// fetched at the DEM's `max_zoom` and the bilinear warp in `MercatorDem`
/// upsamples the relevant sub-region. Failed or missing XYZ tiles
/// contribute NaN samples.
pub async fn fetch_geodetic_tile_elevations_with_halo(
    provider: &dyn DemProvider,
    z: u8,
    geo_x: u32,
    geo_y: u32,
    xyz_tile_size: u32,
    halo_cells: u32,
) -> Result<GeodeticFetchWithHaloResult, DemError> {
    let b = geodetic_tms_bounds(z, geo_x, geo_y);
    let bounds = TileBounds::new(b.west, b.south, b.east, b.north);
    let cell_lon = (bounds.east - bounds.west) / (CESIUM_TILE_SIZE as f64 - 1.0);
    let cell_lat = (bounds.north - bounds.south) / (CESIUM_TILE_SIZE as f64 - 1.0);
    let halo = halo_cells as f64;

    // Entirely outside Web Mercator coverage (polar regions): all-NaN grids.
    if bounds.south - cell_lat * halo >= web_mercator::MAX_LAT
        || bounds.north + cell_lat * halo <= -web_mercator::MAX_LAT
    {
        let halo_size = (CESIUM_TILE_SIZE + 2 * halo_cells) as usize;
        return Ok(GeodeticFetchWithHaloResult {
            elevations: vec![f64::NAN; (CESIUM_TILE_SIZE * CESIUM_TILE_SIZE) as usize],
            elevations_with_halo: vec![f64::NAN; halo_size * halo_size],
            halo_cells,
            source_etags: Vec::new(),
            timing: GeodeticFetchTiming::default(),
        });
    }

    let fetch_z = z.min(provider.max_zoom());

    // Cover the halo-widened bounds so the halo strip samples real data.
    let (x0, y0, tiles_x, tiles_y) = MercatorDem::tiles_covering(
        fetch_z,
        bounds.west - cell_lon * halo,
        bounds.south - cell_lat * halo,
        bounds.east + cell_lon * halo,
        bounds.north + cell_lat * halo,
    );

    // Fetch the whole rectangular block in parallel, row-major.
    let fetch_start = Instant::now();
    let coords: Vec<(u32, u32)> = (0..tiles_y)
        .flat_map(|tj| (0..tiles_x).map(move |ti| (x0 + ti, y0 + tj)))
        .collect();
    let tiles_fetched = coords.len() as u32;
    let results = futures::future::join_all(coords.iter().map(|&(x, y)| async move {
        provider
            .get_tile_elevations(fetch_z, x, y, xyz_tile_size)
            .await
    }))
    .await;
    let xyz_fetch_ms = fetch_start.elapsed().as_secs_f64() * 1000.0;

    // Failed tiles become NaN blocks; `MercatorDem::sample` is NaN-tolerant.
    let nan_tile = || vec![f32::NAN; (xyz_tile_size * xyz_tile_size) as usize];
    let mut source_etags: Vec<String> = Vec::new();
    let mut tile_grids: Vec<Option<Vec<f32>>> = results
        .into_iter()
        .map(|r| match r {
            Ok(t) => {
                if let Some(etag) = t.etag {
                    source_etags.push(etag);
                }
                Some(t.elevations.iter().map(|&v| v as f32).collect())
            }
            Err(_) => None,
        })
        .collect();
    source_etags.sort();
    source_etags.dedup();

    let resample_start = Instant::now();
    let dem = MercatorDem::from_tiles(
        fetch_z,
        x0,
        y0,
        tiles_x,
        tiles_y,
        xyz_tile_size,
        // Same row-major order as `coords` above.
        |_, x, y| {
            let idx = ((y - y0) * tiles_x + (x - x0)) as usize;
            tile_grids[idx].take().unwrap_or_else(nan_tile)
        },
    );

    let elevations: Vec<f64> = dem
        .geodetic_grid(&bounds, CESIUM_TILE_SIZE)
        .iter()
        .map(|&v| v as f64)
        .collect();
    let buffered: BufferedElevations = dem.buffered_geodetic(&bounds, CESIUM_TILE_SIZE, halo_cells);
    let resample_ms = resample_start.elapsed().as_secs_f64() * 1000.0;

    Ok(GeodeticFetchWithHaloResult {
        elevations,
        elevations_with_halo: buffered.elevations,
        halo_cells,
        source_etags,
        timing: GeodeticFetchTiming {
            tiles_fetched,
            xyz_fetch_ms,
            resample_ms,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::dem::DemTile;
    use async_trait::async_trait;

    #[test]
    fn test_geodetic_tms_bounds_z0() {
        // z=0: 2 tiles in X, 1 tile in Y
        let bounds = geodetic_tms_bounds(0, 0, 0);
        assert!((bounds.west - (-180.0)).abs() < 1e-10);
        assert!((bounds.east - 0.0).abs() < 1e-10);
        assert!((bounds.south - (-90.0)).abs() < 1e-10);
        assert!((bounds.north - 90.0).abs() < 1e-10);

        let bounds = geodetic_tms_bounds(0, 1, 0);
        assert!((bounds.west - 0.0).abs() < 1e-10);
        assert!((bounds.east - 180.0).abs() < 1e-10);
    }

    #[test]
    fn test_geodetic_tms_bounds_z1() {
        // z=1: 4 tiles in X, 2 tiles in Y
        let bounds = geodetic_tms_bounds(1, 0, 0);
        assert!((bounds.west - (-180.0)).abs() < 1e-10);
        assert!((bounds.east - (-90.0)).abs() < 1e-10);
        assert!((bounds.south - (-90.0)).abs() < 1e-10);
        assert!((bounds.north - 0.0).abs() < 1e-10);
    }

    /// DEM provider serving a constant elevation everywhere.
    struct FlatDem {
        value: f64,
        max_zoom: u8,
        tile_size: u32,
    }

    #[async_trait]
    impl DemProvider for FlatDem {
        async fn get_tile_elevations(
            &self,
            _z: u8,
            _x: u32,
            _y: u32,
            tile_size: u32,
        ) -> Result<DemTile, DemError> {
            Ok(DemTile {
                elevations: vec![self.value; (tile_size * tile_size) as usize],
                etag: Some("flat".to_string()),
            })
        }

        fn native_tile_size(&self) -> u32 {
            self.tile_size
        }
        fn max_zoom(&self) -> u8 {
            self.max_zoom
        }
        fn version(&self) -> &str {
            "test"
        }
        fn slug(&self) -> &str {
            "flat"
        }
    }

    #[tokio::test]
    async fn flat_dem_resamples_flat() {
        let dem = FlatDem {
            value: 42.0,
            max_zoom: 15,
            tile_size: 64,
        };
        // A z12 tile over Mt. Fuji.
        let r = fetch_geodetic_tile_elevations_with_halo(&dem, 12, 7252, 2852, 64, 1)
            .await
            .expect("fetch");

        assert_eq!(
            r.elevations.len(),
            (CESIUM_TILE_SIZE * CESIUM_TILE_SIZE) as usize
        );
        assert!(r.elevations.iter().all(|&v| (v - 42.0).abs() < 1e-3));

        let halo_size = (CESIUM_TILE_SIZE + 2) as usize;
        assert_eq!(r.elevations_with_halo.len(), halo_size * halo_size);
        assert!(
            r.elevations_with_halo
                .iter()
                .all(|&v| (v - 42.0).abs() < 1e-3)
        );

        assert_eq!(r.source_etags, vec!["flat".to_string()]);
        assert!(r.timing.tiles_fetched > 0);
    }

    #[tokio::test]
    async fn zoom_above_max_upsamples_from_parent() {
        let dem = FlatDem {
            value: 7.0,
            max_zoom: 10,
            tile_size: 64,
        };
        // z14 request against a max-zoom-10 DEM: covering tiles are fetched
        // at z10 and the warp upsamples — values stay flat.
        let r = fetch_geodetic_tile_elevations_with_halo(&dem, 14, 29008, 11408, 64, 1)
            .await
            .expect("fetch");
        assert!(r.elevations.iter().all(|&v| (v - 7.0).abs() < 1e-3));
    }

    /// DEM provider that always fails.
    struct DeadDem;

    #[async_trait]
    impl DemProvider for DeadDem {
        async fn get_tile_elevations(
            &self,
            _z: u8,
            _x: u32,
            _y: u32,
            _tile_size: u32,
        ) -> Result<DemTile, DemError> {
            Err(DemError::NotFound)
        }

        fn native_tile_size(&self) -> u32 {
            64
        }
        fn max_zoom(&self) -> u8 {
            15
        }
        fn version(&self) -> &str {
            "test"
        }
        fn slug(&self) -> &str {
            "dead"
        }
    }

    #[tokio::test]
    async fn failed_tiles_become_nan() {
        let r = fetch_geodetic_tile_elevations_with_halo(&DeadDem, 12, 7252, 2852, 64, 1)
            .await
            .expect("fetch");
        assert!(r.elevations.iter().all(|v| v.is_nan()));
        assert!(r.source_etags.is_empty());
    }

    #[tokio::test]
    async fn polar_tile_is_all_nan() {
        let dem = FlatDem {
            value: 1.0,
            max_zoom: 15,
            tile_size: 64,
        };
        // z6 geodetic top row (87.1875°..90°N) — entirely outside Web
        // Mercator's ±85.05° coverage.
        let r = fetch_geodetic_tile_elevations_with_halo(&dem, 6, 0, 63, 64, 1)
            .await
            .expect("fetch");
        assert!(r.elevations.iter().all(|v| v.is_nan()));
        assert_eq!(r.timing.tiles_fetched, 0);
    }

    /// Adjacent geodetic tiles must agree exactly on their shared edge —
    /// both sample the same `MercatorDem` pixel-centre grid, so the shared
    /// column comes out identical without any boundary nudging.
    #[tokio::test]
    async fn adjacent_tiles_share_edge_values() {
        /// Smooth, position-dependent elevations (varies across tiles).
        struct RampDem;

        #[async_trait]
        impl DemProvider for RampDem {
            async fn get_tile_elevations(
                &self,
                z: u8,
                x: u32,
                y: u32,
                tile_size: u32,
            ) -> Result<DemTile, DemError> {
                let ts = tile_size as usize;
                let n = (1u64 << z) as f64 * tile_size as f64;
                let mut elevations = Vec::with_capacity(ts * ts);
                for py in 0..ts {
                    for px in 0..ts {
                        let gx = x as f64 * tile_size as f64 + px as f64;
                        let gy = y as f64 * tile_size as f64 + py as f64;
                        elevations.push((gx / n) * 1000.0 + (gy / n) * 500.0);
                    }
                }
                Ok(DemTile {
                    elevations,
                    etag: None,
                })
            }

            fn native_tile_size(&self) -> u32 {
                64
            }
            fn max_zoom(&self) -> u8 {
                15
            }
            fn version(&self) -> &str {
                "test"
            }
            fn slug(&self) -> &str {
                "ramp"
            }
        }

        let left = fetch_geodetic_tile_elevations_with_halo(&RampDem, 12, 7251, 2852, 64, 1)
            .await
            .expect("left");
        let right = fetch_geodetic_tile_elevations_with_halo(&RampDem, 12, 7252, 2852, 64, 1)
            .await
            .expect("right");

        let gs = CESIUM_TILE_SIZE as usize;
        for row in 0..gs {
            let l = left.elevations[row * gs + (gs - 1)]; // east edge
            let r = right.elevations[row * gs]; // west edge
            assert!(
                (l - r).abs() < 1e-9,
                "edge mismatch at row {row}: {l} vs {r}"
            );
        }
    }
}
