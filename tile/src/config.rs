//! Configuration management with remote URL loading and auto-reload.

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
use thiserror::Error;
use tokio::sync::RwLock;
use xxhash_rust::xxh64::xxh64;

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to fetch config: {0}")]
    FetchError(String),
    #[error("Failed to parse config: {0}")]
    ParseError(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// Root configuration structure.
///
/// The config JSON only describes overlay sources served under `/tiles/...`.
/// The terrain base DEM and related settings are configured via environment
/// variables — see [`crate::terrain::TerrainSettings`].
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Global version string for ETag calculation (optional)
    #[serde(default)]
    pub version: Option<String>,
    pub sources: HashMap<String, SourceConfig>,
    #[serde(default)]
    pub cache: Option<CacheConfig>,
}

/// Configuration for a named tile source
#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    /// Per-source version string for ETag calculation (overrides global version)
    #[serde(default)]
    pub version: Option<String>,
    /// Human-readable description of the source. Surfaced via the public
    /// `/tiles/catalog.json` endpoint for end-user UIs.
    #[serde(default)]
    pub description: Option<String>,
    pub layers: Vec<LayerConfig>,
}

/// Layer configuration (XYZ, COG, or other types). Most fields are shared
/// across raster (`sources.<name>.layers`) and DEM (`sources.dem.layers`)
/// uses; the DEM-specific knobs (`encoding`, `max_zoom`, `native_tile_size`,
/// `nodata`) are simply ignored in the raster pipeline.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LayerConfig {
    #[serde(rename = "xyz")]
    Xyz {
        /// URL template with {z}, {x}, {y} placeholders
        url: String,
        /// Optional zoom range restriction
        #[serde(default)]
        range: Option<RangeConfig>,
        /// Layer version for ETag calculation
        #[serde(default)]
        version: Option<String>,
        /// DEM-only: tile encoding (`terrarium` | `mapbox`).
        #[serde(default)]
        encoding: Option<String>,
        /// DEM-only: upstream max zoom served.
        #[serde(default)]
        max_zoom: Option<u8>,
        /// DEM-only: native tile size in pixels.
        #[serde(default)]
        native_tile_size: Option<u32>,
    },
    #[serde(rename = "cog")]
    Cog {
        /// URL to the COG file (HTTP, GCS, S3)
        url: String,
        /// NoData values to treat as transparent (raster) / NaN (DEM).
        #[serde(default)]
        nodata: Option<NoDataConfig>,
        /// Layer order (higher = on top)
        #[serde(default)]
        order: i32,
        /// Layer version for ETag calculation
        #[serde(default)]
        version: Option<String>,
        /// DEM-only: upstream max zoom served.
        #[serde(default)]
        max_zoom: Option<u8>,
        /// DEM-only: native tile size in pixels.
        #[serde(default)]
        native_tile_size: Option<u32>,
    },
    /// MapLibre style (not yet implemented, ignored)
    #[serde(rename = "maplibre")]
    MapLibre {
        url: String,
        /// Layer version for ETag calculation
        #[serde(default)]
        version: Option<String>,
    },
    #[serde(rename = "pmtiles")]
    Pmtiles {
        /// URL to the .pmtiles archive (https/gs/s3/r2/file).
        url: String,
        /// Optional zoom range restriction (raster use).
        #[serde(default)]
        range: Option<RangeConfig>,
        /// Layer version for ETag calculation
        #[serde(default)]
        version: Option<String>,
        /// DEM-only: tile encoding (`terrarium` | `mapbox`).
        #[serde(default)]
        encoding: Option<String>,
        /// DEM-only: upstream max zoom served.
        #[serde(default)]
        max_zoom: Option<u8>,
        /// DEM-only: native tile size in pixels.
        #[serde(default)]
        native_tile_size: Option<u32>,
    },
}

