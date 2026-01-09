use std::{env, sync::Arc};

use anyhow::{Context, Result};
use tile::{ConfigManager, cache::CacheMode, server};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Default Cache-Control header for HTTP responses.
/// 1 hour cache with must-revalidate ensures cache invalidation propagates quickly.
const DEFAULT_CACHE_CONTROL: &str = "public, max-age=3600, must-revalidate";

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "tile=info,tower_http=info".into());

    // Use tracing-stackdriver on Cloud Run (K_SERVICE is set by Cloud Run)
    if env::var("K_SERVICE").is_ok() {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_stackdriver::layer())
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

    // Load configuration
    let config_url =
        env::var("CONFIG_URL").context("CONFIG_URL environment variable is required")?;

    let port = env::var("PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    let cache_size_mb = env::var("CACHE_SIZE_MB")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(512);

    let reload_secret = env::var("RELOAD_SECRET").ok();

    // CORS origins: comma-separated list or "*" for permissive
    let cors_origins = env::var("CORS_ORIGINS").ok();

    // Preload mode: "sync" (default), "background", or "lazy"
    let preload_mode = env::var("PRELOAD_MODE")
        .map(|v| v.to_lowercase())
        .unwrap_or_else(|_| "sync".to_string());

    // Persistent cache URL (optional): file://, gs://, s3://, r2://
    let tile_cache_url = env::var("TILE_CACHE_URL").ok();

    // Cache mode: "read-write" (default) or "write-only"
    let cache_mode = env::var("TILE_CACHE_MODE")
        .map(|v| CacheMode::parse(&v))
        .unwrap_or_default();

    // Cache-Control header for HTTP responses
    let cache_control =
        Some(env::var("CACHE_CONTROL").unwrap_or_else(|_| DEFAULT_CACHE_CONTROL.to_string()));

    // Cache-Control header for stored objects in persistent cache (optional)
    let object_cache_control = env::var("TILE_CACHE_CONTROL").ok();

    tracing::info!("Loading configuration from {}", config_url);

    let config_manager = Arc::new(
        ConfigManager::new(&config_url)
            .await
            .context("Failed to load configuration")?,
    );

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
    )
    .await?;

    Ok(())
}
