//! Terrain generation and serving.
//!
//! Provides DEM-backed Cesium quantized-mesh-1.0 tiles and Terrarium raster tiles.
//! Heights default to ellipsoidal (orthometric DEM + geoid via `japan-geoid`);
//! `?heights=` selects the orthometric DEM or the geoid surface alone instead.
//! The geoid *model* is a property of the DEM source, not of the request.

#![allow(dead_code)]

pub mod attribution;
pub mod cached_dem;
pub mod cog_dem;
pub mod composite;
pub mod dem;
pub mod ellipsoid;
pub mod geodetic;
pub mod geoid;
pub mod mapterhorn;
pub mod mesh_gen;
pub mod mirror;
pub mod pmtiles;
pub mod sealevel;
pub mod settings;
pub mod vertical;
pub mod webmercator;
pub mod xyz_dem;

pub use cached_dem::CachedDemProvider;
pub use cog_dem::CogDemSource;
pub use composite::{
    CompositeDemProvider, build as build_composite_dem,
    build_with_correction as build_corrected_composite_dem,
};
pub use dem::{DemError, DemProvider, DemTile, GeoBounds, PixelPositions};
pub use geoid::{
    Geoid, GeoidCoverage, GeoidModel, HeightMode, UnknownGeoidModel, UnknownHeightMode,
};
pub use mapterhorn::MapterhornSource;
pub use mirror::MirrorSource;
pub use pmtiles::{PmtilesEncoding, PmtilesSource};
pub use settings::TerrainSettings;
pub use xyz_dem::{XyzDemEncoding, XyzDemSource};

// DEM tile resampling lives in `terrain-core` (shared with the WebAssembly
// build); re-exported here so existing paths keep working.
pub use terrain_core::resample::DEM_RESAMPLE_VERSION;
pub(crate) use terrain_core::resample::{
    extract_and_upsample, fit_to_tile_size, upsample_subregion,
};