impl LayerConfig {
    /// Get the layer type as a string.
    pub fn layer_type(&self) -> &'static str {
        self.layer_type_static()
    }

    /// Same as `layer_type`, lifetimed for use in `'static` contexts.
    pub fn layer_type_static(&self) -> &'static str {
        match self {
            LayerConfig::Xyz { .. } => "xyz",
            LayerConfig::Cog { .. } => "cog",
            LayerConfig::MapLibre { .. } => "maplibre",
            LayerConfig::Pmtiles { .. } => "pmtiles",
        }
    }

    /// Get the layer URL.
    pub fn url(&self) -> &str {
        match self {
            LayerConfig::Xyz { url, .. } => url,
            LayerConfig::Cog { url, .. } => url,
            LayerConfig::MapLibre { url, .. } => url,
            LayerConfig::Pmtiles { url, .. } => url,
        }
    }

    /// Get the layer version.
    pub fn version(&self) -> Option<&str> {
        match self {
            LayerConfig::Xyz { version, .. } => version.as_deref(),
            LayerConfig::Cog { version, .. } => version.as_deref(),
            LayerConfig::MapLibre { version, .. } => version.as_deref(),
            LayerConfig::Pmtiles { version, .. } => version.as_deref(),
        }
    }

    /// Get the layer order (for sorting).
    pub fn order(&self) -> i32 {
        match self {
            LayerConfig::Xyz { .. } => 0,
            LayerConfig::Cog { order, .. } => *order,
            LayerConfig::MapLibre { .. } => 0,
            LayerConfig::Pmtiles { .. } => 0,
        }
    }
}

/// Zoom/coordinate range configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RangeConfig {
    #[serde(default)]
    pub z_min: Option<u32>,
    #[serde(default)]
    pub z_max: Option<u32>,
    #[serde(default)]
    pub x_min: Option<u32>,
    #[serde(default)]
    pub x_max: Option<u32>,
    #[serde(default)]
    pub y_min: Option<u32>,
    #[serde(default)]
    pub y_max: Option<u32>,
}

impl RangeConfig {
    pub fn contains(&self, z: u32, x: u32, y: u32) -> bool {
        if let Some(z_min) = self.z_min
            && z < z_min
        {
            return false;
        }
        if let Some(z_max) = self.z_max
            && z > z_max
        {
            return false;
        }
        if let Some(x_min) = self.x_min
            && x < x_min
        {
            return false;
        }
        if let Some(x_max) = self.x_max
            && x > x_max
        {
            return false;
        }
        if let Some(y_min) = self.y_min
            && y < y_min
        {
            return false;
        }
        if let Some(y_max) = self.y_max
            && y > y_max
        {
            return false;
        }
        true
    }
}

/// NoData configuration supporting various formats:
/// - Single value: `255` or `[255]`
/// - Multi-band: `[0, 0, 0]` for RGB black
/// - Multiple patterns: `[[0, 0, 0], [255, 255, 255]]` for black and white
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum NoDataConfig {
    /// Single value for all bands
    Single(f64),
    /// Single pattern (one value per band)
    Pattern(Vec<f64>),
    /// Multiple patterns
    Patterns(Vec<Vec<f64>>),
}

impl NoDataConfig {
    /// Check if the given pixel values match any nodata pattern
    pub fn is_nodata(&self, values: &[f64]) -> bool {
        match self {
            NoDataConfig::Single(v) => values.iter().all(|val| (val - v).abs() < 1e-6),
            NoDataConfig::Pattern(pattern) => {
                if values.len() != pattern.len() {
                    return false;
                }
                values
                    .iter()
                    .zip(pattern.iter())
                    .all(|(val, pat)| (val - pat).abs() < 1e-6)
            }
            NoDataConfig::Patterns(patterns) => patterns.iter().any(|pattern| {
                if values.len() != pattern.len() {
                    return false;
                }
                values
                    .iter()
                    .zip(pattern.iter())
                    .all(|(val, pat)| (val - pat).abs() < 1e-6)
            }),
        }
    }

    /// Check if the given u8 pixel values match any nodata pattern
    pub fn is_nodata_u8(&self, values: &[u8]) -> bool {
        let float_values: Vec<f64> = values.iter().map(|&v| v as f64).collect();
        self.is_nodata(&float_values)
    }
}

