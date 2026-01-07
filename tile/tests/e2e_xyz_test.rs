//! E2E tests for XYZ tile proxy functionality.

mod common;

use common::fixtures;
use common::mock_xyz::MockXyzServer;
use common::server::{xyz_config, xyz_config_with_range, TestServer};

#[tokio::test]
async fn test_xyz_proxy_success() {
    // Start mock XYZ server
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz.mock_all_red_tiles().await;

    // Start tile server
    let config = xyz_config("test-xyz", &mock_xyz.xyz_url_template());
    let server = TestServer::start(config).await;
    let client = server.client();

    // Request a tile
    let response = client
        .get(server.tile_url("test-xyz", 10, 909, 403))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");

    // Verify we got a valid PNG
    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).expect("Failed to load PNG");
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 256);

    // Verify it's a red tile
    let rgba = img.to_rgba8();
    let pixel = rgba.get_pixel(128, 128);
    assert_eq!(pixel.0[0], 255); // Red
    assert_eq!(pixel.0[1], 0); // Green
    assert_eq!(pixel.0[2], 0); // Blue
}

#[tokio::test]
async fn test_xyz_proxy_404() {
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz.mock_all_tiles_not_found().await;

    let config = xyz_config("test-xyz", &mock_xyz.xyz_url_template());
    let server = TestServer::start(config).await;
    let client = server.client();

    let response = client
        .get(server.tile_url("test-xyz", 10, 909, 403))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_xyz_proxy_specific_tile() {
    let mock_xyz = MockXyzServer::start().await;

    // Mock specific tiles with different colors
    mock_xyz
        .mock_tile(10, 909, 403, fixtures::create_red_tile())
        .await;
    mock_xyz
        .mock_tile(10, 909, 404, fixtures::create_green_tile())
        .await;
    mock_xyz
        .mock_tile(10, 910, 403, fixtures::create_blue_tile())
        .await;

    let config = xyz_config("test-xyz", &mock_xyz.xyz_url_template());
    let server = TestServer::start(config).await;
    let client = server.client();

    // Request red tile
    let response = client
        .get(server.tile_url("test-xyz", 10, 909, 403))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(128, 128).0[0], 255); // Red

    // Request green tile
    let response = client
        .get(server.tile_url("test-xyz", 10, 909, 404))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(128, 128).0[1], 255); // Green

    // Request blue tile
    let response = client
        .get(server.tile_url("test-xyz", 10, 910, 403))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(img.get_pixel(128, 128).0[2], 255); // Blue
}

#[tokio::test]
async fn test_xyz_proxy_range_filter() {
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz.mock_all_red_tiles().await;

    // Configure with z range 5-15
    let config = xyz_config_with_range("test-xyz", &mock_xyz.xyz_url_template(), 5, 15);
    let server = TestServer::start(config).await;
    let client = server.client();

    // Request within range (z=10) - should succeed
    let response = client
        .get(server.tile_url("test-xyz", 10, 909, 403))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // Request below range (z=4) - should fail
    let response = client
        .get(server.tile_url("test-xyz", 4, 0, 0))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);

    // Request above range (z=16) - should fail
    let response = client
        .get(server.tile_url("test-xyz", 16, 0, 0))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_xyz_proxy_caching() {
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz.mock_all_red_tiles().await;

    let config = xyz_config("test-xyz", &mock_xyz.xyz_url_template());
    let server = TestServer::start(config).await;
    let client = server.client();

    // First request - should hit the mock server
    let response1 = client
        .get(server.tile_url("test-xyz", 10, 909, 403))
        .send()
        .await
        .unwrap();
    assert_eq!(response1.status(), 200);
    let bytes1 = response1.bytes().await.unwrap();

    // Second request - should be cached (same response)
    let response2 = client
        .get(server.tile_url("test-xyz", 10, 909, 403))
        .send()
        .await
        .unwrap();
    assert_eq!(response2.status(), 200);
    let bytes2 = response2.bytes().await.unwrap();

    // Both responses should be identical
    assert_eq!(bytes1, bytes2);
}

#[tokio::test]
async fn test_xyz_proxy_cache_header() {
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz.mock_all_red_tiles().await;

    let config = xyz_config("test-xyz", &mock_xyz.xyz_url_template());
    let server = TestServer::start(config).await;
    let client = server.client();

    let response = client
        .get(server.tile_url("test-xyz", 10, 909, 403))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    // Check cache-control header
    let cache_control = response.headers().get("cache-control").unwrap();
    assert!(cache_control.to_str().unwrap().contains("max-age"));
}
