//! Raster PMTiles tile source.
//!
//! Reads image tiles (PNG / WebP / JPEG) from a PMTiles archive and returns
//! them as RGBA so they can be composited with other layers in the
//! `/tiles/...` rendering pipeline.

use std::sync::Arc;

use async_trait::async_trait;
use image::RgbaImage;
use pmtiles::{AsyncPmTilesReader, ObjectStoreBackend, TileCoord};
use tokio::sync::OnceCell;

use super::source::{TileError, TileSource, single_etag_key};
use crate::config::RangeConfig;

pub struct PmtilesTileSource {
    url: String,
    range: Option<RangeConfig>,
    etag_key: String,
    reader: OnceCell<Arc<AsyncPmTilesReader<ObjectStoreBackend>>>,
}

impl PmtilesTileSource {
    pub fn new(url: String, range: Option<RangeConfig>) -> Self {
        let etag_key = format!("pmtiles:{url}");
        Self {
            url,
            range,
            etag_key,
            reader: OnceCell::new(),
        }
    }

    pub fn with_version(url: String, range: Option<RangeConfig>, version: Option<&str>) -> Self {
        let etag_key = match version {
            Some(v) => format!("pmtiles:{url}:{v}"),
            None => format!("pmtiles:{url}"),
        };
        Self {
            url,
            range,
            etag_key,
            reader: OnceCell::new(),
        }
    }

    async fn reader(&self) -> Result<Arc<AsyncPmTilesReader<ObjectStoreBackend>>, TileError> {
        self.reader
            .get_or_try_init(|| async {
                let (store, path) = crate::terrain::pmtiles::build_object_store_for(&self.url)?;
                let backend = ObjectStoreBackend::new(store, path);
                AsyncPmTilesReader::try_from_source(backend)
                    .await
                    .map(Arc::new)
                    .map_err(|e| TileError::Internal(format!("pmtiles open: {e}")))
            })
            .await
            .cloned()
    }
}

#[async_trait]
impl TileSource for PmtilesTileSource {
    async fn preload(&self) -> Result<(), TileError> {
        let _ = self.reader().await?;
        Ok(())
    }

    async fn get_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<RgbaImage>, TileError> {
        if !self.covers(z, x, y) {
            return Ok(None);
        }
        let reader = self.reader().await?;
        let coord = TileCoord::new(z as u8, x, y)
            .map_err(|e| TileError::Internal(format!("invalid tile coord {z}/{x}/{y}: {e}")))?;
        let bytes = reader
            .get_tile_decompressed(coord)
            .await
            .map_err(|e| TileError::HttpError(format!("pmtiles get_tile: {e}")))?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
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
    fn etag_key_includes_version() {
        let s = PmtilesTileSource::with_version(
            "https://e.com/x.pmtiles".to_string(),
            None,
            Some("v2"),
        );
        assert!(s.etag_keys(0, 0, 0)[0].contains(":v2"));
    }
}
