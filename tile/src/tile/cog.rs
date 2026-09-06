//! COG tile source implementation.

use std::sync::Arc;

use async_trait::async_trait;
use image::RgbaImage;
use object_store::{
    ObjectStore, client::ClientConfigKey, http::HttpBuilder, path::Path as ObjectPath,
};
use tokio::sync::RwLock;
use url::Url;

use super::{
    coord::xyz_to_bounds,
    source::{TileError, TileSource, single_etag_key},
};
use crate::{
    cog::{CogCrs, CogReader, TileBounds, mercator_tile_bounds},
    config::NoDataConfig,
};

/// COG tile source that generates tiles from a Cloud Optimized GeoTIFF.
pub struct CogTileSource {
    /// COG reader (lazily initialized)
    reader: Arc<RwLock<Option<CogReader>>>,
    /// COG file URL
    url: String,
    /// NoData configuration
    nodata: Option<NoDataConfig>,
    /// Tile size (default: 256)
    tile_size: u32,
    /// Cached bounds
    bounds: Arc<RwLock<Option<TileBounds>>>,
    /// Key for ETag calculation
    etag_key: String,
}

impl CogTileSource {
    pub fn new(url: String, nodata: Option<NoDataConfig>) -> Self {
        let etag_key = format!("cog:{}", url);
        Self {
            reader: Arc::new(RwLock::new(None)),
            url,
            nodata,
            tile_size: 256,
            bounds: Arc::new(RwLock::new(None)),
            etag_key,
        }
    }

    pub fn with_version(url: String, nodata: Option<NoDataConfig>, version: Option<&str>) -> Self {
        let etag_key = match version {
            Some(v) => format!("cog:{}:{}", url, v),
            None => format!("cog:{}", url),
        };
        Self {
            reader: Arc::new(RwLock::new(None)),
            url,
            nodata,
            tile_size: 256,
            bounds: Arc::new(RwLock::new(None)),
            etag_key,
        }
    }

    pub fn with_tile_size(mut self, tile_size: u32) -> Self {
        self.tile_size = tile_size;
        self
    }

    /// Get cached bounds if available.
    pub async fn get_bounds(&self) -> Option<TileBounds> {
        *self.bounds.read().await
    }

    /// Get cached bounds as a `terrain::GeoBounds` (after `preload()`).
    pub async fn cached_geo_bounds(&self) -> Option<crate::terrain::GeoBounds> {
        let b = self.get_bounds().await?;
        Some(crate::terrain::GeoBounds::new(
            b.west, b.south, b.east, b.north,
        ))
    }

    /// Initialize the COG reader if not already done.
    async fn ensure_reader(&self) -> Result<(), TileError> {
        let mut reader_guard = self.reader.write().await;
        if reader_guard.is_some() {
            return Ok(());
        }

        tracing::info!("Opening COG: {}", self.url);

        let (store, path) = self.create_object_store().await?;
        let reader = CogReader::open(store, path).await?;

        // Cache bounds in WGS84 (intersection tests run against WGS84 XYZ tiles).
        if let Some(b) = reader.wgs84_bounds() {
            let mut bounds = self.bounds.write().await;
            *bounds = Some(b);
        }

        *reader_guard = Some(reader);
        Ok(())
    }

    async fn create_object_store(&self) -> Result<(Arc<dyn ObjectStore>, ObjectPath), TileError> {
        let parsed_url =
            Url::parse(&self.url).map_err(|e| TileError::Internal(format!("Invalid URL: {e}")))?;
        let path = object_path_from_url(&parsed_url)?;

        match parsed_url.scheme() {
            "http" | "https" => {
                // Extract base URL and path
                let base_url = format!(
                    "{}://{}{}",
                    parsed_url.scheme(),
                    parsed_url.host_str().unwrap_or(""),
                    parsed_url
                        .port()
                        .map(|p| format!(":{p}"))
                        .unwrap_or_default()
                );

                let store = HttpBuilder::new()
                    .with_url(&base_url)
                    .with_config(ClientConfigKey::AllowHttp, "true")
                    .build()
                    .map_err(|e| {
                        TileError::Internal(format!("Failed to create HTTP store: {e}"))
                    })?;

                Ok((Arc::new(store), path))
            }
            "gs" => {
                // Google Cloud Storage
                let bucket = parsed_url
                    .host_str()
                    .ok_or_else(|| TileError::Internal("Missing GCS bucket".to_string()))?;

                let store = object_store::gcp::GoogleCloudStorageBuilder::new()
                    .with_bucket_name(bucket)
                    .build()
                    .map_err(|e| TileError::Internal(format!("Failed to create GCS store: {e}")))?;

                Ok((Arc::new(store), path))
            }
            "s3" => {
                // AWS S3
                let bucket = parsed_url
                    .host_str()
                    .ok_or_else(|| TileError::Internal("Missing S3 bucket".to_string()))?;

                let store = object_store::aws::AmazonS3Builder::new()
                    .with_bucket_name(bucket)
                    .build()
                    .map_err(|e| TileError::Internal(format!("Failed to create S3 store: {e}")))?;

                Ok((Arc::new(store), path))
            }
            scheme => Err(TileError::Internal(format!(
                "Unsupported URL scheme: {scheme}"
            ))),
        }
    }
}

/// Build the object-store key for a COG URL. See
/// [`crate::object_url::object_path_from_url`] for the encoding rule.
fn object_path_from_url(parsed: &Url) -> Result<ObjectPath, TileError> {
    crate::object_url::object_path_from_url(parsed)
        .map_err(|e| TileError::Internal(format!("Invalid URL path: {e}")))
}

