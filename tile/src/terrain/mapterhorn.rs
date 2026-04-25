//! Mapterhorn DEM source.
//!
//! Fetches 512px Terrarium-encoded WebP tiles from
//! `https://tiles.mapterhorn.com/{z}/{x}/{y}.webp` (see <https://mapterhorn.com>)
//! and decodes them to f64 orthometric elevations. The upstream's `ETag`
//! response header is captured per-tile so that downstream cache keys can
//! invalidate automatically on DEM data refresh.

use async_trait::async_trait;
use image::GenericImageView;
use reqwest::StatusCode;

use super::dem::{DemError, DemProvider, DemTile};
use super::terrarium::rgb_to_elevation;

/// Default public Mapterhorn tile endpoint.
pub const DEFAULT_URL_TEMPLATE: &str = "https://tiles.mapterhorn.com/{z}/{x}/{y}.webp";

/// Native Mapterhorn tile size (pixels).
pub const MAPTERHORN_NATIVE_TILE_SIZE: u32 = 512;

/// Documented maximum zoom as of 2025.
pub const MAPTERHORN_DEFAULT_MAX_ZOOM: u8 = 15;

pub struct MapterhornSource {
    url_template: String,
    client: reqwest::Client,
    max_zoom: u8,
    version: String,
    slug: String,
}

impl MapterhornSource {
    pub fn new(url_template: impl Into<String>, version: impl Into<String>, max_zoom: u8) -> Self {
        Self {
            url_template: url_template.into(),
            client: reqwest::Client::new(),
            max_zoom,
            version: version.into(),
            slug: "mapterhorn".to_string(),
        }
    }

    pub fn default_config() -> Self {
        Self::new(DEFAULT_URL_TEMPLATE, "v1", MAPTERHORN_DEFAULT_MAX_ZOOM)
    }

    fn build_url(&self, z: u8, x: u32, y: u32) -> String {
        self.url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string())
    }
}

#[async_trait]
impl DemProvider for MapterhornSource {
    async fn get_tile_elevations(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError> {
        if z > self.max_zoom {
            return Err(DemError::OutOfRange);
        }

        let url = self.build_url(z, x, y);
        let response = self.client.get(&url).send().await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(DemError::NotFound);
        }
        if !status.is_success() {
            return Err(DemError::Http(format!("HTTP {status}")));
        }

        // Capture upstream ETag for cache-key composition (strip weak prefix / quotes).
        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_start_matches("W/").trim_matches('"').to_string())
            .filter(|s| !s.is_empty());

        let bytes = response.bytes().await?;
        let img = image::load_from_memory(&bytes)?;
        let (src_w, src_h) = img.dimensions();

        // Decode to f64 elevations at the native resolution (row-major, north first).
        let rgba = img.to_rgba8();
        let mut native: Vec<f64> = Vec::with_capacity((src_w * src_h) as usize);
        for pixel in rgba.pixels() {
            native.push(rgb_to_elevation(image::Rgb([pixel[0], pixel[1], pixel[2]])));
        }

        // If the caller asked for a different tile size, bilinear-resample.
        let elevations = if src_w == tile_size && src_h == tile_size {
            native
        } else {
            super::resample_bilinear(&native, src_w, src_h, tile_size, tile_size)
        };

        Ok(DemTile { elevations, etag })
    }

    fn native_tile_size(&self) -> u32 {
        MAPTERHORN_NATIVE_TILE_SIZE
    }

    fn max_zoom(&self) -> u8 {
        self.max_zoom
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn slug(&self) -> &str {
        &self.slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_interpolation() {
        let src = MapterhornSource::new("https://ex.com/{z}/{x}/{y}.webp", "v1", 15);
        assert_eq!(
            src.build_url(10, 100, 200),
            "https://ex.com/10/100/200.webp"
        );
    }
}
