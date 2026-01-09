//! HTTP routing configuration.

use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use http::{HeaderValue, Request};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use tokio::net::TcpListener;
use tower::Service;
use tower_http::{
    cors::CorsLayer,
    trace::{MakeSpan, TraceLayer},
};
use tracing::Span;

use super::{handlers, state::AppState};
use crate::{ConfigManager, cache::CacheMode};

/// Custom span maker that extracts trace context from various cloud provider headers.
#[derive(Clone, Debug)]
struct TracingMakeSpan;

impl<B> MakeSpan<B> for TracingMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let trace_id = extract_trace_id(request.headers()).unwrap_or("-".to_string());

        tracing::info_span!(
            "request",
            method = %request.method(),
            uri = %request.uri(),
            trace_id = %trace_id,
        )
    }
}

/// Extract trace ID from various cloud provider headers.
/// Checks headers in order of priority:
/// 1. traceparent (W3C standard)
/// 2. X-Cloud-Trace-Context (Google Cloud)
/// 3. X-Amzn-Trace-Id (AWS X-Ray)
/// 4. cf-ray (Cloudflare)
fn extract_trace_id(headers: &http::HeaderMap) -> Option<String> {
    // W3C traceparent: 00-{trace_id}-{span_id}-{flags}
    if let Some(value) = headers.get("traceparent").and_then(|v| v.to_str().ok()) {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() >= 2 {
            return Some(parts[1].to_string());
        }
    }

    // Google Cloud: X-Cloud-Trace-Context: TRACE_ID/SPAN_ID;o=TRACE_TRUE
    if let Some(trace_id) = headers
        .get("x-cloud-trace-context")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('/').next())
    {
        return Some(trace_id.to_string());
    }

    // AWS X-Ray: X-Amzn-Trace-Id: Root=1-{timestamp}-{id};Parent=...;Sampled=...
    if let Some(value) = headers.get("x-amzn-trace-id").and_then(|v| v.to_str().ok()) {
        for part in value.split(';') {
            if let Some(root) = part.strip_prefix("Root=") {
                return Some(root.to_string());
            }
        }
    }

    // Cloudflare: cf-ray: {ray_id}-{colo}
    if let Some(value) = headers.get("cf-ray").and_then(|v| v.to_str().ok()) {
        return Some(value.to_string());
    }

    None
}

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
        .route("/tiles/{name}/tilejson.json", get(handlers::get_tilejson))
        .route("/tiles/{name}/{z}/{x}/{y}", get(handlers::get_tile))
        .route("/health", get(handlers::health))
        .route("/reload", post(handlers::reload))
        .layer(create_cors_layer(cors_origins))
        .layer(TraceLayer::new_for_http().make_span_with(TracingMakeSpan))
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
