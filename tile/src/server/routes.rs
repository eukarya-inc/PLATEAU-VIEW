//! HTTP routing configuration.

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context as TaskContext, Poll},
};

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
use opentelemetry::{
    Context,
    trace::{
        FutureExt as OtelFutureExt, SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId,
        TraceState,
    },
};
use tokio::net::TcpListener;
use tower::{Layer, Service};
use tower_http::{
    cors::CorsLayer,
    trace::{MakeSpan, OnRequest, TraceLayer},
};
use tracing::Span;

use super::{handlers, state::AppState};
use crate::{ConfigManager, cache::CacheMode};

/// Cloud trace context extracted from request headers.
#[derive(Debug, Clone)]
struct CloudTraceContext {
    trace_id: String,
    span_id: Option<String>,
    sampled: bool,
}

/// Custom span maker that creates a request span.
#[derive(Clone, Debug)]
struct TracingMakeSpan;

impl<B> MakeSpan<B> for TracingMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        tracing::info_span!(
            "request",
            method = %request.method(),
            uri = %request.uri(),
        )
    }
}

/// Custom on_request handler that logs when a request starts.
/// tracing-stackdriver will automatically add logging.googleapis.com/trace fields
/// from the OpenTelemetry context set by TraceContextLayer.
#[derive(Clone, Debug)]
struct TracingOnRequest;

impl<B> OnRequest<B> for TracingOnRequest {
    fn on_request(&mut self, request: &Request<B>, _span: &Span) {
        tracing::info!("<-- {} {}", request.method(), request.uri());
    }
}

/// Convert trace ID string to OpenTelemetry TraceId.
/// Handles variable-length trace IDs by left-padding with zeros to 32 hex chars.
fn parse_otel_trace_id(s: &str) -> TraceId {
    let hex: String = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(32)
        .collect();
    let padded = format!("{:0>32}", hex);
    TraceId::from_hex(&padded).unwrap_or(TraceId::INVALID)
}

/// Convert span ID string to OpenTelemetry SpanId.
/// Handles variable-length span IDs by left-padding with zeros to 16 hex chars.
fn parse_otel_span_id(s: &str) -> SpanId {
    let hex: String = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(16)
        .collect();
    let padded = format!("{:0>16}", hex);
    SpanId::from_hex(&padded).unwrap_or(SpanId::INVALID)
}

/// Extract OpenTelemetry context from HTTP headers.
fn extract_otel_context<B>(request: &Request<B>) -> Context {
    extract_trace_context(request.headers())
        .map(|ctx| {
            let trace_id = parse_otel_trace_id(&ctx.trace_id);
            let span_id = ctx
                .span_id
                .as_ref()
                .map(|s| parse_otel_span_id(s))
                .unwrap_or(SpanId::INVALID);
            let flags = if ctx.sampled {
                TraceFlags::SAMPLED
            } else {
                TraceFlags::default()
            };

            let span_context =
                SpanContext::new(trace_id, span_id, flags, true, TraceState::default());
            Context::current().with_remote_span_context(span_context)
        })
        .unwrap_or_else(Context::current)
}

/// Tower layer that propagates trace context from HTTP headers to OpenTelemetry.
/// This enables tracing-stackdriver to output logging.googleapis.com/trace fields.
/// Must be placed OUTSIDE TraceLayer so the context is set before span creation.
#[derive(Clone)]
struct TraceContextLayer;

impl<S> Layer<S> for TraceContextLayer {
    type Service = TraceContextService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceContextService { inner }
    }
}

/// Tower service that attaches OpenTelemetry context for the duration of the request.
#[derive(Clone)]
struct TraceContextService<S> {
    inner: S,
}

impl<S, B, ResBody> Service<Request<B>> for TraceContextService<S>
where
    S: Service<Request<B>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Error: Send,
    S::Future: Send,
    B: Send + 'static,
    ResBody: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        let otel_cx = extract_otel_context(&request);

        let mut inner = self.inner.clone();
        std::mem::swap(&mut self.inner, &mut inner);

        // Use with_context to attach OpenTelemetry context for the duration of this request.
        // tracing-stackdriver will read this and output logging.googleapis.com/trace fields.
        Box::pin(async move { inner.call(request).await }.with_context(otel_cx))
    }
}

/// Tower layer that logs httpRequest on response.
/// Must be placed INSIDE TraceLayer so logs have trace correlation.
#[derive(Clone)]
struct HttpRequestLoggingLayer;

impl<S> Layer<S> for HttpRequestLoggingLayer {
    type Service = HttpRequestLoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpRequestLoggingService { inner }
    }
}

