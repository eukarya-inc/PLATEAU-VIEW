//! Geoid model wrapper around the `japan-geoid` crate.
//!
//! Supports switching between GSIGEO2011, JPGEO2024, and the combined
//! JPGEO2024+Hrefconv2024 model at request time. Out-of-coverage queries
//! return NaN from the underlying crate; callers decide whether to fall
//! back to 0 (partial coverage) or reject the tile (full out-of-coverage).

use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

use japan_geoid::Geoid as GeoidTrait;
use japan_geoid::gsi::{
    MemoryGrid, load_embedded_gsigeo2011, load_embedded_jpgeo2024,
    load_embedded_jpgeo2024_hrefconv2024,
};
use serde::{Deserialize, Serialize};

/// Selectable geoid model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeoidModel {
    /// 日本のジオイド 2011 (v2.2)
    Gsigeo2011,
    /// ジオイド 2024 (for JGD2024, includes marine areas)
    Jpgeo2024,
    /// JPGEO2024 + Hrefconv2024 combined (land-only, smaller)
    Jpgeo2024Hrefconv,
    /// No geoid offset — orthometric DEM served as-is (debug / grounding check).
    /// Geoid coverage check is bypassed, so tiles are served globally.
    None,
}

impl GeoidModel {
    /// Short stable identifier used in ETag / cache keys / URLs.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Gsigeo2011 => "gsigeo2011",
            Self::Jpgeo2024 => "jpgeo2024",
            Self::Jpgeo2024Hrefconv => "jpgeo2024-hrefconv",
            Self::None => "none",
        }
    }

    /// All known models in a stable order.
    pub fn all() -> &'static [Self] {
        &[
            Self::Gsigeo2011,
            Self::Jpgeo2024,
            Self::Jpgeo2024Hrefconv,
            Self::None,
        ]
    }
}

impl fmt::Display for GeoidModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

impl FromStr for GeoidModel {
    type Err = UnknownGeoidModel;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gsigeo2011" => Ok(Self::Gsigeo2011),
            "jpgeo2024" => Ok(Self::Jpgeo2024),
            "jpgeo2024-hrefconv" | "jpgeo2024_hrefconv" => Ok(Self::Jpgeo2024Hrefconv),
            "none" | "off" => Ok(Self::None),
            _ => Err(UnknownGeoidModel(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct UnknownGeoidModel(pub String);

impl fmt::Display for UnknownGeoidModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown geoid model: {}", self.0)
    }
}

impl std::error::Error for UnknownGeoidModel {}

/// Lazily-loaded geoid grid. Each model's data is embedded in the `japan-geoid`
/// crate and parsed on first use. The loaded grid is cached for the lifetime
/// of the process via `OnceLock`. `GeoidModel::None` carries no grid and
/// always returns 0.
pub struct Geoid {
    model: GeoidModel,
    grid: Option<&'static MemoryGrid<'static>>,
}

impl Geoid {
    /// Load (or retrieve the cached) grid for `model`.
    pub fn load(model: GeoidModel) -> Self {
        let grid = cached_grid(model);
        Self { model, grid }
    }

    pub fn model(&self) -> GeoidModel {
        self.model
    }

    /// Geoid height at (lng, lat) in degrees. Returns NaN outside coverage,
    /// or 0 for the `None` model.
    #[inline]
    pub fn height(&self, lng: f64, lat: f64) -> f64 {
        match self.grid {
            Some(g) => g.get_height(lng, lat),
            None => 0.0,
        }
    }

    /// Geoid height, NaN falling back to 0 (used for partial-coverage tiles
    /// where we still want to render the in-coverage pixels and flatten the
    /// rest to the orthometric height).
    #[inline]
    pub fn height_or_zero(&self, lng: f64, lat: f64) -> f64 {
        let h = self.height(lng, lat);
        if h.is_finite() { h } else { 0.0 }
    }

