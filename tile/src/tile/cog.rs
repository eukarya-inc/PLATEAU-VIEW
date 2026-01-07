//! COG tile source implementation.

use std::sync::Arc;

use async_trait::async_trait;
use image::RgbaImage;
use object_store::{
    client::ClientConfigKey, http::HttpBuilder, path::Path as ObjectPath, ObjectStore,
};
use tokio::sync::RwLock;
use url::Url;

use super::{
    coord::xyz_to_bounds,
    source::{TileError, TileSource},
};
use crate::{
    cog::{CogReader, TileBounds},
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
}

impl CogTileSource {
    pub fn new(url: String, nodata: Option<NoDataConfig>) -> Self {
        Self {
            reader: Arc::new(RwLock::new(None)),
            url,
            nodata,
            tile_size: 256,
            bounds: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_tile_size(mut self, tile_size: u32) -> Self {
        self.tile_size = tile_size;
        self
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

        // Cache bounds
        if let Some(b) = reader.bounds() {
            let mut bounds = self.bounds.write().await;
            *bounds = Some(*b);
        }

        *reader_guard = Some(reader);
        Ok(())
    }

    async fn create_object_store(&self) -> Result<(Arc<dyn ObjectStore>, ObjectPath), TileError> {
        let parsed_url =
            Url::parse(&self.url).map_err(|e| TileError::Internal(format!("Invalid URL: {e}")))?;

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

                let path = ObjectPath::from(parsed_url.path());

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

                let path = ObjectPath::from(parsed_url.path());

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

                let path = ObjectPath::from(parsed_url.path());

                Ok((Arc::new(store), path))
            }
            scheme => Err(TileError::Internal(format!(
                "Unsupported URL scheme: {scheme}"
            ))),
        }
    }
}

#[async_trait]
impl TileSource for CogTileSource {
    async fn get_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<RgbaImage>, TileError> {
        // Ensure reader is initialized
        self.ensure_reader().await?;

        let tile_bounds = xyz_to_bounds(z, x, y);

        // Check if tile intersects COG bounds
        {
            let bounds = self.bounds.read().await;
            if let Some(cog_bounds) = &*bounds {
                if !cog_bounds.intersects(&tile_bounds) {
                    tracing::debug!(
                        url = %self.url,
                        z = z, x = x, y = y,
                        "COG tile outside bounds, skipping"
                    );
                    return Ok(None);
                }
            }
        }

        tracing::debug!(
            url = %self.url,
            z = z, x = x, y = y,
            tile_bounds = ?tile_bounds,
            "Reading COG tile"
        );

        // Read tile from COG
        let reader_guard = self.reader.read().await;
        let reader = reader_guard
            .as_ref()
            .ok_or_else(|| TileError::Internal("Reader not initialized".to_string()))?;

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

    fn covers(&self, _z: u32, _x: u32, _y: u32) -> bool {
        // COG bounds are checked in get_tile after lazy initialization
        true
    }
}
