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
