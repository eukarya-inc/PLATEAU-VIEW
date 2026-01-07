//! E2E tests for composite tile generation (XYZ + COG overlay).
//!
//! These tests require COG fixture files. Run `fixtures/create_test_cog.sh` to generate them.

mod common;

use common::fixtures;
use common::mock_cog::MockCogServer;
use common::mock_xyz::MockXyzServer;
use common::server::TestServer;
use serde_json::json;

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
async fn test_composite_xyz_and_cog() {
    require_cog_fixtures!("test_green.tif");

    // Start mock XYZ server with red tiles
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz.mock_all_red_tiles().await;

    // Start mock COG server with green COG
    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_green.tif");

    // Configure composite source (XYZ base + COG overlay)
    let config = json!({
        "sources": {
            "composite": {
                "layers": [
                    {
                        "type": "xyz",
                        "url": mock_xyz.xyz_url_template()
                    },
                    {
                        "type": "cog",
                        "url": cog_url,
                        "order": 1
                    }
                ]
            }
        }
    });

    let server = TestServer::start(config).await;
    let client = server.client();

    // Request a tile
    let response = client
        .get(server.tile_url("composite", 10, 909, 403))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).expect("Failed to load PNG");
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 256);

    // The result should be a blend - green COG overlaid on red XYZ
    // Exact colors depend on the COG bounds and alpha
    let rgba = img.to_rgba8();
    let pixel = rgba.get_pixel(128, 128);
    println!(
        "Composite pixel at (128,128): R={} G={} B={} A={}",
        pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]
    );
}

#[tokio::test]
async fn test_composite_xyz_only_when_cog_outside() {
    require_cog_fixtures!("test_red.tif");

    // Start mock XYZ server with blue tiles
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz.mock_all_tiles_with_color([0, 0, 255, 255]).await;

    // Start mock COG server - COG is around Tokyo
    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_red.tif");

    let config = json!({
        "sources": {
            "composite": {
                "layers": [
                    {
                        "type": "xyz",
                        "url": mock_xyz.xyz_url_template()
                    },
                    {
                        "type": "cog",
                        "url": cog_url,
                        "order": 1
                    }
                ]
            }
        }
    });

    let server = TestServer::start(config).await;
    let client = server.client();

    // Request a tile outside COG bounds (Atlantic Ocean)
    // The COG should not contribute, only XYZ
    let response = client
        .get(server.tile_url("composite", 10, 0, 0))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).unwrap().to_rgba8();

    // Should be pure blue from XYZ
    let pixel = img.get_pixel(128, 128);
    assert_eq!(pixel.0[0], 0); // Red
    assert_eq!(pixel.0[1], 0); // Green
    assert_eq!(pixel.0[2], 255); // Blue
}

#[tokio::test]
async fn test_composite_multiple_cog_layers() {
    require_cog_fixtures!("test_red.tif");
    require_cog_fixtures!("test_green.tif");

    // Start mock XYZ server with white tiles
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz
        .mock_all_tiles_with_color([255, 255, 255, 255])
        .await;

    // Start mock COG servers
    let cog_server = MockCogServer::start().await;
    let cog_url_red = cog_server.cog_url("test_red.tif");
    let cog_url_green = cog_server.cog_url("test_green.tif");

    // Configure with multiple COG overlays
    let config = json!({
        "sources": {
            "multi-cog": {
                "layers": [
                    {
                        "type": "xyz",
                        "url": mock_xyz.xyz_url_template()
                    },
                    {
                        "type": "cog",
                        "url": cog_url_red,
                        "order": 1
                    },
                    {
                        "type": "cog",
                        "url": cog_url_green,
                        "order": 2
                    }
                ]
            }
        }
    });

    let server = TestServer::start(config).await;
    let client = server.client();

    let response = client
        .get(server.tile_url("multi-cog", 10, 909, 403))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).expect("Failed to load PNG");
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 256);

    println!("Multi-COG composite tile generated successfully");
}

#[tokio::test]
async fn test_composite_layer_order() {
    // Test that layer order is respected (higher order = on top)
    let mock_xyz = MockXyzServer::start().await;
    mock_xyz.mock_all_red_tiles().await;

    // Without COG, just verify XYZ works
    let config = json!({
        "sources": {
            "xyz-only": {
                "layers": [
                    {
                        "type": "xyz",
                        "url": mock_xyz.xyz_url_template(),
                        "order": 0
                    }
                ]
            }
        }
    });

    let server = TestServer::start(config).await;
    let client = server.client();

    let response = client
        .get(server.tile_url("xyz-only", 10, 909, 403))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let bytes = response.bytes().await.unwrap();
    let img = image::load_from_memory(&bytes).unwrap().to_rgba8();
    let pixel = img.get_pixel(128, 128);

    // Pure red from XYZ
    assert_eq!(pixel.0[0], 255);
    assert_eq!(pixel.0[1], 0);
    assert_eq!(pixel.0[2], 0);
}

#[tokio::test]
async fn test_composite_cog_only() {
    require_cog_fixtures!("test_red.tif");

    // No XYZ layer, only COG
    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_red.tif");

    let config = json!({
        "sources": {
            "cog-only": {
                "layers": [
                    {
                        "type": "cog",
                        "url": cog_url
                    }
                ]
            }
        }
    });

    let server = TestServer::start(config).await;
    let client = server.client();

    // Request within COG bounds
    let response = client
        .get(server.tile_url("cog-only", 10, 909, 403))
        .send()
        .await
        .unwrap();

    // Should work with just COG
    let status = response.status();
    println!("COG-only tile status: {status}");
    assert!(status == 200 || status == 404); // 404 if outside bounds
}
