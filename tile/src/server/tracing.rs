//! HTTP tracing and logging middleware.
//!
//! Provides trace context propagation from various cloud providers (W3C, Google Cloud, AWS, Cloudflare)
//! and structured logging for Google Cloud Logging.

use std::{
    future::Future,
    pin::Pin,
    task::{Context as TaskContext, Poll},
};

use http::{HeaderValue, Request};
use opentelemetry::{
    Context,
    trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState},
};
use tower::{Layer, Service};
use tower_http::trace::{MakeSpan, OnRequest};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Cloud trace context extracted from request headers.
#[derive(Debug, Clone)]
pub(crate) struct CloudTraceContext {
    pub trace_id: String,
    pub span_id: Option<String>,
    pub sampled: bool,
}

/// Custom span maker that creates a request span.
#[derive(Clone, Debug)]
pub(crate) struct TracingMakeSpan;

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
/// Sets the OpenTelemetry parent context on the span for proper trace correlation.
/// tracing-stackdriver will automatically add logging.googleapis.com/trace fields.
#[derive(Clone, Debug)]
pub(crate) struct TracingOnRequest;

impl<B> OnRequest<B> for TracingOnRequest {
    fn on_request(&mut self, request: &Request<B>, span: &Span) {
        // Extract trace context from request headers and set as parent
        // This enables tracing-stackdriver to output logging.googleapis.com/trace fields
        if let Some(otel_context) = extract_otel_context(request.headers()) {
            span.set_parent(otel_context);
        }
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
/// Returns Some(Context) if trace headers are found, None otherwise.
fn extract_otel_context(headers: &http::HeaderMap) -> Option<Context> {
    extract_trace_context(headers).map(|ctx| {
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

        let span_context = SpanContext::new(trace_id, span_id, flags, true, TraceState::default());
        Context::current().with_remote_span_context(span_context)
    })
}

/// Tower layer that logs httpRequest on response and injects trace headers.
/// Must be placed INSIDE TraceLayer so logs have trace correlation.
#[derive(Clone)]
pub(crate) struct HttpRequestLoggingLayer;

impl<S> Layer<S> for HttpRequestLoggingLayer {
    type Service = HttpRequestLoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpRequestLoggingService { inner }
    }
}

/// Tower service that logs httpRequest fields on response and injects trace headers.
#[derive(Clone)]
pub(crate) struct HttpRequestLoggingService<S> {
    inner: S,
}

/// Inject trace context headers into response.
/// Adds both W3C traceparent and Google Cloud X-Cloud-Trace-Context headers.
fn inject_trace_headers<B>(
    response: &mut http::Response<B>,
    trace_context: Option<&CloudTraceContext>,
    project_id: Option<&str>,
) {
    let Some(ctx) = trace_context else {
        return;
    };

    let trace_id = parse_otel_trace_id(&ctx.trace_id);
    let span_id = ctx
        .span_id
        .as_ref()
        .map(|s| parse_otel_span_id(s))
        .unwrap_or(SpanId::INVALID);

    // W3C traceparent: 00-{trace_id}-{span_id}-{flags}
    let flags = if ctx.sampled { "01" } else { "00" };
    let traceparent = format!("00-{trace_id}-{span_id}-{flags}");
    if let Ok(value) = HeaderValue::from_str(&traceparent) {
        response.headers_mut().insert("traceparent", value);
    }

    // Google Cloud X-Cloud-Trace-Context: TRACE_ID/SPAN_ID;o=TRACE_TRUE
    let trace_true = if ctx.sampled { "1" } else { "0" };
    let gcp_trace = format!(
        "{}/{};o={}",
        trace_id,
        u64::from_be_bytes(span_id.to_bytes()),
        trace_true
    );
    if let Ok(value) = HeaderValue::from_str(&gcp_trace) {
        response
            .headers_mut()
            .insert("x-cloud-trace-context", value);
    }

    // Add trace URL header for debugging (if project ID is available)
    if let Some(project_id) = project_id {
        let trace_url = format!(
            "https://console.cloud.google.com/traces/list?project={}&tid={}",
            project_id, trace_id
        );
        if let Ok(value) = HeaderValue::from_str(&trace_url) {
            response.headers_mut().insert("x-trace-url", value);
        }
    }
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

        // Extract trace context from request headers for response injection
        let trace_context = extract_trace_context(request.headers());

        let mut inner = self.inner.clone();
        std::mem::swap(&mut self.inner, &mut inner);

        let start = std::time::Instant::now();

        // Get GCP project ID for trace URL (optional)
        let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
            .or_else(|_| std::env::var("GCP_PROJECT"))
            .ok();

        Box::pin(async move {
            let response = inner.call(request).await;

            // Log httpRequest on response (success or error)
            let latency = start.elapsed();
            let latency_secs = latency.as_secs_f64();

            match response {
                Ok(mut res) => {
                    let status = res.status().as_u16();

                    // Inject trace headers into response
                    inject_trace_headers(&mut res, trace_context.as_ref(), project_id.as_deref());

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
                    Ok(res)
                }
                Err(e) => {
                    tracing::error!(
                        http_request.request_method = %method,
                        http_request.request_url = %uri,
                        http_request.status = 500_u16,
                        http_request.latency = format!("{:.6}s", latency_secs),
                        "--> {} {} 500",
                        method,
                        uri
                    );
                    Err(e)
                }
            }
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
