use std::{env, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::{
    Config as TraceConfig, Sampler, TracerProvider as SdkTracerProvider,
};
use tile::{ConfigManager, cache::CacheMode, server, terrain::TerrainSettings};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Default Cache-Control header for HTTP responses.
/// 1 hour cache with must-revalidate ensures cache invalidation propagates quickly.
const DEFAULT_CACHE_CONTROL: &str = "public, max-age=3600, must-revalidate";

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "tile=info,tower_http=info".into());

    // Use tracing-stackdriver on Cloud Run (K_SERVICE is set by Cloud Run)
    if env::var("K_SERVICE").is_ok() {
        // Get GCP project ID for Cloud Trace integration
        let project_id = env::var("GOOGLE_CLOUD_PROJECT")
            .or_else(|_| env::var("GCP_PROJECT"))
            .ok();

        let stackdriver_layer = if let Some(project_id) = project_id {
            tracing_stackdriver::layer()
                .with_cloud_trace(tracing_stackdriver::CloudTraceConfiguration { project_id })
        } else {
            tracing_stackdriver::layer()
        };

        // Create OpenTelemetry tracer for context propagation.
        // We use AlwaysOn sampler to ensure spans are created with proper span IDs.
        // No exporter is configured since we only need context propagation,
        // not actual trace export. tracing-stackdriver reads the OpenTelemetry context
        // to output logging.googleapis.com/trace fields.
        let provider = SdkTracerProvider::builder()
            .with_config(TraceConfig::default().with_sampler(Sampler::AlwaysOn))
            .build();
        let tracer = provider.tracer("tile-server");
        let otel_layer = OpenTelemetryLayer::new(tracer);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(otel_layer)
            .with(stackdriver_layer)
            .init();
    } else {
        // Local development: use pretty console output
        let use_ansi = std::io::IsTerminal::is_terminal(&std::io::stdout());
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_ansi(use_ansi))
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing();

    // Load configuration. CONFIG_URL is optional: when unset, the server still
    // serves the built-in terrain endpoint (`/terrain/...`, `/terrarium/...`,
    // `/terrain-viewer`). Custom XYZ/COG/MapLibre sources require CONFIG_URL.
    let config_url = env::var("CONFIG_URL").ok();

    let port = env::var("PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    // NO_CACHE=true disables all caching: memory cache set to 0, persistent
    // cache forced off, and Cache-Control overridden to prevent browser caching.
    // Useful during local iteration on terrain output.
    let no_cache = env::var("NO_CACHE")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);

    let cache_size_mb = if no_cache {
        0
    } else {
        env::var("CACHE_SIZE_MB")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(512)
    };

    let reload_secret = env::var("RELOAD_SECRET").ok();

    // CORS origins: comma-separated list or "*" for permissive
    let cors_origins = env::var("CORS_ORIGINS").ok();

    // Preload mode: "sync" (default), "background", or "lazy"
    let preload_mode = env::var("PRELOAD_MODE")
        .map(|v| v.to_lowercase())
        .unwrap_or_else(|_| "sync".to_string());

    // Persistent cache URL (optional): file://, gs://, s3://, r2://
    let tile_cache_url = if no_cache {
        None
    } else {
        env::var("TILE_CACHE_URL").ok()
    };

    // Cache mode: "read-write" (default) or "write-only"
    let cache_mode = if no_cache {
        CacheMode::None
    } else {
        env::var("TILE_CACHE_MODE")
            .map(|v| CacheMode::parse(&v))
            .unwrap_or_default()
    };

    // Cache-Control header for HTTP responses. NO_CACHE forces no-store.
    let cache_control = Some(if no_cache {
        "no-store, must-revalidate".to_string()
    } else {
        env::var("CACHE_CONTROL").unwrap_or_else(|_| DEFAULT_CACHE_CONTROL.to_string())
    });

    if no_cache {
        tracing::info!(
            "NO_CACHE=true: memory cache disabled, persistent cache off, \
             Cache-Control forced to no-store"
        );
    }

    // Cache-Control header for stored objects in persistent cache (optional)
    let object_cache_control = env::var("TILE_CACHE_CONTROL").ok();

    // Lazy revalidation TTL. Each tile/terrain request checks whether at
    // least this many seconds have elapsed since the last config check; if
    // so, exactly one in-flight request per pod refetches the config and
    // rebuilds sources when the content hash differs. `0` disables lazy
    // revalidation (manual `/reload` only).
    let config_ttl_secs = env::var("CONFIG_TTL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60);
    let config_ttl = Duration::from_secs(config_ttl_secs);

    let config_manager = Arc::new(match config_url.as_deref() {
        Some(url) => {
            tracing::info!("Loading configuration from {}", url);
            ConfigManager::new(url, config_ttl)
                .await
                .context("Failed to load configuration")?
        }
        None => {
            tracing::info!(
                "CONFIG_URL not set; starting with built-in terrain endpoint only \
                 (set CONFIG_URL to enable /tiles/... sources)"
            );
            ConfigManager::empty()
        }
    });

    if config_ttl_secs == 0 {
        tracing::info!("Lazy config revalidation disabled (CONFIG_TTL_SECS=0)");
    } else {
        tracing::info!("Lazy config revalidation TTL: {}s", config_ttl_secs);
    }

    // Start server
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("Starting tile server on {}", addr);
    tracing::info!("Cache size: {}MB", cache_size_mb);
    if let Some(ref origins) = cors_origins {
        tracing::info!("CORS origins: {}", origins);
    } else {
        tracing::info!("CORS: permissive (all origins allowed)");
    }

    if let Some(ref url) = tile_cache_url {
        tracing::info!("Persistent cache: {} (mode: {:?})", url, cache_mode);
    }

    tracing::info!("Cache-Control (HTTP): {}", cache_control.as_ref().unwrap());

    if let Some(ref cc) = object_cache_control {
        tracing::info!("Cache-Control (objects): {}", cc);
    }

    let terrain_settings = TerrainSettings::from_env();
    tracing::info!(
        dem_url = %terrain_settings.dem_url.as_deref().unwrap_or("(default Mapterhorn)"),
        default_geoid = %terrain_settings.default_geoid,
        "Terrain settings",
    );

    server::run(
        config_manager,
        &addr,
        cache_size_mb,
        reload_secret,
        cors_origins,
        &preload_mode,
        tile_cache_url,
        cache_mode,
        cache_control,
        object_cache_control,
        terrain_settings,
    )
    .await?;

    Ok(())
}