    /// Bounding box (west, south, east, north) of this model's coverage, in degrees.
    /// Intentionally conservative / generous so boundary tiles still get served.
    /// `None` returns `(-180, -90, 180, 90)` (global — no restriction).
    pub fn coverage_bbox(&self) -> (f64, f64, f64, f64) {
        match self.model {
            // GSIGEO2011: 15–50°N, 118–160°E (v2.2 extended coverage)
            GeoidModel::Gsigeo2011 => (118.0, 15.0, 160.0, 50.0),
            // JPGEO2024 includes marine areas around Japan's EEZ.
            GeoidModel::Jpgeo2024 => (118.0, 15.0, 160.0, 50.0),
            // JPGEO2024 + Hrefconv (land-only combined)
            GeoidModel::Jpgeo2024Hrefconv => (122.0, 20.0, 154.0, 46.0),
            GeoidModel::None => (-180.0, -90.0, 180.0, 90.0),
        }
    }

    /// Returns `true` if the tile bounds intersect the geoid coverage box.
    /// Used to short-circuit tiles that lie entirely outside coverage with a 404.
    ///
    /// Uses a simple bbox intersection so it works correctly at all zoom levels
    /// (including low-zoom tiles like z=0 where point sampling would miss Japan).
    /// For `None`, always returns true (no coverage restriction).
    pub fn bounds_have_any_coverage(&self, west: f64, south: f64, east: f64, north: f64) -> bool {
        if matches!(self.model, GeoidModel::None) {
            return true;
        }
        let (cw, cs, ce, cn) = self.coverage_bbox();
        !(east <= cw || west >= ce || north <= cs || south >= cn)
    }
}

fn cached_grid(model: GeoidModel) -> Option<&'static MemoryGrid<'static>> {
    static GSIGEO2011: OnceLock<MemoryGrid<'static>> = OnceLock::new();
    static JPGEO2024: OnceLock<MemoryGrid<'static>> = OnceLock::new();
    static JPGEO2024_HREFCONV: OnceLock<MemoryGrid<'static>> = OnceLock::new();

    match model {
        GeoidModel::Gsigeo2011 => Some(GSIGEO2011.get_or_init(load_embedded_gsigeo2011)),
        GeoidModel::Jpgeo2024 => Some(JPGEO2024.get_or_init(load_embedded_jpgeo2024)),
        GeoidModel::Jpgeo2024Hrefconv => {
            Some(JPGEO2024_HREFCONV.get_or_init(load_embedded_jpgeo2024_hrefconv2024))
        }
        GeoidModel::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_roundtrip() {
        for m in GeoidModel::all() {
            let parsed: GeoidModel = m.slug().parse().unwrap();
            assert_eq!(parsed, *m);
        }
    }

    #[test]
    fn height_inside_japan_is_finite() {
        let g = Geoid::load(GeoidModel::Gsigeo2011);
        // Tokyo
        let h = g.height(139.69, 35.69);
        assert!(h.is_finite(), "expected finite, got {h}");
    }

    #[test]
    fn height_outside_japan_is_nan() {
        let g = Geoid::load(GeoidModel::Gsigeo2011);
        // Middle of the Pacific
        let h = g.height(-150.0, 10.0);
        assert!(h.is_nan(), "expected NaN, got {h}");
    }

    #[test]
    fn bounds_coverage_detects_japan() {
        let g = Geoid::load(GeoidModel::Gsigeo2011);
        // Bounds covering Tokyo.
        assert!(g.bounds_have_any_coverage(139.0, 35.0, 140.0, 36.0));
        // Bounds in the Pacific.
        assert!(!g.bounds_have_any_coverage(-160.0, 5.0, -150.0, 15.0));
    }

    #[test]
    fn bounds_coverage_handles_low_zoom_tiles() {
        let g = Geoid::load(GeoidModel::Gsigeo2011);
        // z=0 x=1 y=0 (eastern hemisphere) — contains Japan.
        assert!(g.bounds_have_any_coverage(0.0, -90.0, 180.0, 90.0));
        // z=0 x=0 y=0 (western hemisphere) — no Japan.
        assert!(!g.bounds_have_any_coverage(-180.0, -90.0, 0.0, 90.0));
    }
}
