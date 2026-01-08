//! Application state management.

use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use tokio::sync::RwLock;

use crate::{
    cache::TileCache,
    config::{ConfigManager, LayerConfig, SourceConfig},
    tile::{CogTileSource, CompositeTileSource, MaplibreTileSource, TileSource, XyzTileSource},
};

/// Application state shared across handlers.
pub struct AppState {
    pub config_manager: Arc<ConfigManager>,
    pub cache: Arc<TileCache>,
    /// Cached tile sources (rebuilt on config reload)
    sources: Arc<RwLock<HashMap<String, Arc<dyn TileSource>>>>,
    /// Secret for reload endpoint authorization
    pub reload_secret: Option<String>,
}

impl AppState {
    pub async fn new(
        config_manager: Arc<ConfigManager>,
        cache_size_mb: u64,
        reload_secret: Option<String>,
        preload_mode: &str,
    ) -> Self {
        let config = config_manager.get().await;

        let cache = Arc::new(TileCache::new(cache_size_mb));

        let sources = Self::build_sources(&config.sources);

        // Preload sources based on mode
        match preload_mode {
            "sync" => {
                // Sync preload (block until complete - better for Cloud Run)
                Self::preload_sources(&sources).await;
            }
            "lazy" => {
                // Lazy mode: don't preload, metadata will be loaded on first request
                tracing::info!("Preload mode: lazy (metadata loaded on first request)");
            }
            _ => {
                // Background preload (default - don't block startup)
                let sources_for_preload = sources.clone();
                tokio::spawn(async move {
                    Self::preload_sources(&sources_for_preload).await;
                });
            }
        }

        Self {
            config_manager,
            cache,
            sources: Arc::new(RwLock::new(sources)),
            reload_secret,
        }
    }

    /// Preload all sources in parallel (e.g., COG metadata).
    async fn preload_sources(sources: &HashMap<String, Arc<dyn TileSource>>) {
        let preload_futures: Vec<_> = sources
            .iter()
            .map(|(name, source)| {
                let name = name.clone();
                let source = source.clone();
                async move {
                    if let Err(e) = source.preload().await {
                        tracing::warn!(source = %name, error = %e, "Failed to preload source");
                    } else {
                        tracing::debug!(source = %name, "Source preloaded");
                    }
                }
            })
            .collect();

        join_all(preload_futures).await;
        tracing::info!("Preloaded {} sources", sources.len());
    }

    fn build_sources(
        source_configs: &HashMap<String, SourceConfig>,
    ) -> HashMap<String, Arc<dyn TileSource>> {
        let mut sources = HashMap::new();

        for (name, config) in source_configs {
            let source = Self::build_source(config);
            sources.insert(name.clone(), source);
        }

        sources
    }

    fn build_source(config: &SourceConfig) -> Arc<dyn TileSource> {
        // Separate layers by type
        let mut xyz_layers: Vec<&LayerConfig> = Vec::new();
        let mut cog_layers: Vec<(&LayerConfig, i32)> = Vec::new();
        let mut maplibre_layers: Vec<&LayerConfig> = Vec::new();

        for layer in &config.layers {
            match layer {
                LayerConfig::Xyz { .. } => {
                    xyz_layers.push(layer);
                }
                LayerConfig::Cog { order, .. } => {
                    cog_layers.push((layer, *order));
                }
                LayerConfig::MapLibre { .. } => {
                    maplibre_layers.push(layer);
                }
            }
        }

        // Sort COG layers by order
        cog_layers.sort_by_key(|(_, order)| *order);

        // Build composite source
        let mut composite = CompositeTileSource::new();

        // Add MapLibre as base if present (takes priority over XYZ)
        if let Some(LayerConfig::MapLibre { url }) = maplibre_layers.first() {
            let maplibre_source = MaplibreTileSource::new(url.clone(), None);
            composite = composite.with_base(Box::new(maplibre_source));
            tracing::info!("Added MapLibre layer as base: {}", url);
        } else if let Some(LayerConfig::Xyz { url, range }) = xyz_layers.first() {
            // Add XYZ as base if no MapLibre
            let xyz_source = XyzTileSource::new(url.clone(), range.clone());
            composite = composite.with_base(Box::new(xyz_source));
        }

        // Add COG overlays
        for (layer, _) in cog_layers {
            if let LayerConfig::Cog { url, nodata, .. } = layer {
                let cog_source = CogTileSource::new(url.clone(), nodata.clone());
                composite = composite.with_overlay(Box::new(cog_source));
            }
        }

        Arc::new(composite)
    }

    /// Get a tile source by name.
    pub async fn get_source(&self, name: &str) -> Option<Arc<dyn TileSource>> {
        let sources = self.sources.read().await;
        sources.get(name).cloned()
    }

    /// Reload sources from config.
    pub async fn reload_sources(&self) {
        let config = self.config_manager.get().await;
        let new_sources = Self::build_sources(&config.sources);

        // Preload new sources in parallel
        Self::preload_sources(&new_sources).await;

        let mut sources = self.sources.write().await;
        *sources = new_sources;
        tracing::info!("Rebuilt {} tile sources", sources.len());
    }

    /// List available source names.
    pub async fn list_sources(&self) -> Vec<String> {
        let sources = self.sources.read().await;
        sources.keys().cloned().collect()
    }
}
