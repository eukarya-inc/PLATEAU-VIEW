//! E2E tests for the `X-Geoid-Coverage` response header on the DEM-generated
//! terrain endpoints.
//!
//! The base DEM is the built-in flat sea-level source, so no network is
//! involved; what varies between the fixtures is only where the GSIGEO2011
//! grid has a value. Requests go straight into the router (no TCP listener).

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use http::{Request, Response, StatusCode};
use tempfile::TempDir;
use tower::Service;

use tile::cache::CacheMode;
use tile::config::ConfigManager;
use tile::server::{AppState, create_router};
use tile::terrain::{GeoidModel, TerrainSettings};

const HEADER: &str = "x-geoid-coverage";

/// GSIGEO2011 fixtures inside the model's coverage bbox: inland Nagano, a z8
/// tile around Izu that is part land and part open sea, open sea south of
/// Japan, and Seoul — where the model has no value at all.
const RASTER_FULL: &str = "10/905/401";
const RASTER_PARTIAL: &str = "8/227/102";
const RASTER_NONE_SEA: &str = "10/918/422";
const RASTER_NONE_KOREA: &str = "10/873/396";
const MESH_FULL: &str = "9/905/358";
const MESH_PARTIAL: &str = "9/908/352";
const MESH_NONE: &str = "9/918/341";

struct Harness {
    app: Router,
    _config_dir: TempDir,
}

fn settings() -> TerrainSettings {
    TerrainSettings {
        dem_url: Some("sealevel".to_string()),
        dem_version: "v1".to_string(),
        dem_max_zoom: 15,
        dem_native_tile_size: 256,
        tile_size: 256,
        default_geoid: GeoidModel::Gsigeo2011,
        max_zoom: 18,
        max_error: 5.0,
        base_datum: tile::terrain::vertical::BaseDatum::Agnostic,
        base_attribution: None,
        mirror_url: None,
    }
}

