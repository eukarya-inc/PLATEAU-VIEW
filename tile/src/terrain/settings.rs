//! Runtime settings for the terrain endpoint.
//!
//! These come from environment variables, not the config JSON, because the
//! base DEM is an operational concern (which mirror you point at) rather
//! than a content concern. The config JSON only describes overlay sources
//! served under `/tiles/...`.

use std::env;
use std::sync::Arc;

use super::dem::DemProvider;
use super::geoid::GeoidModel;
use super::mapterhorn::{
    DEFAULT_URL_TEMPLATE as DEFAULT_MAPTERHORN_URL, MAPTERHORN_DEFAULT_MAX_ZOOM,
    MAPTERHORN_NATIVE_TILE_SIZE, MapterhornSource,
};
use super::pmtiles::{PmtilesEncoding, PmtilesSource};

const DEFAULT_TERRAIN_TILE_SIZE: u32 = 256;
const DEFAULT_TERRAIN_MAX_ZOOM: u8 = 18;
const DEFAULT_TERRAIN_MAX_ERROR: f64 = 5.0;
const DEFAULT_DEM_VERSION: &str = "v1";

/// All terrain endpoint settings, resolved from environment variables.
#[derive(Debug, Clone)]
pub struct TerrainSettings {
    /// Base DEM URL. If `None`, falls back to the public Mapterhorn endpoint.
    /// If the URL ends with `.pmtiles`, a PMTiles source is built; otherwise
    /// the URL is treated as a Mapterhorn-style `{z}/{x}/{y}` template.
    pub dem_url: Option<String>,
    /// Internal cache-key version for the DEM (mixed into ETag).
    pub dem_version: String,
    /// Upstream max zoom. Used to clamp /terrain/ requests at high zoom.
    pub dem_max_zoom: u8,
    /// Native tile pixel size in the upstream (PMTiles archives only).
    pub dem_native_tile_size: u32,
    /// Output Terrarium raster size.
    pub tile_size: u32,
    /// Default geoid model.
    pub default_geoid: GeoidModel,
    /// Max zoom advertised in `layer.json`.
    pub max_zoom: u8,
    /// Martini mesh-simplification error (meters).
    pub max_error: f64,
    /// Pre-rendered quantized-mesh mirror URL. When set, /terrain/ (no
    /// `{name}`) and /terrain/mirror/ and /terrain-mirror/ pass-through to
    /// this bucket. Supports `file://`, `gs://`, `s3://`, `r2://`.
    pub mirror_url: Option<String>,
}

impl TerrainSettings {
    /// Read settings from environment variables.
    pub fn from_env() -> Self {
        Self {
            dem_url: env::var("DEM_URL").ok().filter(|s| !s.is_empty()),
            dem_version: env::var("DEM_VERSION")
                .unwrap_or_else(|_| DEFAULT_DEM_VERSION.to_string()),
            dem_max_zoom: parse_env_u8("DEM_MAX_ZOOM", MAPTERHORN_DEFAULT_MAX_ZOOM),
            dem_native_tile_size: parse_env_u32(
                "DEM_NATIVE_TILE_SIZE",
                MAPTERHORN_NATIVE_TILE_SIZE,
            ),
            tile_size: parse_env_u32("TERRAIN_TILE_SIZE", DEFAULT_TERRAIN_TILE_SIZE),
            default_geoid: env::var("TERRAIN_DEFAULT_GEOID")
                .ok()
                .and_then(|s| s.parse::<GeoidModel>().ok())
                .unwrap_or(GeoidModel::Gsigeo2011),
            max_zoom: parse_env_u8("TERRAIN_MAX_ZOOM", DEFAULT_TERRAIN_MAX_ZOOM),
            max_error: parse_env_f64("TERRAIN_MAX_ERROR", DEFAULT_TERRAIN_MAX_ERROR),
            mirror_url: env::var("TERRAIN_MIRROR_URL")
                .ok()
                .filter(|s| !s.is_empty()),
        }
    }

    /// Build the DEM provider for these settings. URLs ending in `.pmtiles`
    /// are served via [`PmtilesSource`]; everything else (including the
    /// default fallback) is served via [`MapterhornSource`].
    pub fn build_dem(&self) -> Arc<dyn DemProvider> {
        let url = self
            .dem_url
            .clone()
            .unwrap_or_else(|| DEFAULT_MAPTERHORN_URL.to_string());

        if is_pmtiles_url(&url) {
            tracing::info!(url = %url, "Terrain DEM source: PMTiles");
            Arc::new(PmtilesSource::new(
                url,
                PmtilesEncoding::Terrarium,
                self.dem_version.clone(),
                self.dem_max_zoom,
                self.dem_native_tile_size,
            ))
        } else {
            tracing::info!(url = %url, "Terrain DEM source: Mapterhorn (XYZ)");
            Arc::new(MapterhornSource::new(
                url,
                self.dem_version.clone(),
                self.dem_max_zoom,
            ))
        }
    }
}

fn is_pmtiles_url(url: &str) -> bool {
    let stem = url.split(['?', '#']).next().unwrap_or(url);
    stem.ends_with(".pmtiles")
}

fn parse_env_u8(name: &str, default: u8) -> u8 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn parse_env_u32(name: &str, default: u32) -> u32 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn parse_env_f64(name: &str, default: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pmtiles_urls() {
        assert!(is_pmtiles_url("https://example.com/japan.pmtiles"));
        assert!(is_pmtiles_url("https://x.r2.dev/dem.pmtiles?token=abc"));
        assert!(!is_pmtiles_url(
            "https://tiles.mapterhorn.com/{z}/{x}/{y}.webp"
        ));
    }
}
