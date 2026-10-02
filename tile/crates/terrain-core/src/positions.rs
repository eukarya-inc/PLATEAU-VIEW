//! Exact sample positions of a DEM tile's elevations.
//!
//! A DEM provider returns an `n × n` grid for Web Mercator tile `z/x/y`, but
//! *where* each element was evaluated depends on how the provider produced
//! it (native pixel centres, a resize, an upsample from an ancestor, a
//! geographic COG read in linear lat/lon). The height correction must sample
//! ΔH at exactly those points, so the providers record them as
//! [`PixelPositions`].

/// How a provider maps an index along one tile axis to a fraction of the tile
/// (0 = west / north edge, 1 = east / south edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPositions {
    /// `(i + 0.5) / n` — pixel centres of an `n`-pixel tile.
    Centres { n: u32 },
    /// What [`crate::resample::resample_bilinear`] produces when it resizes a
    /// `src`-pixel tile to `dst` pixels (centre-aligned): output `i` reads
    /// source position `s = clamp((i + 0.5)·src/dst − 0.5, 0, src − 1)`, whose
    /// centre is at `(s + 0.5) / src`. Unclamped that is exactly
    /// `(i + 0.5) / dst`; the clamp (upsampling only) pins the outer half
    /// pixel to the edge sample, and the value there *is* the edge sample, so
    /// that is where it lies.
    Resampled { src: u32, dst: u32 },
}

impl AxisPositions {
    /// Tile fraction of whole pixel `i`.
    pub fn pixel_fraction(self, i: u32) -> f64 {
        match self {
            Self::Centres { n } => (i as f64 + 0.5) / n as f64,
            Self::Resampled { src, dst } => {
                // `saturating_sub`: `src == 0` is malformed (a zero-pixel
                // tile); it yields a non-finite fraction instead of panicking.
                let s = ((i as f64 + 0.5) * src as f64 / dst as f64 - 0.5)
                    .clamp(0.0, src.saturating_sub(1) as f64);
                (s + 0.5) / src as f64
            }
        }
    }

    /// Tile fraction of fractional index `p` on an `n`-pixel grid, as
    /// `upsample_subregion` reads it: the bilinear blend of the two
    /// neighbouring pixels' positions, neighbours clamped to the grid (the
    /// value is that same blend of their values, so this is where it lies).
    pub fn blended_fraction(self, p: f64, n: u32) -> f64 {
        let x0 = p.floor();
        let d = p - x0;
        // `max(0)`: `n == 0` is malformed; `clamp` would panic on it.
        let last = (n as i64 - 1).max(0);
        let a = (x0 as i64).clamp(0, last) as u32;
        let b = (x0 as i64).saturating_add(1).clamp(0, last) as u32;
        let (fa, fb) = (self.pixel_fraction(a), self.pixel_fraction(b));
        if d == 0.0 {
            fa
        } else {
            fa * (1.0 - d) + fb * d
        }
    }
}

/// The space in which a tile's fractions are linear along the y axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileSpace {
    /// Web Mercator: latitude follows the Mercator y of the tile (XYZ tiles,
    /// Mapterhorn, PMTiles, Web-Mercator COGs).
    Mercator,
    /// Latitude interpolated linearly between the tile's north and south
    /// edges — how the server's `CogDemSource` samples a geographic
    /// (EPSG:4326 / 6668) COG for an XYZ request.
    LinearLatLon,
}

/// Exact sample positions of a DEM tile's elevations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelPositions {
    pub space: TileSpace,
    pub x: AxisPositions,
    pub y: AxisPositions,
    /// Set when the tile was bilinear-upsampled from an ancestor (see the
    /// server's `CompositeDemProvider::fetch_base_upsampled`): the axes above
    /// then describe the *ancestor's* grid and this records how index `i` of
    /// the served tile maps onto it.
    pub upsampled: Option<Upsampled>,
}

/// Child-in-parent mapping of [`crate::resample::upsample_subregion`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Upsampled {
    /// `2^(child z − parent z)`.
    pub factor: u32,
    pub sub_x: u32,
    pub sub_y: u32,
    /// Pixel size of both the parent grid and the served tile.
    pub tile_size: u32,
}

