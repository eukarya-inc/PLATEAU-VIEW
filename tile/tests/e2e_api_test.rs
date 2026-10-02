//! E2E tests for API endpoints (health, reload).

mod common;

use common::server::{TestServer, minimal_config};

#[tokio::test]
async fn test_health_endpoint() {
    let server = TestServer::start(minimal_config()).await;
    let client = server.client();

    let response = client.get(server.health_url()).send().await.unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "OK");
}

#[tokio::test]
async fn test_reload_without_secret() {
    let server = TestServer::start(minimal_config()).await;
    let client = server.client();

    // Without secret configured, reload should succeed
    let response = client.post(server.reload_url()).send().await.unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Configuration reloaded");
}

#[tokio::test]
async fn test_reload_with_secret_unauthorized() {
    let server = TestServer::start_with_secret(minimal_config(), "test-secret").await;
    let client = server.client();

    // Without token, should get 401
    let response = client.post(server.reload_url()).send().await.unwrap();
    assert_eq!(response.status(), 401);

    // With wrong token, should get 401
    let response = client
        .post(server.reload_url())
        .header("Authorization", "Bearer wrong-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_reload_with_secret_authorized() {
    let server = TestServer::start_with_secret(minimal_config(), "test-secret").await;
    let client = server.client();

    let response = client
        .post(server.reload_url())
        .header("Authorization", "Bearer test-secret")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Configuration reloaded");
}

#[tokio::test]
async fn test_source_not_found() {
    let server = TestServer::start(minimal_config()).await;
    let client = server.client();

    let response = client
        .get(server.tile_url("nonexistent", 10, 909, 403))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

/// `attribution` in the metadata follows each source: a config override on a
/// `/tiles` source or a DEM source wins, a sea-level DEM source credits no base
/// (no Mapterhorn), and a `/tiles` source without one keeps the PLATEAU credit.
#[tokio::test]
async fn attribution_is_per_source() {
    let layer = serde_json::json!({"type": "xyz", "url": "http://127.0.0.1:9/{z}/{x}/{y}.png"});
    let config = serde_json::json!({
        "sources": {
            "plain": { "layers": [layer] },
            "credited": { "attribution": "Ortho credit", "layers": [layer] },
            "flat": { "type": "dem", "base": "sealevel", "layers": [] },
            "custom": { "type": "dem", "base": "sealevel", "attribution": "Custom DEM", "layers": [] }
        }
    });
    let server = TestServer::start(config).await;
    let client = server.client();
    let attribution = async |path: &str| -> String {
        let r = client
            .get(format!("{}{path}", server.base_url))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200, "{path}");
        let v: serde_json::Value = r.json().await.unwrap();
        v["attribution"].as_str().unwrap().to_string()
    };

    use tile::terrain::attribution::{GSI_CREDIT, PLATEAU_CREDIT};
    assert_eq!(
        attribution("/tiles/plain/tilejson.json").await,
        PLATEAU_CREDIT
    );
    assert_eq!(
        attribution("/tiles/credited/tilejson.json").await,
        "Ortho credit"
    );
    let flat = format!("{PLATEAU_CREDIT} | {GSI_CREDIT}");
    for path in [
        "/terrain/flat/layer.json",
        "/terrarium/flat/tilejson.json",
        "/mapbox/flat/tilejson.json",
    ] {
        assert_eq!(attribution(path).await, flat, "{path}");
    }
    for path in [
        "/terrain/custom/layer.json",
        "/terrarium/custom/tilejson.json",
        "/mapbox/custom/tilejson.json",
    ] {
        assert_eq!(attribution(path).await, "Custom DEM", "{path}");
    }
}
