//! Regression: COG reads must interpolate between pixel *centres*.
//!
//! Uses `fixtures/dem_nodata.tif` (see `cog_nodata_test.rs`): 256×256
//! float32, EPSG:4326, pixel 0.0001°, west edge 139.7, north edge 35.7056;
//! columns 0–127 hold `10 + 0.5·row + 0.25·col`, columns 128–255 are nodata.
//!
//! The reader used to feed edge-based pixel coordinates (0 = west edge of
//! pixel 0) to centre-based interpolators, so every value came from half a
//! source pixel east / south of the requested point.

use std::sync::Arc;

use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjectPath;
use tile::cog::{CogReader, TileBounds};

const WEST: f64 = 139.7;
const NORTH: f64 = 35.7056;
const PX: f64 = 0.0001;

async fn reader() -> CogReader {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let store = LocalFileSystem::new_with_prefix(dir).unwrap();
    CogReader::open(Arc::new(store), ObjectPath::from("dem_nodata.tif"))
        .await
        .unwrap()
}

/// What the fixture stores at (row, col): the ASCII value rounded to f32.
fn stored(row: u32, col: u32) -> f64 {
    let v: f64 = format!("{:.2}", 10.0 + 0.5 * row as f64 + 0.25 * col as f64)
        .parse()
        .unwrap();
    v as f32 as f64
}

fn window(col0: u32, row0: u32, n: u32) -> TileBounds {
    TileBounds {
        west: WEST + col0 as f64 * PX,
        east: WEST + (col0 + n) as f64 * PX,
        north: NORTH - row0 as f64 * PX,
        south: NORTH - (row0 + n) as f64 * PX,
    }
}

/// Output pixels that coincide with source pixels return those pixels'
/// values (to f64 rounding of the coordinates), including the raster's own
/// north / west border.
#[tokio::test]
async fn aligned_window_returns_source_pixels() {
    let r = reader().await;
    let nodata = r.nodata_from_metadata();
    for (col0, row0) in [(0u32, 0u32), (37, 101), (60, 190)] {
        let n = 64;
        let out = r
            .read_tile_elevation(&window(col0, row0, n), n, nodata)
            .await
            .unwrap();
        for j in 0..n {
            for i in 0..n {
                let (row, col) = (row0 + j, col0 + i);
                let got = out[(j * n + i) as usize];
                if col < 128 {
                    let want = stored(row, col);
                    assert!(
                        (got - want).abs() < 1e-6,
                        "window ({col0},{row0}) pixel ({col},{row}): got {got}, want {want}"
                    );
                } else {
                    assert!(
                        got.is_nan(),
                        "pixel ({col},{row}) should be nodata, got {got}"
                    );
                }
            }
        }
    }
}

/// A point on the edge shared by two source pixels returns their mean, and
/// the outer half of the raster's edge pixels is clamped (no NaN fringe).
#[tokio::test]
async fn edges_return_mean_and_raster_border_is_covered() {
    let r = reader().await;
    let nodata = r.nodata_from_metadata();
    // 2×2 output whose pixel centres sit on source-pixel corners: the output
    // window is shifted by half a source pixel and is 2 source pixels wide.
    let (c, rw) = (20u32, 30u32);
    let b = TileBounds {
        west: WEST + (c as f64 + 0.5) * PX - PX,
        east: WEST + (c as f64 + 0.5) * PX + PX,
        north: NORTH - (rw as f64 + 0.5) * PX + PX,
        south: NORTH - (rw as f64 + 0.5) * PX - PX,
    };
    let out = r.read_tile_elevation(&b, 2, nodata).await.unwrap();
    // Output pixel (0,0) is centred on the corner shared by source pixels
    // (col c-1..c, row rw-1..rw).
    let want =
        (stored(rw - 1, c - 1) + stored(rw - 1, c) + stored(rw, c - 1) + stored(rw, c)) / 4.0;
    assert!((out[0] - want).abs() < 1e-6, "got {}, want {want}", out[0]);

    // The north-west corner pixel of the raster, sampled at a point inside
    // its outer half: clamped to the pixel's value, not NaN.
    let corner = TileBounds {
        west: WEST,
        east: WEST + 0.5 * PX,
        north: NORTH,
        south: NORTH - 0.5 * PX,
    };
    let out = r.read_tile_elevation(&corner, 1, nodata).await.unwrap();
    assert!((out[0] - stored(0, 0)).abs() < 1e-6, "got {}", out[0]);
}