/// Cache configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    /// GCS bucket for persistent cache (optional)
    #[serde(default)]
    pub gcs_bucket: Option<String>,
}

/// Configuration manager with auto-reload support.
///
/// Supports lazy revalidation: callers invoke [`Self::claim_check_slot`] before
/// the per-request critical section. The first caller after the TTL window has
/// expired wins via CAS, performs the fetch via [`Self::revalidate`], and the
/// content-hash check decides whether downstream sources need rebuilding.
/// Losing callers see the bumped timestamp and skip — they continue serving
/// from the currently loaded state (eventually consistent).
pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    config_url: String,
    client: reqwest::Client,
    /// Unix-seconds timestamp of the last revalidation attempt. Bumped via CAS
    /// before fetching so concurrent requests only let one through per TTL
    /// window. Stays bumped even on fetch failure — preventing retry storms
    /// against a flaky upstream.
    last_checked: AtomicU64,
    /// xxh64 of the raw config-body bytes from the most recent successful
    /// fetch. Used to skip rebuilding sources when the JSON hasn't changed.
    /// `0` means "no successful fetch yet".
    content_hash: AtomicU64,
    /// Revalidation cadence. `0` disables lazy revalidation (manual `/reload`
    /// only).
    revalidate_ttl: Duration,
}

