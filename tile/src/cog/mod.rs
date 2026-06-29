//! Cloud Optimized GeoTIFF (COG) reading module.
//!
//! Based on async-cog implementation with enhancements for multi-band nodata handling.

mod bounds;
mod decode;
mod error;
mod interpolate;
mod reader;
mod resample;

pub use bounds::{CogCrs, TileBounds, mercator_tile_bounds};
pub use decode::MAX_PHYSICAL_ELEVATION_M;
pub use error::CogError;
pub use reader::CogReader;
