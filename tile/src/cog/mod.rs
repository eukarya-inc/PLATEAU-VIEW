//! Cloud Optimized GeoTIFF (COG) reading module.
//!
//! Based on async-cog implementation with enhancements for multi-band nodata handling.

mod bounds;
mod decode;
mod interpolate;
mod reader;

pub use bounds::TileBounds;
pub use reader::{CogError, CogReader};
