use std::{env, sync::Arc};

use anyhow::{Context, Result};
use tile::{server, ConfigManager};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tile=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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

    server::run(config_manager, &addr, cache_size_mb, reload_secret).await?;

    Ok(())
}