/// Tower service that logs httpRequest fields on response.
#[derive(Clone)]
struct HttpRequestLoggingService<S> {
    inner: S,
}

impl<S, B, ResBody> Service<Request<B>> for HttpRequestLoggingService<S>
where
    S: Service<Request<B>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Error: Send,
    S::Future: Send,
    B: Send + 'static,
    ResBody: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        // Capture request info for httpRequest logging
        let method = request.method().to_string();
        let uri = request.uri().to_string();

        let mut inner = self.inner.clone();
        std::mem::swap(&mut self.inner, &mut inner);

        let start = std::time::Instant::now();

        Box::pin(async move {
            let response = inner.call(request).await;

            // Log httpRequest on response (success or error)
            let latency = start.elapsed();
            let latency_secs = latency.as_secs_f64();

            match &response {
                Ok(res) => {
                    let status = res.status().as_u16();
                    tracing::info!(
                        http_request.request_method = %method,
                        http_request.request_url = %uri,
                        http_request.status = status,
                        http_request.latency = format!("{:.6}s", latency_secs),
                        "--> {} {} {}",
                        method,
                        uri,
                        status
                    );
                }
                Err(_) => {
                    tracing::error!(
                        http_request.request_method = %method,
                        http_request.request_url = %uri,
                        http_request.status = 500_u16,
                        http_request.latency = format!("{:.6}s", latency_secs),
                        "--> {} {} 500",
                        method,
                        uri
                    );
                }
            }

            response
        })
    }
}

/// Extract trace context from various cloud provider headers.
/// Checks headers in order of priority:
/// 1. traceparent (W3C standard)
/// 2. X-Cloud-Trace-Context (Google Cloud)
/// 3. X-Amzn-Trace-Id (AWS X-Ray)
/// 4. cf-ray (Cloudflare)
fn extract_trace_context(headers: &http::HeaderMap) -> Option<CloudTraceContext> {
    // W3C traceparent: 00-{trace_id}-{span_id}-{flags}
    if let Some(value) = headers.get("traceparent").and_then(|v| v.to_str().ok()) {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() >= 4 {
            let sampled = parts[3].ends_with('1');
            return Some(CloudTraceContext {
                trace_id: parts[1].to_string(),
                span_id: Some(parts[2].to_string()),
                sampled,
            });
        }
    }

    // Google Cloud: X-Cloud-Trace-Context: TRACE_ID/SPAN_ID;o=TRACE_TRUE
    if let Some(value) = headers
        .get("x-cloud-trace-context")
        .and_then(|v| v.to_str().ok())
    {
        let (trace_span, options) = value.split_once(';').unwrap_or((value, ""));
        let (trace_id, span_id) = trace_span
            .split_once('/')
            .map(|(t, s)| (t.to_string(), Some(s.to_string())))
            .unwrap_or_else(|| (trace_span.to_string(), None));
        let sampled = options.contains("o=1");
        return Some(CloudTraceContext {
            trace_id,
            span_id,
            sampled,
        });
    }

    // AWS X-Ray: X-Amzn-Trace-Id: Root=1-{timestamp}-{id};Parent=...;Sampled=...
    if let Some(value) = headers.get("x-amzn-trace-id").and_then(|v| v.to_str().ok()) {
        let mut trace_id = None;
        let mut span_id = None;
        let mut sampled = false;
        for part in value.split(';') {
            let part = part.trim();
            if let Some(root) = part.strip_prefix("Root=") {
                trace_id = Some(root.to_string());
            } else if let Some(parent) = part.strip_prefix("Parent=") {
                span_id = Some(parent.to_string());
            } else if part.strip_prefix("Sampled=") == Some("1") {
                sampled = true;
            }
        }
        if let Some(trace_id) = trace_id {
            return Some(CloudTraceContext {
                trace_id,
                span_id,
                sampled,
            });
        }
    }

    // Cloudflare: cf-ray: {ray_id}-{colo}
    if let Some(value) = headers.get("cf-ray").and_then(|v| v.to_str().ok()) {
        return Some(CloudTraceContext {
            trace_id: value.to_string(),
            span_id: None,
            sampled: false,
        });
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
        // Layer order (from outermost to innermost):
        // 1. TraceContextLayer - propagates trace context from HTTP headers to OpenTelemetry
        // 2. TraceLayer - creates tracing spans with the correct trace ID
        // 3. HttpRequestLoggingLayer - logs httpRequest fields (inside span for trace correlation)
        .layer(HttpRequestLoggingLayer)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(TracingMakeSpan)
                .on_request(TracingOnRequest),
        )
        .layer(TraceContextLayer)
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
