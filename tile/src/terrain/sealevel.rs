//! Sea-level (0 m orthometric) DEM base.
//!
//! Returns a flat 0 m elevation grid for every tile. Intended as the composite
//! **base** in place of the Mapterhorn fallback: terrain is then defined purely
//! by the COG overlays, and any pixel not covered by an overlay reads 0 m
//! orthometric. The geoid is added downstream ([`super::ellipsoid`]), so sea /
//! uncovered area renders at the geoid height (mean sea level) — matching the
//! ion reference — instead of Mapterhorn's coastline-mismatched values, which
//! produced tall spikes over the sea near the coast.

use async_trait::async_trait;

use super::dem::{DemError, DemProvider, DemTile};

/// Sentinel `DEM_URL` values that select the sea-level base instead of a real
/// DEM upstream (Mapterhorn / PMTiles).
pub fn is_sea_level_url(url: &str) -> bool {
    matches!(
        url.trim().to_ascii_lowercase().as_str(),
        "sealevel" | "sea-level" | "none" | "0m"
    )
}

/// A [`DemProvider`] that reports 0 m everywhere.
#[derive(Debug, Default)]
pub struct SeaLevelDem;

#[async_trait]
impl DemProvider for SeaLevelDem {
    async fn get_tile_elevations(
        &self,
        _z: u8,
        _x: u32,
        _y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError> {
        let n = (tile_size as usize) * (tile_size as usize);
        Ok(DemTile {
            elevations: vec![0.0; n],
            etag: None,
        })
    }

    fn native_tile_size(&self) -> u32 {
        256
    }

    /// High enough not to cap the composite's max zoom; overlays define the
    /// real detail ceiling.
    fn max_zoom(&self) -> u8 {
        20
    }

    fn version(&self) -> &str {
        "1"
    }

    fn slug(&self) -> &str {
        "sealevel"
    }
    // bounds() defaults to None (global): sea level applies everywhere an
    // overlay doesn't.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_flat_zero() {
        let d = SeaLevelDem;
        let t = d.get_tile_elevations(10, 1, 1, 8).await.unwrap();
        assert_eq!(t.elevations.len(), 64);
        assert!(t.elevations.iter().all(|&h| h == 0.0));
    }

    #[test]
    fn detects_sentinels() {
        assert!(is_sea_level_url("sealevel"));
        assert!(is_sea_level_url("None"));
        assert!(!is_sea_level_url(
            "https://tiles.mapterhorn.com/{z}/{x}/{y}.webp"
        ));
    }
}
