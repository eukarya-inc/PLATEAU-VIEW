//! HTTP routing configuration.

use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use http::HeaderValue;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use tokio::net::TcpListener;
use tower::Service;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use super::{
    handlers,
    state::AppState,
    terrain as terrain_handlers,
    tracing::{HttpRequestLoggingLayer, TracingMakeSpan, TracingOnRequest},
};
use crate::{ConfigManager, cache::CacheMode, terrain::TerrainSettings};

/// Create CORS layer from origins configuration.
/// - None or "*" -> permissive (allow all origins)
/// - Comma-separated list -> only allow specified origins
fn create_cors_layer(origins: Option<&str>) -> CorsLayer {
    match origins {
        None | Some("*") => CorsLayer::permissive(),
        Some(origins_str) => {
            let origins: Vec<HeaderValue> = origins_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        }
    }
}

/// Create the application router.
pub fn create_router(state: Arc<AppState>, cors_origins: Option<&str>) -> Router {
    Router::new()
        .route("/", get(handlers::viewer))
        .route("/tiles/sources.json", get(handlers::get_sources))
        .route("/tiles/catalog.json", get(handlers::get_catalog))
        .route("/tiles/{name}/tilejson.json", get(handlers::get_tilejson))
        .route("/tiles/{name}/{z}/{x}/{y}", get(handlers::get_tile))
        .route("/terrain-viewer", get(terrain_handlers::terrain_viewer))
        .route(
            "/terrain/layer.json",
            get(terrain_handlers::terrain_layer_json),
        )
        .route(
            "/terrain/{z}/{x}/{y_ext}",
            get(terrain_handlers::terrain_tile),
        )
        .route(
            "/terrarium/tilejson.json",
            get(terrain_handlers::terrarium_tilejson),
        )
        .route(
            "/terrarium/{z}/{x}/{y_ext}",
            get(terrain_handlers::terrarium_tile),
        )
        .route(
            "/mapbox/tilejson.json",
            get(terrain_handlers::mapbox_tilejson),
        )
        .route(
            "/mapbox/{z}/{x}/{y_ext}",
            get(terrain_handlers::mapbox_tile),
        )
        .route("/health", get(handlers::health))
        .route("/reload", post(handlers::reload))
        .layer(create_cors_layer(cors_origins))
        // Layer order (from outermost to innermost):
        // 1. TraceLayer - creates tracing spans, TracingOnRequest sets parent context via set_parent()
        // 2. HttpRequestLoggingLayer - logs httpRequest fields and injects trace headers to response
        .layer(HttpRequestLoggingLayer)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(TracingMakeSpan)
                .on_request(TracingOnRequest),
        )
        .with_state(state)
}

/// Run the HTTP server with h2c (HTTP/2 cleartext) support.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    config_manager: Arc<ConfigManager>,
    addr: &str,
    cache_size_mb: u64,
    reload_secret: Option<String>,
    cors_origins: Option<String>,
    preload_mode: &str,
    tile_cache_url: Option<String>,
    cache_mode: CacheMode,
    cache_control: Option<String>,
    object_cache_control: Option<String>,
    terrain_settings: TerrainSettings,
) -> Result<()> {
    let state = Arc::new(
        AppState::new(
            config_manager,
            cache_size_mb,
            reload_secret,
            preload_mode,
            tile_cache_url.as_deref(),
            cache_mode,
            cache_control,
            object_cache_control,
            terrain_settings,
        )
        .await,
    );
    let app = create_router(state, cors_origins.as_deref());

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
