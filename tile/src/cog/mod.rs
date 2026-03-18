//! Cloud Optimized GeoTIFF (COG) reading module.
//!
//! Based on async-cog implementation with enhancements for multi-band nodata handling.

mod bounds;
mod decode;
mod error;
mod interpolate;
mod reader;
mod resample;

pub use bounds::TileBounds;
pub use error::CogError;
pub use reader::CogReader;
