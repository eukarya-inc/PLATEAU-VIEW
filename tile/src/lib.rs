//! # tile
//!
//! High-performance tile server with Cloud Optimized GeoTIFF (COG) overlay support.
//!
//! ## Features
//!
//! - XYZ tile proxy from remote tile servers
//! - COG tile generation with HTTP range requests
//! - Multiple COG overlay composition
//! - NoData value handling (single/multi-band, multiple patterns)
//! - Automatic overview selection for optimal performance
//! - Bilinear interpolation for smooth rendering
//! - Memory and GCS caching
//! - Configuration via remote JSON file with auto-reload

pub mod cache;
pub mod cog;
pub mod config;
pub mod server;
pub mod tile;

pub use config::{Config, ConfigManager};
