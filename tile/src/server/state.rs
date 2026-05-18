//! Application state management.

use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use tokio::sync::{Mutex, RwLock};
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

/// Default DEM source name used when terrain endpoints are hit without an
/// explicit `{name}` path segment (i.e. `/terrain/{z}/{x}/{y}` rather than
/// `/terrain/{name}/{z}/{x}/{y}`). Also doubles as the back-compat reserved
/// name: a source named `"dem"` is treated as a DEM source even if its
/// config entry doesn't set `type: "dem"`.
pub const DEFAULT_DEM_SOURCE_KEY: &str = "dem";

/// Application state shared across handlers.
pub struct AppState {
    pub config_manager: Arc<ConfigManager>,
    pub cache: Arc<TileCache>,
    /// Cached tile sources (rebuilt on config reload)
    sources: Arc<RwLock<HashMap<String, Arc<dyn TileSource>>>>,
    /// Per-layer inventory for raster sources, rebuilt on config reload.
    inventory: Arc<RwLock<Vec<LayerEntry>>>,
    /// Per-layer inventory for DEM overlays, rebuilt alongside `terrain` on
    /// config reload (so adding/removing COG overlays in CMS reflects in
    /// `/tiles/sources.json` after the next reload).
    dem_inventory: Arc<RwLock<Vec<LayerEntry>>>,
    /// Secret for reload endpoint authorization
    pub reload_secret: Option<String>,
    /// Cache-Control header value (optional)
    pub cache_control: Option<String>,
    /// Terrain endpoint state, keyed by DEM source name. Wrapped in a lock so
    /// config reload can swap in a freshly built set of `TerrainState`s — the
    /// DEM source list (or any overlay within them) may change in CMS. Always
    /// contains at least an entry under [`DEFAULT_DEM_SOURCE_KEY`]; that entry
    /// is what `/terrain/{z}/{x}/{y}` (no name) resolves to.
    terrains: Arc<RwLock<HashMap<String, Arc<super::terrain::TerrainState>>>>,
    /// Cached env-derived terrain settings, needed at reload time to rebuild
    /// the base DEM with the same configuration.
    terrain_settings: TerrainSettings,
    /// Serializes config reload work. Both manual `/reload` and lazy
    /// revalidation take this — `/reload` blocks (`.lock()`) so the operator
    /// sees a deterministic outcome, while lazy revalidation uses `try_lock`
    /// and skips when an operation is already in flight.
    reload_mutex: Arc<Mutex<()>>,
}

/// One layer described in the config, with a live source instance for
/// metadata queries (`bounds()`, `zoom_range()`).
#[derive(Clone)]
pub struct LayerEntry {
    /// Owning source name in the config. For DEM overlays this is the
    /// originating DEM source name (one of multiple may exist).
    pub source_name: String,
    /// Position within the owning source's `layers` array.
    pub layer_idx: usize,
    /// `xyz` | `cog` | `pmtiles` | `maplibre`.
    pub layer_type: &'static str,
    pub url: String,
    pub version: Option<String>,
    /// Either a raster TileSource or a DEM provider — we only need bounds
    /// from one of them, and we record which kind produced the entry.
    pub kind: LayerEntryKind,
}

#[derive(Clone)]
pub enum LayerEntryKind {
    Raster(Arc<dyn TileSource>),
    Dem(Arc<dyn crate::terrain::DemProvider>),
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

        let mut inventory = Vec::new();
        let sources = Self::build_sources(&config.sources, &mut inventory);

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

        let (terrains, dem_inventory) =
            Self::build_terrains(&terrain_settings, &config.sources).await;

