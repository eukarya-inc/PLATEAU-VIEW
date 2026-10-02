//! E2E tests for the terrain/terrarium endpoints.
//!
//! These tests stand up a wiremock server impersonating Mapterhorn, serving
//! a constant-elevation Terrarium WebP tile, and exercise the `DemProvider`
//! trait directly. Full server-level tests that go through axum routes are
//! skipped here (the endpoint logic is already covered by unit tests inside
//! `src/terrain/`).

use image::{ImageBuffer, Rgb, RgbImage};
use tile::terrain::{DemProvider, Geoid, GeoidModel, MapterhornSource};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a 512×512 Terrarium WebP tile with a constant orthometric elevation.
fn constant_terrarium_webp(elev_m: f64) -> Vec<u8> {
    let mut img: RgbImage = ImageBuffer::new(512, 512);
    // Encode the elevation into RGB per Terrarium formula.
    let value = elev_m + 32768.0;
    let r = (value / 256.0).floor() as u8;
    let g = (value.floor() as u32 % 256) as u8;
    let b = ((value.fract()) * 256.0).floor() as u8;
    for p in img.pixels_mut() {
        *p = Rgb([r, g, b]);
    }
    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    img.write_to(&mut cursor, image::ImageFormat::WebP).unwrap();
    bytes
}

#[tokio::test]
async fn mapterhorn_source_decodes_constant_elevation() {
    let server = MockServer::start().await;
    let body = constant_terrarium_webp(100.0);

    Mock::given(method("GET"))
        .and(path_regex(r"^/\d+/\d+/\d+\.webp$"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(body)
                .insert_header("etag", "\"abc123\"")
                .insert_header("content-type", "image/webp"),
        )
        .mount(&server)
        .await;

    let url = format!("{}/{{z}}/{{x}}/{{y}}.webp", server.uri());
    let src = MapterhornSource::new(url, "v1", 15);
    let tile = src
        .get_tile_elevations(10, 100, 200, 512)
        .await
        .expect("dem fetch");

    assert_eq!(tile.elevations.len(), 512 * 512);
    // Allow small rounding from 8-bit encoding round-trip.
    for h in &tile.elevations {
        assert!((h - 100.0).abs() < 1.0, "unexpected elevation: {h}");
    }
    assert_eq!(tile.etag.as_deref(), Some("abc123"));
}

/// 512×512 Terrarium WebP whose elevation is `10·row + col` metres.
fn ramp_terrarium_webp() -> Vec<u8> {
    let mut img: RgbImage = ImageBuffer::new(512, 512);
    for (col, row, p) in img.enumerate_pixels_mut() {
        let value = (10 * row + col) as f64 + 32768.0;
        *p = Rgb([(value / 256.0).floor() as u8, (value as u32 % 256) as u8, 0]);
    }
    let mut bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::WebP,
    )
    .unwrap();
    bytes
}

/// The raster endpoints ask Mapterhorn for 256 px tiles; the 512 px source
/// must be reduced between pixel centres (2×2 block means), and the reduced
/// tile — only that one — must carry the resample version in its ETag.
#[tokio::test]
async fn mapterhorn_halves_between_pixel_centres() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/\d+/\d+/\d+\.webp$"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(ramp_terrarium_webp())
                .insert_header("etag", "\"abc123\""),
        )
        .mount(&server)
        .await;
    let src = MapterhornSource::new(format!("{}/{{z}}/{{x}}/{{y}}.webp", server.uri()), "v1", 15);

    let native = src.get_tile_elevations(10, 100, 200, 512).await.unwrap();
    assert_eq!(native.etag.as_deref(), Some("abc123"));
    assert_eq!(native.elevations[511], 511.0);

    let half = src.get_tile_elevations(10, 100, 200, 256).await.unwrap();
    assert_eq!(
        half.etag,
        Some(format!(
            "abc123+resample:{}",
            tile::terrain::DEM_RESAMPLE_VERSION
        ))
    );
    // Output pixel (c, r) is the mean of source rows 2r, 2r+1 and columns
    // 2c, 2c+1: 10·(2r + 0.5) + (2c + 0.5). The corner-aligned mapping read
    // 10·r·511/255 + c·511/255 instead — off by half a source pixel at the
    // edges.
    for (c, r) in [(0u32, 0u32), (255, 0), (0, 255), (255, 255), (100, 37)] {
        let want = 10.0 * (2 * r) as f64 + 5.0 + (2 * c) as f64 + 0.5;
        let got = half.elevations[(r * 256 + c) as usize];
        assert!((got - want).abs() < 1e-9, "({c},{r}): {got} vs {want}");
    }
}

#[tokio::test]
async fn geoid_blocks_tiles_outside_japan() {
    let geoid = Geoid::load(GeoidModel::Gsigeo2011);
    // Pacific Ocean — should report no coverage.
    assert!(!geoid.bounds_have_any_coverage(-160.0, 5.0, -150.0, 15.0));
    // Tokyo — should report coverage.
    assert!(geoid.bounds_have_any_coverage(139.0, 35.0, 140.0, 36.0));
}
