//! Configuration management with remote URL loading and auto-reload.

use std::{collections::HashMap, sync::Arc, time::Duration};

use serde::Deserialize;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to fetch config: {0}")]
    FetchError(String),
    #[error("Failed to parse config: {0}")]
    ParseError(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// Root configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub sources: HashMap<String, SourceConfig>,
    #[serde(default)]
    pub cache: Option<CacheConfig>,
}

/// Configuration for a named tile source
#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    pub layers: Vec<LayerConfig>,
}

/// Layer configuration (XYZ, COG, or other types)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum LayerConfig {
    #[serde(rename = "xyz")]
    Xyz {
        /// URL template with {z}, {x}, {y} placeholders
        url: String,
        /// Optional zoom range restriction
        #[serde(default)]
        range: Option<RangeConfig>,
    },
    #[serde(rename = "cog")]
    Cog {
        /// URL to the COG file (HTTP, GCS, S3)
        url: String,
        /// NoData values to treat as transparent
        #[serde(default)]
        nodata: Option<NoDataConfig>,
        /// Layer order (higher = on top)
        #[serde(default)]
        order: i32,
    },
    /// MapLibre style (not yet implemented, ignored)
    #[serde(rename = "maplibre")]
    MapLibre { url: String },
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
        if let Some(z_min) = self.z_min {
            if z < z_min {
                return false;
            }
        }
        if let Some(z_max) = self.z_max {
            if z > z_max {
                return false;
            }
        }
        if let Some(x_min) = self.x_min {
            if x < x_min {
                return false;
            }
        }
        if let Some(x_max) = self.x_max {
            if x > x_max {
                return false;
            }
        }
        if let Some(y_min) = self.y_min {
            if y < y_min {
                return false;
            }
        }
        if let Some(y_max) = self.y_max {
            if y > y_max {
                return false;
            }
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

/// Configuration manager with auto-reload support
pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    config_url: String,
    client: reqwest::Client,
}

impl ConfigManager {
    pub async fn new(config_url: &str) -> Result<Self, ConfigError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| ConfigError::FetchError(e.to_string()))?;

        let config = Self::fetch_config(&client, config_url).await?;

        tracing::info!("Loaded configuration with {} sources", config.sources.len());

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_url: config_url.to_string(),
            client,
        })
    }

    async fn fetch_config(client: &reqwest::Client, url: &str) -> Result<Config, ConfigError> {
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

        serde_json::from_str(&text).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Reload configuration from URL
    pub async fn reload(&self) -> Result<(), ConfigError> {
        let new_config = Self::fetch_config(&self.client, &self.config_url).await?;
        let mut config = self.config.write().await;
        *config = new_config;
        tracing::info!("Configuration reloaded from {}", self.config_url);
        Ok(())
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