        Self {
            config_manager,
            cache,
            sources: Arc::new(RwLock::new(sources)),
            inventory: Arc::new(RwLock::new(inventory)),
            dem_inventory: Arc::new(RwLock::new(dem_inventory)),
            reload_secret,
            cache_control,
            terrains: Arc::new(RwLock::new(terrains)),
            terrain_settings,
            reload_mutex: Arc::new(Mutex::new(())),
        }
    }

    /// Lazy revalidation hook to call near the start of tile/terrain handlers.
    ///
    /// On the hot path (TTL not yet expired) this is a single relaxed atomic
    /// load. When the TTL window has elapsed, exactly one caller per pod wins
    /// the CAS, then takes the reload mutex (`try_lock` — skip if `/reload`
    /// or another revalidation is already in flight) and synchronously
    /// refetches the config. If the body hash changed, sources are rebuilt
    /// before this method returns.
    ///
    /// Synchronous on purpose: Cloud Run throttles CPU outside the active
    /// request, so a `tokio::spawn`-ed background task could be paused or
    /// killed mid-rebuild. The triggering request eats the latency (~hundreds
    /// of ms once per TTL window per pod); other concurrent requests skip
    /// via CAS and continue with the current state.
    pub async fn maybe_revalidate(&self) {
        if !self.config_manager.claim_check_slot() {
            return;
        }
        let _guard = match self.reload_mutex.try_lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        match self.config_manager.reload().await {
            Ok(true) => {
                tracing::info!("Config changed on revalidation; rebuilding sources");
                self.reload_sources().await;
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(error = %e, "Config revalidation fetch failed");
            }
        }
    }

    /// Accessor for the reload mutex so the `/reload` handler can take it
    /// before invoking `config_manager.reload()` + `reload_sources()`. Using
    /// the same mutex as [`Self::maybe_revalidate`] guarantees the two
    /// reload paths never overlap.
    pub fn reload_mutex(&self) -> Arc<Mutex<()>> {
        self.reload_mutex.clone()
    }

    /// Look up a terrain state by source name. `None` (or an empty string)
    /// resolves to [`DEFAULT_DEM_SOURCE_KEY`]. Returns `None` if no terrain
    /// is registered under the requested name.
    pub async fn get_terrain(
        &self,
        name: Option<&str>,
    ) -> Option<Arc<super::terrain::TerrainState>> {
        let key = match name {
            None | Some("") => DEFAULT_DEM_SOURCE_KEY,
            Some(n) => n,
        };
        self.terrains.read().await.get(key).cloned()
    }

    /// Construct the set of terrain states, one per DEM source in the config.
    ///
    /// The base DEM (env-configured) is shared across every entry; only the
    /// overlay stack differs between them. If no config source is identified
    /// as a DEM source (via `type: "dem"` or the reserved name `"dem"`), a
    /// single bare-base entry is still produced under
    /// [`DEFAULT_DEM_SOURCE_KEY`] so `/terrain/...` keeps working out of the
    /// box without any config.
    async fn build_terrains(
        settings: &TerrainSettings,
        sources: &HashMap<String, SourceConfig>,
    ) -> (
        HashMap<String, Arc<super::terrain::TerrainState>>,
        Vec<LayerEntry>,
    ) {
        // Wrap the base DEM in an LRU so the parent-tile fallback in
        // `CompositeDemProvider::fetch_base_upsampled` doesn't re-fetch +
        // re-decode the same Mapterhorn parent for every child request.
        // 200 entries × ~2 MiB ≈ 400 MiB upper bound — well within the
        // Cloud Run memory budget and dramatically cuts per-request
        // memory pressure under concurrent quantized-mesh load.
        let base: Arc<dyn DemProvider> = Arc::new(crate::terrain::CachedDemProvider::new(
            settings.build_dem(),
            200,
        ));
        let mut dem_inventory: Vec<LayerEntry> = Vec::new();
        let mut terrains: HashMap<String, Arc<super::terrain::TerrainState>> = HashMap::new();

        // Collect every config source recognised as a DEM source.
        let dem_sources: Vec<(&String, &SourceConfig)> = sources
            .iter()
            .filter(|(name, cfg)| cfg.is_dem(name))
            .collect();

        for (name, dem_cfg) in &dem_sources {
            let dem: Arc<dyn DemProvider> = if dem_cfg.layers.is_empty() {
                base.clone()
            } else {
                let overlays: Vec<Arc<dyn DemProvider>> = dem_cfg
                    .layers
                    .iter()
                    .enumerate()
                    .filter_map(|(i, layer)| {
                        let provider = build_dem_overlay(name, i, layer)?;
                        dem_inventory.push(LayerEntry {
                            source_name: (*name).to_string(),
                            layer_idx: i,
                            layer_type: layer.layer_type_static(),
                            url: layer.url().to_string(),
                            version: layer.version().map(|s| s.to_string()),
                            kind: LayerEntryKind::Dem(provider.clone()),
                        });
                        Some(provider)
                    })
                    .collect();
                tracing::info!(
                    source = %name,
                    "Building composite DEM: 1 base + {} overlays",
                    overlays.len()
                );
                Arc::new(build_composite_dem(base.clone(), overlays).await)
            };
            terrains.insert(
                (*name).to_string(),
                Arc::new(super::terrain::TerrainState {
                    dem,
                    tile_size: settings.tile_size,
                    default_geoid: settings.default_geoid,
                    max_zoom: settings.max_zoom,
                    max_error: settings.max_error,
                }),
            );
        }

        // Guarantee a default entry: if no DEM source is configured at all, or
        // if every configured DEM source has a non-default name, fall back to
        // a bare-base provider under the default key so legacy `/terrain/...`
        // (no name) keeps responding.
        terrains
            .entry(DEFAULT_DEM_SOURCE_KEY.to_string())
            .or_insert_with(|| {
                Arc::new(super::terrain::TerrainState {
                    dem: base.clone(),
                    tile_size: settings.tile_size,
                    default_geoid: settings.default_geoid,
                    max_zoom: settings.max_zoom,
                    max_error: settings.max_error,
                })
            });

        (terrains, dem_inventory)
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
        inventory: &mut Vec<LayerEntry>,
    ) -> HashMap<String, Arc<dyn TileSource>> {
        let mut sources = HashMap::new();

        for (name, config) in source_configs {
            // DEM sources are repurposed for terrain overlays — they must
            // not appear under `/tiles/...`.
            if config.is_dem(name) {
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

            let source = Self::build_source(name, config, inventory);
            sources.insert(name.clone(), source);
        }

        sources
    }

    fn build_source(
        source_name: &str,
        config: &SourceConfig,
        inventory: &mut Vec<LayerEntry>,
    ) -> Arc<dyn TileSource> {
        // Separate layers by type, preserving original index for inventory.
        let mut xyz_layers: Vec<(usize, &LayerConfig)> = Vec::new();
        let mut cog_layers: Vec<(usize, &LayerConfig, i32)> = Vec::new();
        let mut maplibre_layers: Vec<(usize, &LayerConfig)> = Vec::new();
        let mut pmtiles_layers: Vec<(usize, &LayerConfig)> = Vec::new();

        for (i, layer) in config.layers.iter().enumerate() {
            match layer {
                LayerConfig::Xyz { .. } => xyz_layers.push((i, layer)),
                LayerConfig::Cog { order, .. } => cog_layers.push((i, layer, *order)),
                LayerConfig::MapLibre { .. } => maplibre_layers.push((i, layer)),
                LayerConfig::Pmtiles { .. } => pmtiles_layers.push((i, layer)),
            }
        }

        // Sort COG layers by order
        cog_layers.sort_by_key(|(_, _, order)| *order);

        // Helper to record a layer entry for the inspector inventory.
        let mut push_entry = |idx: usize, layer: &LayerConfig, src: Arc<dyn TileSource>| {
            inventory.push(LayerEntry {
                source_name: source_name.to_string(),
                layer_idx: idx,
                layer_type: layer.layer_type_static(),
                url: layer.url().to_string(),
                version: layer.version().map(|s| s.to_string()),
                kind: LayerEntryKind::Raster(src),
            });
        };

        // Build composite source
        let mut composite = CompositeTileSource::new();

        // Add MapLibre as base if present (takes priority over XYZ)
        if let Some((idx, LayerConfig::MapLibre { url, version, .. })) = maplibre_layers.first() {
            let maplibre_source: Arc<dyn TileSource> = Arc::new(MaplibreTileSource::with_version(
                url.clone(),
                None,
                version.as_deref(),
            ));
            push_entry(*idx, maplibre_layers[0].1, maplibre_source.clone());
            composite = composite.with_base(maplibre_source);
        }

        // Add all XYZ layers as overlays.
        for (idx, layer) in &xyz_layers {
            if let LayerConfig::Xyz {
                url,
                range,
                version,
                ..
            } = layer
            {
                let src: Arc<dyn TileSource> = Arc::new(XyzTileSource::with_version(
                    url.clone(),
                    range.clone(),
                    version.as_deref(),
                ));
                push_entry(*idx, layer, src.clone());
                composite = composite.with_overlay(src);
            }
        }

        // Add COG overlays.
        for (idx, layer, _) in &cog_layers {
            if let LayerConfig::Cog {
                url,
                nodata,
                version,
                ..
            } = layer
            {
                let src: Arc<dyn TileSource> = Arc::new(CogTileSource::with_version(
                    url.clone(),
                    nodata.clone(),
                    version.as_deref(),
                ));
                push_entry(*idx, layer, src.clone());
                composite = composite.with_overlay(src);
            }
        }

        // Add PMTiles overlays.
        for (idx, layer) in &pmtiles_layers {
            if let LayerConfig::Pmtiles {
                url,
                range,
                version,
                ..
            } = layer
            {
                let src: Arc<dyn TileSource> =
                    Arc::new(crate::tile::PmtilesTileSource::with_version(
                        url.clone(),
                        range.clone(),
                        version.as_deref(),
                    ));
                push_entry(*idx, layer, src.clone());
                composite = composite.with_overlay(src);
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

    /// Reload sources from config. Rebuilds raster sources, DEM terrain, and
    /// both inventories so adding/removing/replacing entries in CMS — for
    /// either `/tiles/...` rasters or `sources.dem` overlays — reflects in
    /// the live server after the next `/reload`.
    pub async fn reload_sources(&self) {
        let config = self.config_manager.get().await;

        let mut new_inventory = Vec::new();
        let new_sources = Self::build_sources(&config.sources, &mut new_inventory);
        Self::preload_sources(&new_sources).await;

        let (new_terrains, new_dem_inventory) =
            Self::build_terrains(&self.terrain_settings, &config.sources).await;

        let mut sources = self.sources.write().await;
        *sources = new_sources;
        let mut inv = self.inventory.write().await;
        *inv = new_inventory;
        let mut dem_inv = self.dem_inventory.write().await;
        *dem_inv = new_dem_inventory;
        let mut terrains = self.terrains.write().await;
        *terrains = new_terrains;
        tracing::info!(
            "Rebuilt {} tile sources and DEM terrain on reload",
            sources.len()
        );
    }

    /// Snapshot the per-layer inventory (for `/tiles/bounds.json`). Combines
    /// the raster inventory with the DEM-overlay inventory. Both are rebuilt
    /// on `/reload`.
    pub async fn inventory_snapshot(&self) -> Vec<LayerEntry> {
        let raster = self.inventory.read().await.clone();
        let dem = self.dem_inventory.read().await.clone();
        let mut all = Vec::with_capacity(raster.len() + dem.len());
        all.extend(raster);
        all.extend(dem);
        all
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
/// Default version string baked into DEM overlay providers when the config
/// doesn't supply one. Bump (e.g. on every behavior-changing deploy) to
/// invalidate downstream tile caches whose etag composition includes this
/// value. Format is a compact timestamp so multiple bumps per day are easy
/// to read at a glance.
const DEM_OVERLAY_DEFAULT_VERSION: &str = "20260508-2230";

fn build_dem_overlay(
    source_name: &str,
    idx: usize,
    layer: &LayerConfig,
) -> Option<Arc<dyn DemProvider>> {
    // Slug feeds into per-overlay cache keys; include the source name so two
    // DEM sources that happen to reuse the same upstream URL at the same
    // layer index don't collide in any downstream cache that keys on slug.
    let slug = format!("{source_name}-dem{idx}");
    let version = layer
        .version()
        .unwrap_or(DEM_OVERLAY_DEFAULT_VERSION)
        .to_string();
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
