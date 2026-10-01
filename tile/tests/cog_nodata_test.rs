//! Regression: a COG's own `GDAL_NODATA` tag must be honoured.
//!
//! `fixtures/dem_nodata.tif` is a 256×256 float32 EPSG:4326 COG (128²
//! DEFLATE blocks, no overviews) over lon 139.7000–139.7256, lat
//! 35.6800–35.7056, built with `gdal_translate -of COG` from an AAIGrid with
//! `NODATA_value -9999`: the west half holds `10 + 0.5·row + 0.25·col`, the
//! east half is `-9999`. GDAL writes the tag as ASCII `"-9999"`.
//!
//! async-tiff 0.3 moved the tag out of `other_tags()`; a reader that only
//! looked there returned `None`, so DEM overlays configured without an explicit
//! `nodata` (all R2 COGs) served `-9999` as an elevation and `paint_over`
//! painted it over the layers below.

use std::sync::Arc;

use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjectPath;
use tile::terrain::{CogDemSource, DemProvider, build_composite_dem, sealevel::SeaLevelDem};

fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/dem_nodata.tif")
}

#[tokio::test]
async fn gdal_nodata_tag_is_read() {
    let store = LocalFileSystem::new_with_prefix(fixture().parent().unwrap()).unwrap();
    let reader = tile::cog::CogReader::open(Arc::new(store), ObjectPath::from("dem_nodata.tif"))
        .await
        .unwrap();
    assert_eq!(reader.nodata_from_metadata(), Some(-9999.0));
}

fn tile_of(lon: f64, lat: f64, z: u8) -> (u32, u32) {
    let n = (1u64 << z) as f64;
    let x = ((lon + 180.0) / 360.0 * n).floor() as u32;
    let y =
        ((1.0 - lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor() as u32;
    (x, y)
}

fn pixel_lonlat(z: u8, x: u32, y: u32, n: u32, i: u32, j: u32) -> (f64, f64) {
    let t = |k: u32| (k as f64 + 0.5) / n as f64;
    let zn = (1u64 << z) as f64;
    let lon = ((x as f64 + t(i)) / zn) * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * (y as f64 + t(j)) / zn))
        .sinh()
        .atan()
        .to_degrees();
    (lon, lat)
}

/// End to end: an overlay with no configured `nodata` must turn the tagged
/// sentinel into NaN, and the composite must show the base (sea level, 0 m)
/// there instead of -9999.
#[tokio::test]
async fn nodata_pixels_are_nan_and_not_painted() {
    let url = url::Url::from_file_path(fixture()).unwrap().to_string();
    let overlay: Arc<dyn DemProvider> =
        Arc::new(CogDemSource::new("nodata-test", url, None, "v", 18, 256));
    let composite = build_composite_dem(Arc::new(SeaLevelDem), vec![overlay.clone()]).await;

    // A z14 tile (lon 139.7021–139.7241) straddling the data / nodata
    // boundary (lon 139.7128).
    let (z, n) = (14u8, 256u32);
    let (x, y) = tile_of(139.7128, 35.6928, z);

    let raw = overlay.get_tile_elevations(z, x, y, n).await.unwrap();
    let comp = composite.get_tile_elevations(z, x, y, n).await.unwrap();

    let (mut data, mut nodata) = (0, 0);
    for j in 0..n {
        for i in 0..n {
            let k = (j * n + i) as usize;
            let (lon, lat) = pixel_lonlat(z, x, y, n, i, j);
            assert!(
                raw.elevations[k].is_nan() || raw.elevations[k] > -1000.0,
                "sentinel leaked from the overlay at {lon},{lat}"
            );
            assert!(
                comp.elevations[k] > -1000.0,
                "sentinel painted at {lon},{lat}"
            );
            // Well inside one half (the reader interpolates bilinearly, so
            // stay a few source pixels away from the boundary and edges).
            let inside = (35.6802..35.7054).contains(&lat);
            if inside && (139.7003..139.7124).contains(&lon) {
                assert!(
                    raw.elevations[k] > 9.0,
                    "{lon},{lat}: {}",
                    raw.elevations[k]
                );
                assert_eq!(comp.elevations[k], raw.elevations[k]);
                data += 1;
            } else if inside && (139.7132..139.7253).contains(&lon) {
                assert!(
                    raw.elevations[k].is_nan(),
                    "{lon},{lat}: {}",
                    raw.elevations[k]
                );
                assert_eq!(comp.elevations[k], 0.0, "base must show through");
                nodata += 1;
            }
        }
    }
    assert!(data > 100 && nodata > 100, "data={data} nodata={nodata}");
}
