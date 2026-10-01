//! DEM (digital elevation model) source abstraction.
//!
//! Providers return Web Mercator XYZ tiles as f64 elevation grids. NaN marks
//! no-data pixels. An optional per-tile ETag fragment is threaded through so
//! that downstream cache keys can track upstream cache-busting without a
//! manual version bump.

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DemError {
    #[error("dem tile not found")]
    NotFound,
    #[error("http error: {0}")]
    Http(String),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("out of range")]
    OutOfRange,
}

impl From<reqwest::Error> for DemError {
    fn from(e: reqwest::Error) -> Self {
        DemError::Http(e.to_string())
    }
}

impl From<image::ImageError> for DemError {
    fn from(e: image::ImageError) -> Self {
        DemError::Decode(e.to_string())
    }
}

/// Result of fetching a single XYZ DEM tile.
#[derive(Debug, Clone)]
pub struct DemTile {
    /// Row-major, top-to-bottom (north first). Length = tile_size * tile_size.
    pub elevations: Vec<f64>,
    /// Opaque ETag fragment captured from the upstream source, if any.
    pub etag: Option<String>,
    /// Where each element of `elevations` was evaluated. `None` means the
    /// XYZ convention: pixel `(i, j)` of an `n × n` tile is the Web Mercator
    /// pixel centre `((i + 0.5) / n, (j + 0.5) / n)` of the requested tile.
    ///
    /// Only the height correction reads this (it must sample ΔH at the same
    /// points as the layer's elevations); nothing else depends on it.
    pub positions: Option<PixelPositions>,
}

/// How a provider maps an index along one tile axis to a fraction of the tile
/// (0 = west / north edge, 1 = east / south edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPositions {
    /// `(i + 0.5) / n` — pixel centres of an `n`-pixel tile.
    Centres { n: u32 },
    /// What [`super::resample_bilinear`] produces when it resizes a `src`-pixel
    /// tile to `dst` pixels: output `i` is source pixel `i·(src−1)/(dst−1)`,
    /// whose centre is at `(that + 0.5) / src`.
    CornerAligned { src: u32, dst: u32 },
}

impl AxisPositions {
    /// Tile fraction of (possibly fractional) index `i`. Bilinear resampling
    /// is linear in the index, so a fractional index maps linearly too.
    pub fn fraction(self, i: f64) -> f64 {
        match self {
            Self::Centres { n } => (i + 0.5) / n as f64,
            Self::CornerAligned { src, dst } => {
                let s = i * (src - 1) as f64 / (dst - 1).max(1) as f64;
                (s + 0.5) / src as f64
            }
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
    /// edges — how [`super::CogDemSource`] samples a geographic (EPSG:4326 /
    /// 6668) COG for an XYZ request.
    LinearLatLon,
}

/// Exact sample positions of a [`DemTile`]'s elevations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelPositions {
    pub space: TileSpace,
    pub x: AxisPositions,
    pub y: AxisPositions,
    /// Set when the tile was bilinear-upsampled from an ancestor (see
    /// `CompositeDemProvider::fetch_base_upsampled`): the axes above then
    /// describe the *ancestor's* grid and this records how index `i` of the
    /// served tile maps onto it.
    pub upsampled: Option<Upsampled>,
}

/// Child-in-parent mapping of `composite::upsample_subregion`.
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
    /// [`super::resample_bilinear`] (identity when the sizes match).
    pub fn resampled(src_w: u32, src_h: u32, dst: u32) -> Self {
        if src_w == dst && src_h == dst {
            return Self::centres(dst);
        }
        Self {
            space: TileSpace::Mercator,
            x: AxisPositions::CornerAligned { src: src_w, dst },
            y: AxisPositions::CornerAligned { src: src_h, dst },
            upsampled: None,
        }
    }

    /// Longitude / latitude (degrees) at which element `(i, j)` (column, row)
    /// of tile `z/x/y` was evaluated.
    pub fn lonlat(&self, z: u8, x: u32, y: u32, i: u32, j: u32) -> (f64, f64) {
        let (z, x, y, fi, fj) = match self.upsampled {
            None => (z, x, y, i as f64, j as f64),
            Some(u) => {
                // Same arithmetic as `upsample_subregion`.
                let scale = 1.0 / u.factor as f64;
                let off_x = u.sub_x as f64 * u.tile_size as f64 * scale;
                let off_y = u.sub_y as f64 * u.tile_size as f64 * scale;
                let dz = u.factor.trailing_zeros() as u8;
                (
                    z - dz,
                    x / u.factor,
                    y / u.factor,
                    off_x + (i as f64 + 0.5) * scale - 0.5,
                    off_y + (j as f64 + 0.5) * scale - 0.5,
                )
            }
        };
        let tx = self.x.fraction(fi);
        let ty = self.y.fraction(fj);
        let n = (1u64 << z) as f64;
        match self.space {
            TileSpace::Mercator => {
                let lon = ((x as f64 + tx) / n) * 360.0 - 180.0;
                (lon, mercator_lat(y as f64 + ty, n))
            }
            TileSpace::LinearLatLon => {
                // Same expressions as `cog::resample::resample_to_tile` over
                // `cog_dem::mercator_xyz_to_bounds`.
                let west = (x as f64 / n) * 360.0 - 180.0;
                let east = ((x + 1) as f64 / n) * 360.0 - 180.0;
                let north = mercator_lat(y as f64, n);
                let south = mercator_lat((y + 1) as f64, n);
                (west + tx * (east - west), north - ty * (north - south))
            }
        }
    }
}

fn mercator_lat(ty: f64, n: f64) -> f64 {
    (std::f64::consts::PI * (1.0 - 2.0 * ty / n))
        .sinh()
        .atan()
        .to_degrees()
}

/// Geographic coverage of a DEM source, in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

impl GeoBounds {
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// Returns true if `self` and `other` share any area (touching counts).
    pub fn intersects(&self, other: &GeoBounds) -> bool {
        !(self.east < other.west
            || self.west > other.east
            || self.north < other.south
            || self.south > other.north)
    }
}

#[async_trait]
pub trait DemProvider: Send + Sync {
    /// Fetch elevations for a Web Mercator XYZ tile.
    async fn get_tile_elevations(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError>;

    /// Native tile size (pixels) served by the upstream.
    fn native_tile_size(&self) -> u32;

    /// Maximum zoom served.
    fn max_zoom(&self) -> u8;

    /// Stable version identifier for this provider (manual bump).
    fn version(&self) -> &str;

    /// Stable slug used in cache keys / etags.
    fn slug(&self) -> &str;

    /// Optional one-shot startup hook. Implementations that need to read
    /// remote metadata (PMTiles header, GeoTIFF IFD, etc.) should do so here
    /// so that `bounds()` is populated before the first request. The default
    /// is a no-op for sources without metadata to fetch.
    async fn preload(&self) -> Result<(), DemError> {
        Ok(())
    }

    /// Geographic coverage in degrees (west, south, east, north).
    /// `None` means global / unknown — the composite treats such overlays
    /// as "always intersects" and skips R-tree pruning for them. Should be
    /// stable after `preload()`.
    fn bounds(&self) -> Option<GeoBounds> {
        None
    }
}
