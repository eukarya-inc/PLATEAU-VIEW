//! Application state management.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    cache::TileCache,
    config::{ConfigManager, LayerConfig, SourceConfig},
    tile::{CogTileSource, CompositeTileSource, TileSource, XyzTileSource},
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
    ) -> Self {
        let config = config_manager.get().await;

        let cache = Arc::new(TileCache::new(cache_size_mb));

        let sources = Self::build_sources(&config.sources);

        Self {
            config_manager,
            cache,
            sources: Arc::new(RwLock::new(sources)),
            reload_secret,
        }
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
        // Separate XYZ (base) and COG (overlay) layers
        let mut xyz_layers: Vec<&LayerConfig> = Vec::new();
        let mut cog_layers: Vec<(&LayerConfig, i32)> = Vec::new();

        for layer in &config.layers {
            match layer {
                LayerConfig::Xyz { .. } => {
                    xyz_layers.push(layer);
                }
                LayerConfig::Cog { order, .. } => {
                    cog_layers.push((layer, *order));
                }
                LayerConfig::MapLibre { .. } => {
                    // MapLibre style layers are not yet implemented, skip
                }
            }
        }

        // Sort COG layers by order
        cog_layers.sort_by_key(|(_, order)| *order);

        // Build composite source
        let mut composite = CompositeTileSource::new();

        // Add XYZ as base (use first one if multiple)
        if let Some(LayerConfig::Xyz { url, range }) = xyz_layers.first() {
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