impl PixelPositions {
    /// The XYZ convention (what `positions: None` means).
    pub fn centres(n: u32) -> Self {
        Self {
            space: TileSpace::Mercator,
            x: AxisPositions::Centres { n },
            y: AxisPositions::Centres { n },
            upsampled: None,
        }
    }

    /// A Mercator tile decoded at `src_w × src_h` and resized to `dst` by
    /// [`crate::resample::resample_bilinear`] (identity when the sizes match).
    pub fn resampled(src_w: u32, src_h: u32, dst: u32) -> Self {
        if src_w == dst && src_h == dst {
            return Self::centres(dst);
        }
        Self {
            space: TileSpace::Mercator,
            x: AxisPositions::Resampled { src: src_w, dst },
            y: AxisPositions::Resampled { src: src_h, dst },
            upsampled: None,
        }
    }

    /// Longitude / latitude (degrees) at which element `(i, j)` (column, row)
    /// of tile `z/x/y` was evaluated.
    ///
    /// Malformed input (an upsample factor of 0 or deeper than `z`, `z ≥ 64`)
    /// gives non-finite or meaningless coordinates, never a panic.
    pub fn lonlat(&self, z: u8, x: u32, y: u32, i: u32, j: u32) -> (f64, f64) {
        let (z, x, y, tx, ty) = match self.upsampled {
            None => (z, x, y, self.x.pixel_fraction(i), self.y.pixel_fraction(j)),
            Some(u) => {
                // Same arithmetic as `resample::upsample_subregion`, whose
                // off-grid neighbours clamp to the parent's edge.
                let scale = 1.0 / u.factor as f64;
                let off_x = u.sub_x as f64 * u.tile_size as f64 * scale;
                let off_y = u.sub_y as f64 * u.tile_size as f64 * scale;
                let dz = u.factor.trailing_zeros() as u8;
                let fi = off_x + (i as f64 + 0.5) * scale - 0.5;
                let fj = off_y + (j as f64 + 0.5) * scale - 0.5;
                (
                    z.saturating_sub(dz),
                    x.checked_div(u.factor).unwrap_or(0),
                    y.checked_div(u.factor).unwrap_or(0),
                    self.x.blended_fraction(fi, u.tile_size),
                    self.y.blended_fraction(fj, u.tile_size),
                )
            }
        };
        // `1u64 << z` as f64 is exactly 2^z; for z ≥ 64 (malformed) use the
        // same power of two in floating point instead of overflowing the shift.
        let n = match 1u64.checked_shl(z as u32) {
            Some(v) => v as f64,
            None => 2f64.powi(z as i32),
        };
        match self.space {
            TileSpace::Mercator => {
                let lon = ((x as f64 + tx) / n) * 360.0 - 180.0;
                (lon, mercator_lat(y as f64 + ty, n))
            }
            TileSpace::LinearLatLon => {
                // Same expressions as the server's
                // `cog::resample::resample_to_tile` over
                // `cog_dem::mercator_xyz_to_bounds`. `x as f64 + 1.0` is
                // `(x + 1) as f64` exactly (both are integers below 2^53)
                // without overflowing at `u32::MAX`.
                let west = (x as f64 / n) * 360.0 - 180.0;
                let east = ((x as f64 + 1.0) / n) * 360.0 - 180.0;
                let north = mercator_lat(y as f64, n);
                let south = mercator_lat(y as f64 + 1.0, n);
                (west + tx * (east - west), north - ty * (north - south))
            }
        }
    }
}

