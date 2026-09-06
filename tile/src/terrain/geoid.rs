//! Geoid model wrapper around the `japan-geoid` crate, plus the per-request
//! [`HeightMode`] that selects *which surface* to serve.
//!
//! The **model** (GSIGEO2011, JPGEO2024, JPGEO2024+Hrefconv2024) is a property
//! of the DEM source, because a geoid model is bound to a vertical datum:
//! GSIGEO2011 goes with JGD2011 orthometric heights, JPGEO2024 with JGD2024.
//! Letting a request pick the model would let a caller combine a DEM with a
//! geoid from a different datum and get numbers that mean nothing, so the model
//! is declared per source in the config JSON (falling back to
//! `TERRAIN_DEFAULT_GEOID`) and is *not* selectable per request.
//!
//! What a request may still choose is the [`HeightMode`] — the orthometric DEM
//! as-is, the geoid surface alone, or their sum (ellipsoidal, the default).
//!
//! Out-of-coverage queries return NaN from the underlying crate; callers decide
//! whether to fall back to 0 (partial coverage) or reject the tile (full
//! out-of-coverage).

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
}

impl GeoidModel {
    /// Short stable identifier used in ETag / cache keys / config JSON.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Gsigeo2011 => "gsigeo2011",
            Self::Jpgeo2024 => "jpgeo2024",
            Self::Jpgeo2024Hrefconv => "jpgeo2024-hrefconv",
        }
    }

    /// All known models in a stable order.
    pub fn all() -> &'static [Self] {
        &[Self::Gsigeo2011, Self::Jpgeo2024, Self::Jpgeo2024Hrefconv]
    }

    /// Comma-separated list of accepted spellings, for error messages.
    pub fn valid_values() -> String {
        Self::all()
            .iter()
            .map(|m| m.slug())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Resolve an optional textual model name (from config JSON or an env var)
    /// against a fallback.
    ///
    /// Absent / blank → `fallback`. Present but unparseable → `fallback` *and*
    /// an ERROR log naming both the rejected value and the model actually in
    /// use: a geoid that doesn't match the DEM's datum yields plausible-looking
    /// but meaningless heights, so this must never pass unnoticed.
    pub fn resolve_or(raw: Option<&str>, fallback: GeoidModel, context: &str) -> GeoidModel {
        match raw.map(str::trim).filter(|s| !s.is_empty()) {
            None => fallback,
            Some(s) => match s.parse::<GeoidModel>() {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!(
                        context = %context,
                        error = %e,
                        "Invalid geoid model; falling back to {fallback}"
                    );
                    fallback
                }
            },
        }
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
            _ => Err(UnknownGeoidModel(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct UnknownGeoidModel(pub String);

impl fmt::Display for UnknownGeoidModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown geoid model: `{}` (valid: {})",
            self.0,
            GeoidModel::valid_values()
        )
    }
}

impl std::error::Error for UnknownGeoidModel {}

/// Which vertical surface a terrain request wants back.
///
/// This is the *only* geoid-related knob a request may turn — the model itself
/// comes from the DEM source's configuration. Serving three different surfaces
/// from one DEM source means the mode has to take part in cache keys and ETags
/// exactly like the model does; see `TerrainCacheKey` in `server/terrain.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HeightMode {
    /// The DEM as-is: orthometric heights, no geoid applied.
    Orthometric,
    /// The geoid surface alone — the DEM's elevation is ignored entirely.
    /// Useful for inspecting/validating the model that a source is bound to.
    GeoidOnly,
    /// Orthometric + geoid. The default, and what an unparameterised request
    /// has always returned.
    #[default]
    Ellipsoidal,
}

