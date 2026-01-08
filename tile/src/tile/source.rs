//! Tile source trait and error types.

use async_trait::async_trait;
use image::RgbaImage;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TileError {
    #[error("Tile not found")]
    NotFound,
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("COG error: {0}")]
    CogError(String),
    #[error("Image error: {0}")]
    ImageError(String),
    #[error("Out of range")]
    OutOfRange,
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for TileError {
    fn from(e: reqwest::Error) -> Self {
        TileError::HttpError(e.to_string())
    }
}

impl From<image::ImageError> for TileError {
    fn from(e: image::ImageError) -> Self {
        TileError::ImageError(e.to_string())
    }
}

impl From<crate::cog::CogError> for TileError {
    fn from(e: crate::cog::CogError) -> Self {
        TileError::CogError(e.to_string())
    }
}

/// Trait for tile sources.
#[async_trait]
pub trait TileSource: Send + Sync {
    /// Preload metadata for this source (e.g., bounds).
    /// Called during initialization to enable fast rejection in `covers()`.
    /// Default implementation does nothing.
    async fn preload(&self) -> Result<(), TileError> {
        Ok(())
    }

    /// Get a tile at the specified coordinates.
    /// Returns None if the tile is not available (e.g., out of bounds).
    async fn get_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<RgbaImage>, TileError>;

    /// Check if this source covers the specified coordinates.
    /// Used for fast rejection before attempting to fetch.
    fn covers(&self, z: u32, x: u32, y: u32) -> bool;
}