#[async_trait]
impl TileSource for CogTileSource {
    async fn preload(&self) -> Result<(), TileError> {
        // Check if bounds already loaded
        {
            let bounds = self.bounds.read().await;
            if bounds.is_some() {
                return Ok(());
            }
        }

        tracing::info!("Preloading COG bounds: {}", self.url);

        let (store, path) = self.create_object_store().await?;

        // Use read_bounds_only for fast bounds extraction (reads only first IFD)
        let bounds = CogReader::read_bounds_only(store, path).await?;

        // Cache bounds
        let mut bounds_guard = self.bounds.write().await;
        *bounds_guard = bounds;

        if let Some(b) = bounds {
            tracing::info!(
                "Preloaded COG bounds: {} -> west={}, east={}, south={}, north={}",
                self.url,
                b.west,
                b.east,
                b.south,
                b.north
            );
        } else {
            tracing::warn!("COG has no bounds: {}", self.url);
        }

        Ok(())
    }

    async fn get_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<RgbaImage>, TileError> {
        // Ensure reader is initialized
        self.ensure_reader().await?;

        // WGS84 bounds for the intersection short-circuit (cached bounds are WGS84).
        let wgs84_bounds = xyz_to_bounds(z, x, y);

        // Check if tile intersects COG bounds
        {
            let bounds = self.bounds.read().await;
            if let Some(cog_bounds) = &*bounds
                && !cog_bounds.intersects(&wgs84_bounds)
            {
                tracing::debug!(
                    url = %self.url,
                    z = z, x = x, y = y,
                    "COG tile outside bounds, skipping"
                );
                return Ok(None);
            }
        }

        tracing::debug!(
            url = %self.url,
            z = z, x = x, y = y,
            tile_bounds = ?wgs84_bounds,
            "Reading COG tile"
        );

        // Read tile from COG
        let reader_guard = self.reader.read().await;
        let reader = reader_guard
            .as_ref()
            .ok_or_else(|| TileError::Internal("Reader not initialized".to_string()))?;

        // Build the requested-tile bounds in the COG's native CRS so the linear
        // sampling matches the COG's pixel grid (exact for Web Mercator).
        let tile_bounds = match reader.crs() {
            CogCrs::Geographic => wgs84_bounds,
            CogCrs::WebMercator => mercator_tile_bounds(z, x, y),
        };

        let rgba_data = reader
            .read_tile_rgba(&tile_bounds, self.tile_size, self.nodata.as_ref())
            .await?;

        // Convert to image
        let img =
            RgbaImage::from_raw(self.tile_size, self.tile_size, rgba_data).ok_or_else(|| {
                TileError::ImageError("Failed to create image from RGBA data".to_string())
            })?;

        // Check if tile is completely transparent (no data)
        let is_empty = img.pixels().all(|p| p.0[3] == 0);
        if is_empty {
            tracing::debug!(
                url = %self.url,
                z = z, x = x, y = y,
                "COG tile is completely transparent"
            );
            return Ok(None);
        }

        tracing::debug!(
            url = %self.url,
            z = z, x = x, y = y,
            "COG tile generated successfully"
        );

        Ok(Some(img))
    }

    fn covers(&self, z: u32, x: u32, y: u32) -> bool {
        // Try to check cached bounds (non-blocking)
        // If bounds aren't loaded yet or lock is busy, assume it covers
        if let Ok(bounds_guard) = self.bounds.try_read()
            && let Some(cog_bounds) = &*bounds_guard
        {
            let tile_bounds = xyz_to_bounds(z, x, y);
            return cog_bounds.intersects(&tile_bounds);
        }
        // Bounds not yet loaded - assume it covers
        true
    }

    fn etag_keys(&self, z: u32, x: u32, y: u32) -> Vec<String> {
        single_etag_key(&self.etag_key, self.covers(z, x, y))
    }

    async fn bounds(&self) -> Option<crate::terrain::GeoBounds> {
        self.cached_geo_bounds().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Non-ASCII COG filenames must map to a **single**-encoded object key;
    /// `ObjectPath::from` on an already-encoded URL path would re-encode the
    /// `%` signs and 404 on every request.
    #[test]
    fn non_ascii_url_path_is_single_encoded() {
        let url = Url::parse("https://example.com/patch/shizuoka/静岡市（葵区）・DEM.tif").unwrap();
        assert!(url.path().contains('%'));

        let path = object_path_from_url(&url).unwrap();
        // The key holds the *decoded* name; `object_store` percent-encodes it
        // exactly once when it builds the request URL.
        assert_eq!(path.to_string(), "patch/shizuoka/静岡市（葵区）・DEM.tif");
        assert!(
            !path.to_string().contains('%'),
            "an already-encoded key would be encoded a second time and 404"
        );
        // The old `ObjectPath::from(url.path())` form keeps the escapes, which
        // is what produced the double-encoded 404s.
        assert_ne!(path, ObjectPath::from(url.path()));
        assert!(ObjectPath::from(url.path()).to_string().contains('%'));
    }

    #[test]
    fn ascii_url_path_unchanged() {
        let url = Url::parse("https://example.com/base/dem5/5238.tif").unwrap();
        let path = object_path_from_url(&url).unwrap();
        assert_eq!(path.to_string(), "base/dem5/5238.tif");
    }
}
