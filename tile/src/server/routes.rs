//! HTTP routing configuration.

use std::sync::Arc;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use tokio::net::TcpListener;
use tower::Service;
use tower_http::trace::TraceLayer;

use super::{handlers, state::AppState};
use crate::ConfigManager;

/// Create the application router.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/tiles/{name}/{z}/{x}/{y}", get(handlers::get_tile))
        .route("/health", get(handlers::health))
        .route("/reload", post(handlers::reload))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Run the HTTP server with h2c (HTTP/2 cleartext) support.
pub async fn run(
    config_manager: Arc<ConfigManager>,
    addr: &str,
    cache_size_mb: u64,
    reload_secret: Option<String>,
) -> Result<()> {
    let state = Arc::new(AppState::new(config_manager, cache_size_mb, reload_secret).await);
    let app = create_router(state);

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on {} (HTTP/1.1 and h2c supported)", addr);

    loop {
        let (socket, remote_addr) = listener.accept().await?;
        let tower_service = app.clone();

        tokio::spawn(async move {
            let socket = TokioIo::new(socket);

            let hyper_service =
                hyper::service::service_fn(move |request| tower_service.clone().call(request));

            let builder = Builder::new(TokioExecutor::new());

            if let Err(err) = builder.serve_connection(socket, hyper_service).await {
                // Don't log connection reset errors (common with health checks)
                if !err.to_string().contains("connection reset") {
                    tracing::debug!(
                        remote_addr = %remote_addr,
                        error = %err,
                        "Connection error"
                    );
                }
            }
        });
    }
}
