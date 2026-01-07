//! Direct COG reader tests for debugging.

mod common;

use object_store::client::ClientConfigKey;
use object_store::http::HttpBuilder;
use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjectPath;
use std::sync::Arc;

use common::fixtures;
use common::mock_cog::MockCogServer;

/// Test opening a COG file directly with the local file system reader.
#[tokio::test]
async fn test_cog_reader_local() {
    let path = fixtures::fixtures_dir().join("test_red.tif");
    if !path.exists() {
        eprintln!("Skipping test: test_red.tif not found");
        return;
    }

    // Create a local file system store
    let store = LocalFileSystem::new_with_prefix(fixtures::fixtures_dir())
        .expect("Failed to create local file system");

    let object_path = ObjectPath::from("test_red.tif");

    // Try to open the COG
    match tile::cog::CogReader::open(Arc::new(store), object_path).await {
        Ok(reader) => {
            println!("COG opened successfully!");
            println!("Bounds: {:?}", reader.bounds());
            println!("Dimensions: {:?}", reader.dimensions());
            println!("Samples per pixel: {}", reader.samples_per_pixel());

            // Try to read a tile
            if let Some(bounds) = reader.bounds() {
                println!(
                    "COG bounds: west={}, east={}, north={}, south={}",
                    bounds.west, bounds.east, bounds.north, bounds.south
                );

                // Read a tile in the center of the COG
                let tile_bounds = tile::cog::TileBounds {
                    west: bounds.west,
                    east: bounds.east,
                    north: bounds.north,
                    south: bounds.south,
                };

                match reader.read_tile_rgba(&tile_bounds, 256, None).await {
                    Ok(data) => {
                        println!("Read tile successfully! Data length: {}", data.len());
                        assert_eq!(data.len(), 256 * 256 * 4);
                    }
                    Err(e) => {
                        panic!("Failed to read tile: {e:?}");
                    }
                }
            } else {
                println!("No bounds found in COG");
            }
        }
        Err(e) => {
            panic!("Failed to open COG: {e:?}");
        }
    }
}

/// Test opening a COG file via HTTP with the mock server.
#[tokio::test]
async fn test_cog_reader_http() {
    let path = fixtures::fixtures_dir().join("test_red.tif");
    if !path.exists() {
        eprintln!("Skipping test: test_red.tif not found");
        return;
    }

    // Start mock COG server
    let cog_server = MockCogServer::start().await;
    let cog_url = cog_server.cog_url("test_red.tif");

    println!("COG URL: {cog_url}");

    // Parse the URL to get base URL and path
    let parsed_url = url::Url::parse(&cog_url).expect("Failed to parse URL");
    let base_url = format!(
        "{}://{}{}",
        parsed_url.scheme(),
        parsed_url.host_str().unwrap_or(""),
        parsed_url
            .port()
            .map(|p| format!(":{p}"))
            .unwrap_or_default()
    );
    let path = parsed_url.path();

    println!("Base URL: {base_url}");
    println!("Path: {path}");

    // Create HTTP object store (with HTTP allowed for local testing)
    let store = HttpBuilder::new()
        .with_url(&base_url)
        .with_config(ClientConfigKey::AllowHttp, "true")
        .build()
        .expect("Failed to create HTTP store");

    let object_path = ObjectPath::from(path);

    // Try to open the COG
    match tile::cog::CogReader::open(Arc::new(store), object_path).await {
        Ok(reader) => {
            println!("COG opened successfully via HTTP!");
            println!("Bounds: {:?}", reader.bounds());
            println!("Dimensions: {:?}", reader.dimensions());
            println!("Samples per pixel: {}", reader.samples_per_pixel());

            // Try to read a tile
            if let Some(bounds) = reader.bounds() {
                println!(
                    "COG bounds: west={}, east={}, north={}, south={}",
                    bounds.west, bounds.east, bounds.north, bounds.south
                );

                // Read a tile in the center of the COG
                let tile_bounds = tile::cog::TileBounds {
                    west: bounds.west,
                    east: bounds.east,
                    north: bounds.north,
                    south: bounds.south,
                };

                match reader.read_tile_rgba(&tile_bounds, 256, None).await {
                    Ok(data) => {
                        println!(
                            "Read tile via HTTP successfully! Data length: {}",
                            data.len()
                        );
                        assert_eq!(data.len(), 256 * 256 * 4);
                    }
                    Err(e) => {
                        panic!("Failed to read tile via HTTP: {e:?}");
                    }
                }
            } else {
                println!("No bounds found in COG");
            }
        }
        Err(e) => {
            panic!("Failed to open COG via HTTP: {e:?}");
        }
    }
}
