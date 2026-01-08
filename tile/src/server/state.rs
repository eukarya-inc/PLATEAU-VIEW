//! Application state management.

use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use tokio::sync::RwLock;
use xxhash_rust::xxh64::xxh64;

use crate::{
    cache::{CacheMode, TileCache},
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
    /// Cache-Control header value (optional)
    pub cache_control: Option<String>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        config_manager: Arc<ConfigManager>,
        cache_size_mb: u64,
        reload_secret: Option<String>,
        preload_mode: &str,
        persistent_cache_url: Option<&str>,
        cache_mode: CacheMode,
        cache_control: Option<String>,
    ) -> Self {
        let config = config_manager.get().await;

        let cache = Arc::new(TileCache::new(
            cache_size_mb,
            persistent_cache_url,
            cache_mode,
        ));

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
            cache_control,
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
        if let Some(LayerConfig::MapLibre { url, .. }) = maplibre_layers.first() {
            let maplibre_source = MaplibreTileSource::new(url.clone(), None);
            composite = composite.with_base(Box::new(maplibre_source));
        } else if let Some(LayerConfig::Xyz { url, range, .. }) = xyz_layers.first() {
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

    /// Get the version string for a source.
    /// Priority: per-source version > global version > computed from layers.
    pub async fn get_source_version(&self, source_name: &str) -> Option<String> {
        let config = self.config_manager.get().await;

        let source_config = config.sources.get(source_name)?;

        // Check per-source version first
        if let Some(version) = &source_config.version {
            return Some(version.clone());
        }

        // Fall back to global version
        if let Some(version) = &config.version {
            return Some(version.clone());
        }

        // Compute version from layers (type/url/version sorted by order)
        Some(Self::compute_layers_version(&source_config.layers))
    }

    /// Compute a version string from layers' type/url/version sorted by order.
    fn compute_layers_version(layers: &[LayerConfig]) -> String {
        // Sort layers by order
        let mut sorted_layers: Vec<_> = layers.iter().collect();
        sorted_layers.sort_by_key(|l| l.order());

        // Build string: "type:url:version|type:url:version|..."
        let input: String = sorted_layers
            .iter()
            .map(|layer| {
                let version = layer.version().unwrap_or("");
                format!("{}:{}:{}", layer.layer_type(), layer.url(), version)
            })
            .collect::<Vec<_>>()
            .join("|");

        // Hash and return as hex string prefixed with "layers-"
        let hash = xxh64(input.as_bytes(), 0);
        format!("layers-{hash:x}")
    }
}