/// Latitude (degrees) of Web Mercator tile row coordinate `ty` (fractional
/// tile index from the north edge) at a zoom with `n = 2^z` tiles per axis.
///
/// This is the one transcendental step of [`PixelPositions::lonlat`]
/// (`sinh`, `atan`): native and WebAssembly builds may differ here in the
/// last ulp, everything else in this crate is `+ − × ÷ floor round`.
pub fn mercator_lat(ty: f64, n: f64) -> f64 {
    (std::f64::consts::PI * (1.0 - 2.0 * ty / n))
        .sinh()
        .atan()
        .to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resample::{resample_bilinear, upsample_subregion};

    /// Bilinear interpolation reproduces a linear function exactly, and edge
    /// clamping returns the edge sample, so feeding the resamplers a grid
    /// whose value *is* each pixel's tile fraction must give back exactly the
    /// fractions `PixelPositions` records.
    fn ramp(n: u32, f: impl Fn(u32) -> f64) -> Vec<f64> {
        (0..n * n).map(|k| f(k % n)).collect()
    }

    #[test]
    fn resampled_positions_match_resample_bilinear() {
        for (src, dst) in [(512u32, 256u32), (256, 512), (64, 100), (300, 7)] {
            let grid = ramp(src, |c| (c as f64 + 0.5) / src as f64);
            let out = resample_bilinear(&grid, src, src, dst, dst);
            let p = PixelPositions::resampled(src, src, dst);
            for i in 0..dst {
                let got = out[i as usize];
                let want = p.x.pixel_fraction(i);
                assert!(
                    (got - want).abs() < 1e-12,
                    "{src}->{dst} px {i}: {got} vs {want}"
                );
            }
        }
    }

    #[test]
    fn upsampled_positions_match_upsample_subregion() {
        // A 512-px parent resized to 256, then upsampled by 4 (sub-tiles at
        // the parent's edges included, where neighbours clamp).
        let (src, n, factor) = (512u32, 256u32, 4u32);
        let parent_pos = PixelPositions::resampled(src, src, n);
        let parent = ramp(n, |c| parent_pos.x.pixel_fraction(c));
        for sub in [0u32, 1, 3] {
            let child = upsample_subregion(&parent, n, factor, sub, 0);
            let pos = PixelPositions {
                upsampled: Some(Upsampled {
                    factor,
                    sub_x: sub,
                    sub_y: 0,
                    tile_size: n,
                }),
                ..parent_pos
            };
            for i in 0..n {
                // Parent tile fraction of child pixel i (lonlat works in the
                // parent tile; recover the fraction from the longitude).
                let (lon, _) = pos.lonlat(10, 4 * 3 + sub, 0, i, 0);
                let parent_frac = (lon + 180.0) / 360.0 * 2f64.powi(8) - 3.0;
                let got = child[i as usize];
                assert!(
                    (got - parent_frac).abs() < 1e-9,
                    "sub {sub} px {i}: value {got} vs recorded {parent_frac}"
                );
            }
        }
    }

    /// Malformed positions give odd numbers, never a panic.
    #[test]
    fn malformed_positions_do_not_panic() {
        let weird = [
            AxisPositions::Centres { n: 0 },
            AxisPositions::Resampled { src: 0, dst: 0 },
            AxisPositions::Resampled { src: 0, dst: 5 },
            AxisPositions::Resampled {
                src: u32::MAX,
                dst: 1,
            },
        ];
        for x in weird {
            for y in weird {
                for space in [TileSpace::Mercator, TileSpace::LinearLatLon] {
                    for upsampled in [
                        None,
                        Some(Upsampled {
                            factor: 0,
                            sub_x: 9,
                            sub_y: u32::MAX,
                            tile_size: 0,
                        }),
                        Some(Upsampled {
                            factor: 1 << 31,
                            sub_x: 0,
                            sub_y: 0,
                            tile_size: u32::MAX,
                        }),
                    ] {
                        let p = PixelPositions {
                            space,
                            x,
                            y,
                            upsampled,
                        };
                        for z in [0u8, 20, 63, 64, 255] {
                            let _ = p.lonlat(z, u32::MAX, u32::MAX, u32::MAX, 0);
                            let _ = x.blended_fraction(f64::NAN, 0);
                            let _ = x.blended_fraction(f64::INFINITY, u32::MAX);
                            let _ = x.blended_fraction(f64::NEG_INFINITY, 3);
                        }
                    }
                }
            }
        }
    }
}
