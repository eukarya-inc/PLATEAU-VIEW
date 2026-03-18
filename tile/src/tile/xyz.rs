//! XYZ tile source implementation.

use async_trait::async_trait;
use image::RgbaImage;

use super::source::{TileError, TileSource, single_etag_key};
use crate::config::RangeConfig;

/// XYZ tile source that fetches tiles from a remote URL.
pub struct XyzTileSource {
    /// URL template with {z}, {x}, {y} placeholders
    url_template: String,
    /// Optional coordinate range restriction
    range: Option<RangeConfig>,
    /// HTTP client
    client: reqwest::Client,
    /// Key for ETag calculation (typically "xyz:url:version")
    etag_key: String,
}

impl XyzTileSource {
    pub fn new(url_template: String, range: Option<RangeConfig>) -> Self {
        let etag_key = format!("xyz:{}", url_template);
        Self {
            url_template,
            range,
            client: reqwest::Client::new(),
            etag_key,
        }
    }

    pub fn with_version(
        url_template: String,
        range: Option<RangeConfig>,
        version: Option<&str>,
    ) -> Self {
        let etag_key = match version {
            Some(v) => format!("xyz:{}:{}", url_template, v),
            None => format!("xyz:{}", url_template),
        };
        Self {
            url_template,
            range,
            client: reqwest::Client::new(),
            etag_key,
        }
    }

    fn build_url(&self, z: u32, x: u32, y: u32) -> String {
        self.url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string())
    }
}

#[async_trait]
impl TileSource for XyzTileSource {
    async fn get_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<RgbaImage>, TileError> {
        if !self.covers(z, x, y) {
            return Ok(None);
        }

        let url = self.build_url(z, x, y);
        tracing::debug!(url = %url, z = z, x = x, y = y, "Fetching XYZ tile");

        let response = self.client.get(&url).send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            tracing::debug!(url = %url, "XYZ tile not found (404)");
            return Ok(None);
        }

        if !status.is_success() {
            tracing::warn!(url = %url, status = %status, "XYZ tile fetch failed");
            return Err(TileError::HttpError(format!("HTTP {status}")));
        }

        let bytes = response.bytes().await?;
        tracing::debug!(url = %url, bytes = bytes.len(), "XYZ tile fetched successfully");

        let img = image::load_from_memory(&bytes)?;

        Ok(Some(img.to_rgba8()))
    }

    fn covers(&self, z: u32, x: u32, y: u32) -> bool {
        match &self.range {
            Some(range) => range.contains(z, x, y),
            None => true,
        }
    }

    fn etag_keys(&self, z: u32, x: u32, y: u32) -> Vec<String> {
        single_etag_key(&self.etag_key, self.covers(z, x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let source = XyzTileSource::new("https://example.com/{z}/{x}/{y}.png".to_string(), None);
        assert_eq!(
            source.build_url(10, 909, 403),
            "https://example.com/10/909/403.png"
        );
    }

    #[test]
    fn test_covers_no_range() {
        let source = XyzTileSource::new("https://example.com/{z}/{x}/{y}.png".to_string(), None);
        assert!(source.covers(0, 0, 0));
        assert!(source.covers(20, 1000000, 1000000));
    }

    #[test]
    fn test_covers_with_range() {
        let source = XyzTileSource::new(
            "https://example.com/{z}/{x}/{y}.png".to_string(),
            Some(RangeConfig {
                z_min: Some(5),
                z_max: Some(15),
                x_min: None,
                x_max: None,
                y_min: None,
                y_max: None,
            }),
        );
        assert!(!source.covers(4, 0, 0));
        assert!(source.covers(5, 0, 0));
        assert!(source.covers(15, 0, 0));
        assert!(!source.covers(16, 0, 0));
    }
}
