//! COG error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CogError {
    #[error("Failed to open COG file: {0}")]
    OpenError(String),
    #[error("Failed to read COG data: {0}")]
    ReadError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("TIFF error: {0}")]
    TiffError(String),
    #[error("No IFD available")]
    NoIfd,
    #[error(
        "Unsupported CRS: expected geographic (EPSG:4326/6668) or Web Mercator (EPSG:3857), got EPSG:{0}"
    )]
    UnsupportedCrs(u16),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Object store error: {0}")]
    ObjectStoreError(String),
}