impl HeightMode {
    /// Short stable identifier used in ETag / cache keys / URLs.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Orthometric => "orthometric",
            Self::GeoidOnly => "geoid",
            Self::Ellipsoidal => "ellipsoidal",
        }
    }

    /// All modes in a stable order.
    pub fn all() -> &'static [Self] {
        &[Self::Orthometric, Self::GeoidOnly, Self::Ellipsoidal]
    }

    /// Canonical spellings, for error messages.
    pub fn valid_values() -> String {
        Self::all()
            .iter()
            .map(|m| m.slug())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Display for HeightMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

impl FromStr for HeightMode {
    type Err = UnknownHeightMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "orthometric" | "ortho" => Ok(Self::Orthometric),
            "geoid" | "geoid-only" | "geoid_only" => Ok(Self::GeoidOnly),
            "ellipsoidal" | "ellipsoid" => Ok(Self::Ellipsoidal),
            _ => Err(UnknownHeightMode(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct UnknownHeightMode(pub String);

impl fmt::Display for UnknownHeightMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown heights mode: `{}` (valid: {}; default: {})",
            self.0,
            HeightMode::valid_values(),
            HeightMode::default()
        )
    }
}

impl std::error::Error for UnknownHeightMode {}

/// Lazily-loaded geoid grid. Each model's data is embedded in the `japan-geoid`
/// crate and parsed on first use. The loaded grid is cached for the lifetime
/// of the process via `OnceLock`.
pub struct Geoid {
    model: GeoidModel,
    grid: &'static MemoryGrid<'static>,
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

    /// Geoid height at (lng, lat) in degrees. Returns NaN outside coverage.
    #[inline]
    pub fn height(&self, lng: f64, lat: f64) -> f64 {
        self.grid.get_height(lng, lat)
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
    pub fn coverage_bbox(&self) -> (f64, f64, f64, f64) {
        match self.model {
            // GSIGEO2011: 15–50°N, 118–160°E (v2.2 extended coverage)
            GeoidModel::Gsigeo2011 => (118.0, 15.0, 160.0, 50.0),
            // JPGEO2024 includes marine areas around Japan's EEZ.
            GeoidModel::Jpgeo2024 => (118.0, 15.0, 160.0, 50.0),
            // JPGEO2024 + Hrefconv (land-only combined)
            GeoidModel::Jpgeo2024Hrefconv => (122.0, 20.0, 154.0, 46.0),
        }
    }

    /// Returns `true` if the tile bounds intersect the geoid coverage box.
    /// Used to short-circuit tiles that lie entirely outside coverage with a 404.
    ///
    /// Uses a simple bbox intersection so it works correctly at all zoom levels
    /// (including low-zoom tiles like z=0 where point sampling would miss Japan).
    pub fn bounds_have_any_coverage(&self, west: f64, south: f64, east: f64, north: f64) -> bool {
        let (cw, cs, ce, cn) = self.coverage_bbox();
        !(east <= cw || west >= ce || north <= cs || south >= cn)
    }
}

fn cached_grid(model: GeoidModel) -> &'static MemoryGrid<'static> {
    static GSIGEO2011: OnceLock<MemoryGrid<'static>> = OnceLock::new();
    static JPGEO2024: OnceLock<MemoryGrid<'static>> = OnceLock::new();
    static JPGEO2024_HREFCONV: OnceLock<MemoryGrid<'static>> = OnceLock::new();

    match model {
        GeoidModel::Gsigeo2011 => GSIGEO2011.get_or_init(load_embedded_gsigeo2011),
        GeoidModel::Jpgeo2024 => JPGEO2024.get_or_init(load_embedded_jpgeo2024),
        GeoidModel::Jpgeo2024Hrefconv => {
            JPGEO2024_HREFCONV.get_or_init(load_embedded_jpgeo2024_hrefconv2024)
        }
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
    fn model_names_are_no_longer_a_mode() {
        // The old debug "no geoid" model is gone — it is now `heights=orthometric`.
        assert!("none".parse::<GeoidModel>().is_err());
        assert!("off".parse::<GeoidModel>().is_err());
    }

    #[test]
    fn unknown_model_message_lists_valid_values() {
        let err = "gsigeo2099".parse::<GeoidModel>().unwrap_err().to_string();
        assert!(err.contains("gsigeo2011"), "{err}");
        assert!(err.contains("jpgeo2024-hrefconv"), "{err}");
    }

    #[test]
    fn height_mode_slug_roundtrip() {
        for m in HeightMode::all() {
            let parsed: HeightMode = m.slug().parse().unwrap();
            assert_eq!(parsed, *m);
        }
    }

    #[test]
    fn height_mode_default_is_ellipsoidal() {
        assert_eq!(HeightMode::default(), HeightMode::Ellipsoidal);
    }

    #[test]
    fn height_mode_accepts_aliases_case_insensitively() {
        assert_eq!(
            "Ortho".parse::<HeightMode>().unwrap(),
            HeightMode::Orthometric
        );
        assert_eq!(
            "geoid-only".parse::<HeightMode>().unwrap(),
            HeightMode::GeoidOnly
        );
        assert_eq!(
            "ELLIPSOID".parse::<HeightMode>().unwrap(),
            HeightMode::Ellipsoidal
        );
    }

    #[test]
    fn unknown_height_mode_message_lists_valid_values() {
        let err = "gsigeo2011".parse::<HeightMode>().unwrap_err().to_string();
        assert!(err.contains("orthometric"), "{err}");
        assert!(err.contains("geoid"), "{err}");
        assert!(err.contains("ellipsoidal"), "{err}");
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