impl ConfigManager {
    pub async fn new(config_url: &str, revalidate_ttl: Duration) -> Result<Self, ConfigError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ConfigError::FetchError(e.to_string()))?;

        let (config, body_hash) = Self::fetch_config(&client, config_url).await?;

        tracing::info!("Loaded configuration with {} sources", config.sources.len());

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_url: config_url.to_string(),
            client,
            last_checked: AtomicU64::new(unix_secs()),
            content_hash: AtomicU64::new(body_hash),
            revalidate_ttl,
        })
    }

    /// Create a `ConfigManager` with no external config. Used when `CONFIG_URL`
    /// is not set — the terrain endpoint still works with built-in defaults,
    /// and `/tiles/...` sources are simply empty.
    pub fn empty() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            config: Arc::new(RwLock::new(Config {
                version: None,
                sources: HashMap::new(),
                cache: None,
            })),
            config_url: String::new(),
            client,
            last_checked: AtomicU64::new(0),
            content_hash: AtomicU64::new(0),
            revalidate_ttl: Duration::from_secs(0),
        }
    }

    /// Reload is a no-op if no config URL is configured.
    pub fn has_url(&self) -> bool {
        !self.config_url.is_empty()
    }

    /// Whether lazy revalidation is enabled (TTL > 0 and a URL is configured).
    pub fn revalidation_enabled(&self) -> bool {
        self.has_url() && !self.revalidate_ttl.is_zero()
    }

    /// Try to claim the revalidation slot for the current TTL window.
    ///
    /// Returns `true` for the single caller per pod that wins the CAS race
    /// after the window has expired. Callers that return `false` should skip
    /// revalidation work and continue serving from the current state.
    pub fn claim_check_slot(&self) -> bool {
        if !self.revalidation_enabled() {
            return false;
        }
        let now = unix_secs();
        let ttl = self.revalidate_ttl.as_secs();
        let last = self.last_checked.load(Ordering::Acquire);
        if now.saturating_sub(last) < ttl {
            return false;
        }
        self.last_checked
            .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    async fn fetch_config(
        client: &reqwest::Client,
        url: &str,
    ) -> Result<(Config, u64), ConfigError> {
        let text = if let Some(path) = url.strip_prefix("file://") {
            // Read from local file
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| ConfigError::FetchError(format!("Failed to read file: {e}")))?
        } else {
            // Fetch from HTTP/HTTPS
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|e| ConfigError::FetchError(e.to_string()))?;

            if !response.status().is_success() {
                return Err(ConfigError::FetchError(format!(
                    "HTTP {}",
                    response.status()
                )));
            }

            response
                .text()
                .await
                .map_err(|e| ConfigError::FetchError(e.to_string()))?
        };

        let hash = xxh64(text.as_bytes(), 0);
        let cfg =
            serde_json::from_str(&text).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        Ok((cfg, hash))
    }

    /// Reload configuration from URL unconditionally and swap it in.
    /// Returns `Ok(true)` if the content hash changed (callers should rebuild
    /// downstream sources), `Ok(false)` if the body is byte-identical to the
    /// previous successful fetch. No-op when running without a CONFIG_URL.
    pub async fn reload(&self) -> Result<bool, ConfigError> {
        if !self.has_url() {
            tracing::info!("Reload requested but no CONFIG_URL is set; nothing to do");
            return Ok(false);
        }
        let (new_config, new_hash) = Self::fetch_config(&self.client, &self.config_url).await?;
        let prev_hash = self.content_hash.swap(new_hash, Ordering::AcqRel);
        // Bump the revalidation timestamp so any concurrent lazy check that
        // arrives within the next TTL window short-circuits.
        self.last_checked.store(unix_secs(), Ordering::Release);
        let changed = prev_hash != new_hash;
        if changed {
            let mut config = self.config.write().await;
            *config = new_config;
            tracing::info!("Configuration reloaded from {}", self.config_url);
        } else {
            tracing::debug!(
                "Config refetched from {} but content is unchanged",
                self.config_url
            );
        }
        Ok(changed)
    }

    /// Get current configuration
    pub async fn get(&self) -> Config {
        self.config.read().await.clone()
    }

    /// Get a specific source configuration
    pub async fn get_source(&self, name: &str) -> Option<SourceConfig> {
        self.config.read().await.sources.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nodata_single() {
        let nodata = NoDataConfig::Single(255.0);
        assert!(nodata.is_nodata(&[255.0]));
        assert!(nodata.is_nodata(&[255.0, 255.0, 255.0]));
        assert!(!nodata.is_nodata(&[0.0]));
        assert!(!nodata.is_nodata(&[255.0, 0.0, 255.0]));
    }

    #[test]
    fn test_nodata_pattern() {
        let nodata = NoDataConfig::Pattern(vec![0.0, 0.0, 0.0]);
        assert!(nodata.is_nodata(&[0.0, 0.0, 0.0]));
        assert!(!nodata.is_nodata(&[255.0, 255.0, 255.0]));
        assert!(!nodata.is_nodata(&[0.0, 0.0])); // Wrong number of bands
    }

    #[test]
    fn test_nodata_patterns() {
        let nodata = NoDataConfig::Patterns(vec![vec![0.0, 0.0, 0.0], vec![255.0, 255.0, 255.0]]);
        assert!(nodata.is_nodata(&[0.0, 0.0, 0.0]));
        assert!(nodata.is_nodata(&[255.0, 255.0, 255.0]));
        assert!(!nodata.is_nodata(&[128.0, 128.0, 128.0]));
    }

    #[test]
    fn test_nodata_u8() {
        let nodata = NoDataConfig::Patterns(vec![vec![0.0, 0.0, 0.0], vec![255.0, 255.0, 255.0]]);
        assert!(nodata.is_nodata_u8(&[0, 0, 0]));
        assert!(nodata.is_nodata_u8(&[255, 255, 255]));
        assert!(!nodata.is_nodata_u8(&[128, 128, 128]));
    }

    #[test]
    fn test_range_contains() {
        let range = RangeConfig {
            z_min: Some(5),
            z_max: Some(15),
            x_min: None,
            x_max: None,
            y_min: None,
            y_max: None,
        };
        assert!(range.contains(10, 100, 100));
        assert!(!range.contains(4, 100, 100));
        assert!(!range.contains(16, 100, 100));
    }

    #[test]
    fn test_config_parse() {
        let json = r#"{
            "sources": {
                "ortho": {
                    "layers": [
                        {
                            "type": "xyz",
                            "url": "https://example.com/{z}/{x}/{y}.png"
                        },
                        {
                            "type": "cog",
                            "url": "https://example.com/cog.tif",
                            "nodata": [[0, 0, 0], [255, 255, 255]],
                            "order": 1
                        }
                    ]
                }
            }
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.sources.contains_key("ortho"));
        assert_eq!(config.sources["ortho"].layers.len(), 2);
    }
}
