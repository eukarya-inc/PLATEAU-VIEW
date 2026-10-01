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

/// Version of how COG pixels are sampled (coordinate convention, window
/// margin, edge handling). Mixed into the cache keys / ETags of everything a
/// COG contributes to — `/tiles` COG layers and COG DEM overlays — so a change
/// here re-renders exactly the tiles it affects. Bump on any change that
/// alters sampled values for unchanged COG input.
///
/// `centre-v1`: interpolate between pixel centres. Before it, edge-based
/// coordinates were fed to centre-based interpolators and every read was half
/// a source pixel north-west of where it belongs.
pub const COG_SAMPLING_VERSION: &str = "centre-v1";
pub use decode::MAX_PHYSICAL_ELEVATION_M;
pub use error::CogError;
pub use reader::CogReader;