impl Harness {
    /// A fresh `AppState` (empty memory cache) over `cache_dir`, so building a
    /// second one on the same directory simulates a restart that reads back
    /// what the first one wrote.
    async fn start(cache_dir: &TempDir, cors_origins: Option<&str>) -> Self {
        let config_dir = TempDir::new().unwrap();
        let config_path = config_dir.path().join("config.json");
        tokio::fs::write(&config_path, r#"{"sources":{}}"#)
            .await
            .unwrap();
        let config_url = format!("file://{}", config_path.display());
        let config_manager = Arc::new(
            ConfigManager::new(std::slice::from_ref(&config_url), Duration::from_secs(0))
                .await
                .unwrap(),
        );
        let cache_url = format!("file://{}", cache_dir.path().display());
        let state = Arc::new(
            AppState::new(
                config_manager,
                64,
                None,
                "sync",
                Some(&cache_url),
                CacheMode::ReadWrite,
                None,
                None,
                settings(),
            )
            .await,
        );
        Self {
            app: create_router(state, cors_origins),
            _config_dir: config_dir,
        }
    }

    async fn get(&self, uri: &str, origin: Option<&str>) -> (Response<Body>, Vec<u8>) {
        let mut req = Request::builder().uri(uri);
        if let Some(o) = origin {
            req = req.header("origin", o);
        }
        let resp = self
            .app
            .clone()
            .call(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let (parts, body) = resp.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .unwrap()
            .to_vec();
        (Response::from_parts(parts, Body::empty()), bytes)
    }
}

fn coverage(resp: &Response<Body>) -> Option<String> {
    resp.headers()
        .get(HEADER)
        .map(|v| v.to_str().unwrap().to_string())
}

fn raster(tile: &str, query: &str) -> String {
    format!("/terrarium/{tile}.png{query}")
}

fn mesh(tile: &str, query: &str) -> String {
    format!("/terrain/dem/{tile}.terrain{query}")
}

#[tokio::test]
async fn reports_full_partial_and_none() {
    let cache = TempDir::new().unwrap();
    let h = Harness::start(&cache, None).await;
    let cases = [
        (raster(RASTER_FULL, ""), "full"),
        (raster(RASTER_PARTIAL, ""), "partial"),
        (raster(RASTER_NONE_SEA, ""), "none"),
        (raster(RASTER_NONE_KOREA, ""), "none"),
        (raster(RASTER_PARTIAL, "?heights=geoid"), "partial"),
        (format!("/mapbox/{RASTER_PARTIAL}.png"), "partial"),
        (mesh(MESH_FULL, ""), "full"),
        (mesh(MESH_PARTIAL, ""), "partial"),
        (mesh(MESH_NONE, ""), "none"),
        (mesh(MESH_PARTIAL, "?heights=geoid"), "partial"),
    ];
    for (uri, want) in cases {
        let (resp, _) = h.get(&uri, None).await;
        assert_eq!(resp.status(), StatusCode::OK, "{uri}");
        assert_eq!(coverage(&resp).as_deref(), Some(want), "{uri}");
    }
}

#[tokio::test]
async fn orthometric_and_404_omit_the_header() {
    let cache = TempDir::new().unwrap();
    let h = Harness::start(&cache, None).await;
    for uri in [
        raster(RASTER_PARTIAL, "?heights=orthometric"),
        mesh(MESH_PARTIAL, "?heights=orthometric"),
    ] {
        let (resp, _) = h.get(&uri, None).await;
        assert_eq!(resp.status(), StatusCode::OK, "{uri}");
        assert_eq!(coverage(&resp), None, "{uri}");
    }
    // Outside the coverage bbox altogether: 404, no header.
    let (resp, _) = h.get(&raster("4/2/7", ""), None).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(coverage(&resp), None);
}

/// A repeated request (a memory-cache hit) returns the bytes, ETag and
/// coverage header of the first render. That the header is then rebuilt
/// without generating, after the coverage memo lost its entry, is covered by
/// `cache_hit_rebuilds_the_header_without_generating` in `server/terrain.rs`,
/// which can reach the process-wide memo. (A `file://` persistent cache can't
/// stand in for a restart here: it drops the `etag_hash` metadata, so its
/// entries never validate and every read regenerates.)
#[tokio::test]
async fn cache_hits_keep_the_header() {
    let cache = TempDir::new().unwrap();
    let first = Harness::start(&cache, None).await;
    for uri in [
        raster(RASTER_PARTIAL, ""),
        raster(RASTER_NONE_SEA, ""),
        mesh(MESH_PARTIAL, ""),
        mesh(MESH_FULL, ""),
    ] {
        let (rendered, body) = first.get(&uri, None).await;
        let (mem_hit, mem_body) = first.get(&uri, None).await;
        assert_eq!(mem_body, body, "{uri}");
        assert_eq!(
            mem_hit.headers().get("etag"),
            rendered.headers().get("etag")
        );
        assert_eq!(coverage(&mem_hit), coverage(&rendered), "{uri}");
        assert!(coverage(&rendered).is_some(), "{uri}");
    }
}

/// With an explicit origin list the CORS layer must expose the header, or
/// browser clients can't read it.
#[tokio::test]
async fn header_is_exposed_to_cors_clients() {
    let cache = TempDir::new().unwrap();
    let origin = "https://example.com";
    for cors in [None, Some(origin)] {
        let h = Harness::start(&cache, cors).await;
        let (resp, _) = h.get(&raster(RASTER_FULL, ""), Some(origin)).await;
        let exposed = resp
            .headers()
            .get("access-control-expose-headers")
            .map(|v| v.to_str().unwrap().to_ascii_lowercase())
            .unwrap_or_default();
        assert!(
            exposed == "*" || exposed.split(',').any(|h| h.trim() == HEADER),
            "cors {cors:?}: expose-headers = {exposed:?}"
        );
    }
}
