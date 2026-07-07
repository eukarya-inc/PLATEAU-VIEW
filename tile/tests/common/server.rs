//! Test server utilities.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;
use tokio::fs;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use tile::cache::CacheMode;
use tile::config::ConfigManager;
use tile::server::{AppState, create_router};
use tile::terrain::TerrainSettings;

/// A test server instance.
pub struct TestServer {
    #[allow(dead_code)]
    pub port: u16,
    pub base_url: String,
    #[allow(dead_code)]
    pub config_dir: TempDir,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

#[allow(dead_code)]
impl TestServer {
    /// Start a new test server with the given configuration.
    pub async fn start(config: serde_json::Value) -> Self {
        let config_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = config_dir.path().join("config.json");

        // Write config to file
        fs::write(&config_path, config.to_string())
            .await
            .expect("Failed to write config");

        // Find an available port
        let port = portpicker::pick_unused_port().expect("No available port");
        let addr = format!("127.0.0.1:{port}");
        let base_url = format!("http://{addr}");

        // Create config manager
        let config_url = format!("file://{}", config_path.display());
        let config_manager = Arc::new(
            ConfigManager::new(std::slice::from_ref(&config_url), Duration::from_secs(0))
                .await
                .expect("Failed to create config manager"),
        );

        // Create app state (sync preload for tests)
        let state = Arc::new(
            AppState::new(
                config_manager,
                64,
                None,
                "sync",
                None,
                CacheMode::default(),
                None,
                None, // object_cache_control
                TerrainSettings::from_env(),
            )
            .await,
        );
        let app = create_router(state, None);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Start server in background
        let listener = TcpListener::bind(&addr)
            .await
            .expect("Failed to bind listener");

        tokio::spawn(async move {
            use hyper_util::rt::{TokioExecutor, TokioIo};
            use hyper_util::server::conn::auto::Builder;
            use tower::Service;

            let builder = Builder::new(TokioExecutor::new());

            tokio::select! {
                _ = async {
                    loop {
                        if let Ok((socket, _)) = listener.accept().await {
                            let tower_service = app.clone();
                            let builder = builder.clone();

                            tokio::spawn(async move {
                                let socket = TokioIo::new(socket);
                                let hyper_service = hyper::service::service_fn(move |request| {
                                    tower_service.clone().call(request)
                                });
                                let _ = builder.serve_connection(socket, hyper_service).await;
                            });
                        }
                    }
                } => {}
                _ = shutdown_rx => {
                    // Shutdown signal received
                }
            }
        });

        // Wait for server to be ready
        Self::wait_for_server(&base_url).await;

        Self {
            port,
            base_url,
            config_dir,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Start a test server with a reload secret.
    pub async fn start_with_secret(config: serde_json::Value, secret: &str) -> Self {
        let config_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = config_dir.path().join("config.json");

        fs::write(&config_path, config.to_string())
            .await
            .expect("Failed to write config");

        let port = portpicker::pick_unused_port().expect("No available port");
        let addr = format!("127.0.0.1:{port}");
        let base_url = format!("http://{addr}");

        let config_url = format!("file://{}", config_path.display());
        let config_manager = Arc::new(
            ConfigManager::new(std::slice::from_ref(&config_url), Duration::from_secs(0))
                .await
                .expect("Failed to create config manager"),
        );

        let state = Arc::new(
            AppState::new(
                config_manager,
                64,
                Some(secret.to_string()),
                "sync",
                None,
                CacheMode::default(),
                None,
                None, // object_cache_control
                TerrainSettings::from_env(),
            )
            .await,
        );
        let app = create_router(state, None);

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let listener = TcpListener::bind(&addr)
            .await
            .expect("Failed to bind listener");

        tokio::spawn(async move {
            use hyper_util::rt::{TokioExecutor, TokioIo};
            use hyper_util::server::conn::auto::Builder;
            use tower::Service;

            let builder = Builder::new(TokioExecutor::new());

            tokio::select! {
                _ = async {
                    loop {
                        if let Ok((socket, _)) = listener.accept().await {
                            let tower_service = app.clone();
                            let builder = builder.clone();

                            tokio::spawn(async move {
                                let socket = TokioIo::new(socket);
                                let hyper_service = hyper::service::service_fn(move |request| {
                                    tower_service.clone().call(request)
                                });
                                let _ = builder.serve_connection(socket, hyper_service).await;
                            });
                        }
                    }
                } => {}
                _ = shutdown_rx => {}
            }
        });

        Self::wait_for_server(&base_url).await;

        Self {
            port,
            base_url,
            config_dir,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Start a test server with a cache-control header.
    pub async fn start_with_cache_control(config: serde_json::Value, cache_control: &str) -> Self {
        let config_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = config_dir.path().join("config.json");

        fs::write(&config_path, config.to_string())
            .await
            .expect("Failed to write config");

        let port = portpicker::pick_unused_port().expect("No available port");
        let addr = format!("127.0.0.1:{port}");
        let base_url = format!("http://{addr}");

        let config_url = format!("file://{}", config_path.display());
        let config_manager = Arc::new(
            ConfigManager::new(std::slice::from_ref(&config_url), Duration::from_secs(0))
                .await
                .expect("Failed to create config manager"),
        );

        let state = Arc::new(
            AppState::new(
                config_manager,
                64,
                None,
                "sync",
                None,
                CacheMode::default(),
                Some(cache_control.to_string()),
                None, // object_cache_control
                TerrainSettings::from_env(),
            )
            .await,
        );
        let app = create_router(state, None);

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let listener = TcpListener::bind(&addr)
            .await
            .expect("Failed to bind listener");

        tokio::spawn(async move {
            use hyper_util::rt::{TokioExecutor, TokioIo};
            use hyper_util::server::conn::auto::Builder;
            use tower::Service;

            let builder = Builder::new(TokioExecutor::new());

            tokio::select! {
                _ = async {
                    loop {
                        if let Ok((socket, _)) = listener.accept().await {
                            let tower_service = app.clone();
                            let builder = builder.clone();

                            tokio::spawn(async move {
                                let socket = TokioIo::new(socket);
                                let hyper_service = hyper::service::service_fn(move |request| {
                                    tower_service.clone().call(request)
                                });
                                let _ = builder.serve_connection(socket, hyper_service).await;
                            });
                        }
                    }
                } => {}
                _ = shutdown_rx => {}
            }
        });

        Self::wait_for_server(&base_url).await;

        Self {
            port,
            base_url,
            config_dir,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Wait for the server to be ready.
    async fn wait_for_server(base_url: &str) {
        let client = reqwest::Client::new();
        let health_url = format!("{base_url}/health");

        for _ in 0..50 {
            if client.get(&health_url).send().await.is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("Server did not start within 5 seconds");
    }

    /// Get the URL for a tile.
    pub fn tile_url(&self, name: &str, z: u32, x: u32, y: u32) -> String {
        format!("{}/tiles/{name}/{z}/{x}/{y}.png", self.base_url)
    }

    /// Get the health check URL.
    pub fn health_url(&self) -> String {
        format!("{}/health", self.base_url)
    }

    /// Get the reload URL.
    pub fn reload_url(&self) -> String {
        format!("{}/reload", self.base_url)
    }

    /// Update the configuration file.
    pub async fn update_config(&self, config: serde_json::Value) {
        let config_path = self.config_dir.path().join("config.json");
        fs::write(&config_path, config.to_string())
            .await
            .expect("Failed to write config");
    }

    /// Create an HTTP client.
    pub fn client(&self) -> reqwest::Client {
        reqwest::Client::new()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Create a minimal test configuration.
#[allow(dead_code)]
pub fn minimal_config() -> serde_json::Value {
    json!({
        "sources": {}
    })
}

/// Create a configuration with an XYZ source.
#[allow(dead_code)]
pub fn xyz_config(name: &str, url_template: &str) -> serde_json::Value {
    json!({
        "sources": {
            name: {
                "layers": [
                    {
                        "type": "xyz",
                        "url": url_template
                    }
                ]
            }
        }
    })
}

/// Create a configuration with an XYZ source and range.
#[allow(dead_code)]
pub fn xyz_config_with_range(
    name: &str,
    url_template: &str,
    z_min: u32,
    z_max: u32,
) -> serde_json::Value {
    json!({
        "sources": {
            name: {
                "layers": [
                    {
                        "type": "xyz",
                        "url": url_template,
                        "range": {
                            "z_min": z_min,
                            "z_max": z_max
                        }
                    }
                ]
            }
        }
    })
}

/// Create a configuration with a COG source.
#[allow(dead_code)]
pub fn cog_config(name: &str, cog_url: &str) -> serde_json::Value {
    json!({
        "sources": {
            name: {
                "layers": [
                    {
                        "type": "cog",
                        "url": cog_url
                    }
                ]
            }
        }
    })
}

/// Create a configuration with a COG source and nodata.
#[allow(dead_code)]
pub fn cog_config_with_nodata(
    name: &str,
    cog_url: &str,
    nodata: serde_json::Value,
) -> serde_json::Value {
    json!({
        "sources": {
            name: {
                "layers": [
                    {
                        "type": "cog",
                        "url": cog_url,
                        "nodata": nodata
                    }
                ]
            }
        }
    })
}

/// Create a configuration with XYZ base and COG overlay.
#[allow(dead_code)]
pub fn composite_config(name: &str, xyz_url_template: &str, cog_url: &str) -> serde_json::Value {
    json!({
        "sources": {
            name: {
                "layers": [
                    {
                        "type": "xyz",
                        "url": xyz_url_template
                    },
                    {
                        "type": "cog",
                        "url": cog_url,
                        "order": 1
                    }
                ]
            }
        }
    })
}
