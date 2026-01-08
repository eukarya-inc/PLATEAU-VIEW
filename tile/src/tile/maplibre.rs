//! MapLibre style-based tile source implementation.
//!
//! This module requires the `maplibre` feature to be enabled and is only
//! supported on Linux. It uses the maplibre-native library for server-side
//! rendering of MapLibre style.json files.

#[cfg(feature = "maplibre")]
use std::path::PathBuf;
#[cfg(feature = "maplibre")]
use std::sync::Arc;

use async_trait::async_trait;
use image::RgbaImage;
#[cfg(feature = "maplibre")]
use tokio::sync::RwLock;

use super::source::{TileError, TileSource};
use crate::config::RangeConfig;

/// MapLibre style-based tile source that renders tiles from a style.json.
pub struct MaplibreTileSource {
    /// Original style URL (file://, http://, https://)
    #[cfg(feature = "maplibre")]
    style_url: String,
    /// Cached local path to style.json (for remote URLs)
    #[cfg(feature = "maplibre")]
    cached_style_path: Arc<RwLock<Option<PathBuf>>>,
    /// Optional zoom range restriction
    range: Option<RangeConfig>,
    /// Style URL (stored for non-maplibre builds for logging purposes)
    #[cfg(not(feature = "maplibre"))]
    #[allow(dead_code)]
    style_url: String,
}

impl MaplibreTileSource {
    /// Create a new MapLibre tile source.
    ///
    /// # Arguments
    /// * `style_url` - URL to the style.json file (file://, http://, https://)
    /// * `range` - Optional zoom/coordinate range restriction
    #[cfg(feature = "maplibre")]
    pub fn new(style_url: String, range: Option<RangeConfig>) -> Self {
        Self {
            style_url,
            cached_style_path: Arc::new(RwLock::new(None)),
            range,
        }
    }

    /// Create a new MapLibre tile source (stub for non-maplibre builds).
    #[cfg(not(feature = "maplibre"))]
    pub fn new(style_url: String, range: Option<RangeConfig>) -> Self {
        Self { style_url, range }
    }

    /// Ensure style.json is available locally and return the path.
    #[cfg(feature = "maplibre")]
    async fn ensure_style(&self) -> Result<PathBuf, TileError> {
        // Check if already cached
        {
            let cached = self.cached_style_path.read().await;
            if let Some(path) = &*cached {
                return Ok(path.clone());
            }
        }

        let path = if let Some(file_path) = self.style_url.strip_prefix("file://") {
            // Local file: use directly
            PathBuf::from(file_path)
        } else if self.style_url.starts_with("http://") || self.style_url.starts_with("https://") {
            // Remote URL: download to temp directory
            self.download_style().await?
        } else {
            // Assume it's a local path without prefix
            PathBuf::from(&self.style_url)
        };

        // Verify file exists
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Err(TileError::Internal(format!(
                "Style file not found: {}",
                path.display()
            )));
        }

        // Cache the path
        let mut cached = self.cached_style_path.write().await;
        *cached = Some(path.clone());

        Ok(path)
    }

    /// Download style.json from remote URL to a temp file.
    #[cfg(feature = "maplibre")]
    async fn download_style(&self) -> Result<PathBuf, TileError> {
        let client = reqwest::Client::new();
        let response = client
            .get(&self.style_url)
            .send()
            .await
            .map_err(|e| TileError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TileError::HttpError(format!(
                "Failed to download style: HTTP {}",
                response.status()
            )));
        }

        let content = response
            .text()
            .await
            .map_err(|e| TileError::HttpError(e.to_string()))?;

        // Create temp directory for styles if it doesn't exist
        let temp_dir = std::env::temp_dir().join("maplibre-styles");
        tokio::fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| TileError::Internal(format!("Failed to create temp dir: {e}")))?;

        // Generate filename from URL hash
        let hash = format!("{:x}", md5_hash(&self.style_url));
        let style_path = temp_dir.join(format!("{hash}.json"));

        // Write style content
        tokio::fs::write(&style_path, content)
            .await
            .map_err(|e| TileError::Internal(format!("Failed to write style file: {e}")))?;

        tracing::info!("Downloaded style to: {}", style_path.display());

        Ok(style_path)
    }
}

/// Simple hash function for URL to filename conversion.
#[cfg(feature = "maplibre")]
fn md5_hash(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

#[async_trait]
impl TileSource for MaplibreTileSource {
    #[cfg(feature = "maplibre")]
    async fn get_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<RgbaImage>, TileError> {
        // Check range first
        if !self.covers(z, x, y) {
            return Ok(None);
        }

        let style_path = self.ensure_style().await?;

        // Use the global render pool
        let pool = maplibre_native::SingleThreadedRenderPool::global_pool();

        // render_tile expects z as u8
        let z_u8: u8 = z.try_into().map_err(|_| TileError::OutOfRange)?;

        let image = pool
            .render_tile(style_path, z_u8, x, y)
            .await
            .map_err(|e| TileError::Internal(format!("MapLibre render error: {e:?}")))?;

        // Convert maplibre Image to RgbaImage
        let rgba = image.as_image().clone();

        // Resize to 256x256 if needed (MapLibre default is 512x512)
        let rgba = if rgba.width() != 256 || rgba.height() != 256 {
            image::imageops::resize(&rgba, 256, 256, image::imageops::FilterType::Lanczos3)
        } else {
            rgba
        };

        Ok(Some(rgba))
    }

    #[cfg(not(feature = "maplibre"))]
    async fn get_tile(&self, _z: u32, _x: u32, _y: u32) -> Result<Option<RgbaImage>, TileError> {
        // Return NotFound (404) when maplibre feature is not enabled
        Err(TileError::NotFound)
    }

    fn covers(&self, z: u32, x: u32, y: u32) -> bool {
        // Check range if configured
        if let Some(ref range) = self.range {
            return range.contains(z, x, y);
        }
        // MapLibre styles can render any tile within valid zoom range (0-22)
        z <= 22
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let source = MaplibreTileSource::new("file:///path/to/style.json".to_string(), None);
        assert_eq!(source.style_url, "file:///path/to/style.json");
    }

    #[test]
    fn test_covers_no_range() {
        let source = MaplibreTileSource::new("file:///style.json".to_string(), None);
        assert!(source.covers(0, 0, 0));
        assert!(source.covers(15, 100, 100));
        assert!(source.covers(22, 1000, 1000));
        assert!(!source.covers(23, 0, 0)); // Beyond max zoom
    }

    #[test]
    fn test_covers_with_range() {
        let range = RangeConfig {
            z_min: Some(5),
            z_max: Some(15),
            x_min: None,
            x_max: None,
            y_min: None,
            y_max: None,
        };
        let source = MaplibreTileSource::new("file:///style.json".to_string(), Some(range));
        assert!(!source.covers(4, 0, 0));
        assert!(source.covers(5, 0, 0));
        assert!(source.covers(10, 100, 100));
        assert!(source.covers(15, 100, 100));
        assert!(!source.covers(16, 0, 0));
    }

    #[test]
    #[cfg(feature = "maplibre")]
    fn test_md5_hash() {
        let hash1 = md5_hash("https://example.com/style.json");
        let hash2 = md5_hash("https://example.com/style.json");
        let hash3 = md5_hash("https://other.com/style.json");
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
