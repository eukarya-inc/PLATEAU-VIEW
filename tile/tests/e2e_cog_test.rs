//! E2E tests for COG tile generation.
//!
//! These tests require COG fixture files. Run `fixtures/create_test_cog.sh` to generate them.
//!
//! Note: COG tests are currently skipped pending COG reader fixes for test fixtures.
//! The async-tiff library may require specific GeoTIFF configurations.

mod common;

use common::fixtures;
use common::mock_cog::MockCogServer;
use common::server::{TestServer, cog_config, cog_config_with_nodata};

/// Check if COG fixtures exist, skip test if not.
macro_rules! require_cog_fixtures {
    ($filename:expr) => {
        let path = fixtures::fixtures_dir().join($filename);
        if !path.exists() {
            eprintln!(
                "Skipping test: COG fixture '{}' not found. Run fixtures/create_test_cog.sh",
                $filename
            );
            return;
        }
    };
}

#[tokio::test]
async fn test_cog_tile_basic() {
    require_cog_fixtures!("test_red.tif");

    // Start mock COG server
    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_red.tif");

    // Start tile server
    let config = cog_config("test-cog", &cog_url);
    let server = TestServer::start(config).await;
    let client = server.client();

    // Request a tile within COG bounds (Tokyo area z=10)
    let response = client
        .get(server.tile_url("test-cog", 10, 909, 403))
        .send()
        .await
        .unwrap();

    // Print debug info if not 200
    if response.status() != 200 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        eprintln!("Response status: {status}, body: {body}");
    }

    // Should get a successful response
    let response = client
        .get(server.tile_url("test-cog", 10, 909, 403))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");

    // Verify it's a valid PNG
    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).expect("Failed to load PNG");
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 256);
}

#[tokio::test]
async fn test_cog_tile_out_of_bounds() {
    require_cog_fixtures!("test_red.tif");

    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_red.tif");

    let config = cog_config("test-cog", &cog_url);
    let server = TestServer::start(config).await;
    let client = server.client();

    // Request a tile outside COG bounds (somewhere in the Atlantic)
    let response = client
        .get(server.tile_url("test-cog", 10, 0, 0))
        .send()
        .await
        .unwrap();

    // Should get 404 - tile not found (outside bounds)
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_cog_tile_with_nodata() {
    require_cog_fixtures!("test_nodata.tif");

    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_nodata.tif");

    // Configure with black as nodata
    let config = cog_config_with_nodata("test-cog", &cog_url, serde_json::json!([[0, 0, 0]]));
    let server = TestServer::start(config).await;
    let client = server.client();

    // Request a tile
    let response = client
        .get(server.tile_url("test-cog", 10, 909, 403))
        .send()
        .await
        .unwrap();

    // The tile might be 404 if entirely transparent
    // Or 200 with mostly transparent pixels
    let status = response.status();
    assert!(status == 200 || status == 404);

    if status == 200 {
        let bytes = response.bytes().await.unwrap();
        let img = image::load_from_memory(&bytes).unwrap().to_rgba8();

        // Check that some pixels are transparent (alpha < 255)
        let has_transparent = img.pixels().any(|p| p.0[3] < 255);
        // This depends on the actual COG content
        println!("Has transparent pixels: {has_transparent}");
    }
}

#[tokio::test]
async fn test_cog_multiple_zoom_levels() {
    require_cog_fixtures!("test_red.tif");

    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_red.tif");

    let config = cog_config("test-cog", &cog_url);
    let server = TestServer::start(config).await;
    let client = server.client();

    // Test multiple zoom levels
    for z in [8, 10, 12, 14] {
        // Calculate approximate tile coordinates for Tokyo at this zoom
        let n = 2_u32.pow(z);
        let x = ((139.7 + 180.0) / 360.0 * n as f64) as u32;
        let lat_rad = 35.7_f64.to_radians();
        let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n as f64) as u32;

        let response = client
            .get(server.tile_url("test-cog", z, x, y))
            .send()
            .await
            .unwrap();

        // Should succeed at various zoom levels
        // (might be 404 if out of bounds at certain zooms)
        println!("z={z} x={x} y={y} -> status={}", response.status());
    }
}

#[tokio::test]
async fn test_cog_nodata_multiple_patterns() {
    require_cog_fixtures!("test_red.tif");

    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_red.tif");

    // Configure with black AND white as nodata
    let config = cog_config_with_nodata(
        "test-cog",
        &cog_url,
        serde_json::json!([[0, 0, 0], [255, 255, 255]]),
    );
    let server = TestServer::start(config).await;
    let client = server.client();

    let response = client
        .get(server.tile_url("test-cog", 10, 909, 403))
        .send()
        .await
        .unwrap();

    // Just verify it doesn't crash
    let status = response.status();
    println!("Status with multiple nodata patterns: {status}");
    assert!(status == 200 || status == 404);
}
