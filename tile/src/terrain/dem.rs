//! DEM (digital elevation model) source abstraction.
//!
//! Providers return Web Mercator XYZ tiles as f64 elevation grids. NaN marks
//! no-data pixels. An optional per-tile ETag fragment is threaded through so
//! that downstream cache keys can track upstream cache-busting without a
//! manual version bump.

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DemError {
    #[error("dem tile not found")]
    NotFound,
    #[error("http error: {0}")]
    Http(String),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("out of range")]
    OutOfRange,
}

impl From<reqwest::Error> for DemError {
    fn from(e: reqwest::Error) -> Self {
        DemError::Http(e.to_string())
    }
}

impl From<image::ImageError> for DemError {
    fn from(e: image::ImageError) -> Self {
        DemError::Decode(e.to_string())
    }
}

/// Result of fetching a single XYZ DEM tile.
#[derive(Debug, Clone)]
pub struct DemTile {
    /// Row-major, top-to-bottom (north first). Length = tile_size * tile_size.
    pub elevations: Vec<f64>,
    /// Opaque ETag fragment captured from the upstream source, if any.
    pub etag: Option<String>,
}

#[async_trait]
pub trait DemProvider: Send + Sync {
    /// Fetch elevations for a Web Mercator XYZ tile.
    async fn get_tile_elevations(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError>;

    /// Native tile size (pixels) served by the upstream.
    fn native_tile_size(&self) -> u32;

    /// Maximum zoom served.
    fn max_zoom(&self) -> u8;

    /// Stable version identifier for this provider (manual bump).
    fn version(&self) -> &str;

    /// Stable slug used in cache keys / etags.
    fn slug(&self) -> &str;
}
