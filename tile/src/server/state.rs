//! Application state management.

use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use tokio::sync::RwLock;
use xxhash_rust::xxh64::xxh64;

use crate::{
    cache::{CacheMode, TileCache},
    config::{ConfigManager, LayerConfig, SourceConfig},
    terrain::{
        CogDemSource, DemProvider, GeoBounds, PmtilesEncoding, PmtilesSource, TerrainSettings,
        XyzDemEncoding, XyzDemSource, build_composite_dem,
    },
    tile::{CogTileSource, CompositeTileSource, MaplibreTileSource, TileSource, XyzTileSource},
};

/// Name of the special source whose layers are interpreted as DEM overlays
/// rather than `/tiles/...` raster sources.
const DEM_SOURCE_KEY: &str = "dem";

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
    /// Terrain endpoint state.
    pub terrain: Arc<super::terrain::TerrainState>,
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
        object_cache_control: Option<String>,
        terrain_settings: TerrainSettings,
    ) -> Self {
        let config = config_manager.get().await;

        let cache = Arc::new(TileCache::new(
            cache_size_mb,
            persistent_cache_url,
            cache_mode,
            object_cache_control,
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

        let terrain = Self::build_terrain(&terrain_settings, &config.sources).await;

        Self {
            config_manager,
            cache,
            sources: Arc::new(RwLock::new(sources)),
            reload_secret,
            cache_control,
            terrain,
        }
    }

    /// Construct the terrain state. Base DEM comes from `terrain_settings`
    /// (env vars). If a source named `"dem"` exists in the config, its
    /// `layers` (in original order — first = bottom-most overlay, last =
    /// frontmost) are used as overlays atop the base via
    /// `CompositeDemProvider`.
    async fn build_terrain(
        settings: &TerrainSettings,
        sources: &HashMap<String, SourceConfig>,
    ) -> Arc<super::terrain::TerrainState> {
        let base = settings.build_dem();

        let dem: Arc<dyn DemProvider> = match sources.get(DEM_SOURCE_KEY) {
            None => base,
            Some(dem_cfg) if dem_cfg.layers.is_empty() => base,
            Some(dem_cfg) => {
                let overlays: Vec<Arc<dyn DemProvider>> = dem_cfg
                    .layers
                    .iter()
                    .enumerate()
                    .filter_map(|(i, layer)| build_dem_overlay(i, layer))
                    .collect();
                tracing::info!(
                    "Building composite DEM: 1 base + {} overlays",
                    overlays.len()
                );
                Arc::new(build_composite_dem(base, overlays).await)
            }
        };

        Arc::new(super::terrain::TerrainState {
            dem,
            tile_size: settings.tile_size,
            default_geoid: settings.default_geoid,
            max_zoom: settings.max_zoom,
            max_error: settings.max_error,
        })
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
            // The "dem" source is repurposed for terrain overlays — it must
            // not appear under `/tiles/...`.
            if name == DEM_SOURCE_KEY {
                continue;
            }
            // Skip sources that only have maplibre layers when feature is off
            #[cfg(not(feature = "maplibre"))]
            {
                let has_non_maplibre = config
                    .layers
                    .iter()
                    .any(|l| !matches!(l, LayerConfig::MapLibre { .. }));
                if !has_non_maplibre {
                    tracing::debug!(source = %name, "Skipping maplibre-only source (feature disabled)");
                    continue;
                }
            }

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
        let mut pmtiles_layers: Vec<&LayerConfig> = Vec::new();

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
                LayerConfig::Pmtiles { .. } => {
                    pmtiles_layers.push(layer);
                }
            }
        }

        // Sort COG layers by order
        cog_layers.sort_by_key(|(_, order)| *order);

        // Build composite source
        let mut composite = CompositeTileSource::new();

        // Add MapLibre as base if present (takes priority over XYZ)
        if let Some(LayerConfig::MapLibre { url, version, .. }) = maplibre_layers.first() {
            let maplibre_source =
                MaplibreTileSource::with_version(url.clone(), None, version.as_deref());
            composite = composite.with_base(Box::new(maplibre_source));
        }

        // Add all XYZ layers as overlays (they have range filters so will only render when in range)
        for layer in &xyz_layers {
            if let LayerConfig::Xyz {
                url,
                range,
                version,
                ..
            } = layer
            {
                let xyz_source =
                    XyzTileSource::with_version(url.clone(), range.clone(), version.as_deref());
                composite = composite.with_overlay(Box::new(xyz_source));
            }
        }

        // Add COG overlays
        for (layer, _) in cog_layers {
            if let LayerConfig::Cog {
                url,
                nodata,
                version,
                ..
            } = layer
            {
                let cog_source =
                    CogTileSource::with_version(url.clone(), nodata.clone(), version.as_deref());
                composite = composite.with_overlay(Box::new(cog_source));
            }
        }

        // Add PMTiles overlays
        for layer in &pmtiles_layers {
            if let LayerConfig::Pmtiles {
                url,
                range,
                version,
                ..
            } = layer
            {
                let src = crate::tile::PmtilesTileSource::with_version(
                    url.clone(),
                    range.clone(),
                    version.as_deref(),
                );
                composite = composite.with_overlay(Box::new(src));
            }
        }

        Arc::new(composite)
    }

    /// Get a tile source by name.
    pub async fn get_source(&self, name: &str) -> Option<Arc<dyn TileSource>> {
        let sources = self.sources.read().await;
        sources.get(name).cloned()
    }

    /// Get ETag keys for a tile from a specific source.
    /// Only includes layers that actually cover the specified tile coordinates.
    pub async fn get_source_etag_keys(
        &self,
        name: &str,
        z: u32,
        x: u32,
        y: u32,
    ) -> Option<Vec<String>> {
        let sources = self.sources.read().await;
        let source = sources.get(name)?;
        Some(source.etag_keys(z, x, y))
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

    /// List available source names (sorted alphabetically).
    pub async fn list_sources(&self) -> Vec<String> {
        let sources = self.sources.read().await;
        let mut names: Vec<String> = sources.keys().cloned().collect();
        names.sort();
        names
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

/// Build a single DEM overlay from a `LayerConfig` entry inside the
/// `sources.dem.layers` list. Returns `None` for unsupported variants
/// (e.g. `maplibre`).
fn build_dem_overlay(idx: usize, layer: &LayerConfig) -> Option<Arc<dyn DemProvider>> {
    let slug = format!("dem{idx}");
    let version = layer.version().unwrap_or("v1").to_string();
    match layer {
        LayerConfig::Pmtiles {
            url,
            encoding,
            max_zoom,
            native_tile_size,
            ..
        } => {
            let enc = parse_pmtiles_encoding(encoding.as_deref());
            Some(Arc::new(PmtilesSource::new(
                url.clone(),
                enc,
                version,
                max_zoom.unwrap_or(15),
                native_tile_size.unwrap_or(512),
            )))
        }
        LayerConfig::Xyz {
            url,
            encoding,
            max_zoom,
            native_tile_size,
            range,
            ..
        } => {
            let enc = parse_xyz_dem_encoding(encoding.as_deref());
            // Range-derived bounds aren't geographic, so leave bounds None
            // unless we add an explicit `bounds` field later.
            let _ = range; // unused for DEM; XYZ DEM ignores raster zoom range.
            Some(Arc::new(XyzDemSource::new(
                slug,
                url.clone(),
                enc,
                version,
                max_zoom.unwrap_or(15),
                native_tile_size.unwrap_or(256),
                None::<GeoBounds>,
            )))
        }
        LayerConfig::Cog {
            url,
            nodata,
            max_zoom,
            native_tile_size,
            ..
        } => {
            let nodata_value = nodata.as_ref().and_then(|n| match n {
                crate::config::NoDataConfig::Single(v) => Some(*v),
                _ => None,
            });
            Some(Arc::new(CogDemSource::new(
                slug,
                url.clone(),
                nodata_value,
                version,
                max_zoom.unwrap_or(18),
                native_tile_size.unwrap_or(256),
            )))
        }
        LayerConfig::MapLibre { .. } => {
            tracing::warn!("MapLibre layers are not supported as DEM overlays; skipping");
            None
        }
    }
}

fn parse_pmtiles_encoding(s: Option<&str>) -> PmtilesEncoding {
    match s.unwrap_or("terrarium").to_lowercase().as_str() {
        "mapbox" => PmtilesEncoding::Mapbox,
        _ => PmtilesEncoding::Terrarium,
    }
}

fn parse_xyz_dem_encoding(s: Option<&str>) -> XyzDemEncoding {
    match s.unwrap_or("terrarium").to_lowercase().as_str() {
        "mapbox" => XyzDemEncoding::Mapbox,
        _ => XyzDemEncoding::Terrarium,
    }
}
