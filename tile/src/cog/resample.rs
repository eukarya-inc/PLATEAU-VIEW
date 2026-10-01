//! Tile resampling utilities for COG processing.

use super::bounds::{TileBounds, geo_to_pixel_x, geo_to_pixel_y};
use super::interpolate::{snap, support};

/// Tile coordinate range for reading COG tiles.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TileRange {
    pub x_start: usize,
    pub x_end: usize,
    pub y_start: usize,
    pub y_end: usize,
}

impl TileRange {
    /// Chunks holding every source pixel the resampler will read for a
    /// `tile_size × tile_size` request over `bounds`.
    ///
    /// The window is derived from the first and last output sample centres —
    /// the same positions [`resample_to_tile`] evaluates — and their non-zero
    /// bilinear support ([`support`]). The mapping is linear and monotonic in
    /// each axis (x grows east, y grows south as rows go down), so the two
    /// extreme samples bound every other. An aligned 1:1 request therefore
    /// reads exactly its own chunks, while a sample whose support straddles
    /// a chunk boundary still gets the neighbouring chunk.
    #[allow(clippy::too_many_arguments)]
    pub fn from_bounds(
        bounds: &TileBounds,
        cog_bounds: &TileBounds,
        img_width: u32,
        img_height: u32,
        cog_tile_w: u32,
        cog_tile_h: u32,
        tile_count: (usize, usize),
        tile_size: u32,
    ) -> Self {
        let last = tile_size.saturating_sub(1);
        let xs = [
            sample_x(bounds, cog_bounds, img_width, tile_size, 0),
            sample_x(bounds, cog_bounds, img_width, tile_size, last),
        ];
        let ys = [
            sample_y(bounds, cog_bounds, img_height, tile_size, 0),
            sample_y(bounds, cog_bounds, img_height, tile_size, last),
        ];
        let (x_start, x_end) = chunk_span(xs, img_width as usize, cog_tile_w, tile_count.0);
        let (y_start, y_end) = chunk_span(ys, img_height as usize, cog_tile_h, tile_count.1);
        Self {
            x_start,
            x_end,
            y_start,
            y_end,
        }
    }

    /// Check if the tile range is empty (no intersection).
    pub fn is_empty(&self) -> bool {
        self.x_end <= self.x_start || self.y_end <= self.y_start
    }

    /// Width / height of the part of the buffer that holds real image pixels
    /// (edge chunks are padded past the image's east / south edge).
    pub fn valid_size(
        &self,
        cog_tile_w: u32,
        cog_tile_h: u32,
        img_width: u32,
        img_height: u32,
    ) -> (usize, usize) {
        let (bw, bh) = self.buffer_size(cog_tile_w, cog_tile_h);
        let ox = self.x_start * cog_tile_w as usize;
        let oy = self.y_start * cog_tile_h as usize;
        (
            bw.min((img_width as usize).saturating_sub(ox)),
            bh.min((img_height as usize).saturating_sub(oy)),
        )
    }

    /// Calculate buffer dimensions for this tile range.
    pub fn buffer_size(&self, cog_tile_w: u32, cog_tile_h: u32) -> (usize, usize) {
        let width = (self.x_end - self.x_start) * cog_tile_w as usize;
        let height = (self.y_end - self.y_start) * cog_tile_h as usize;
        (width, height)
    }
}

/// Resample a buffer to output tile using bilinear interpolation.
///
/// Output pixel centres are mapped to **centre-based** buffer coordinates
/// (`(k, m)` = centre of buffer pixel `(k, m)`) and handed to `interpolate`.
/// `geo_to_pixel_*` is edge-based (0 = the west / north edge of pixel 0, the
/// convention [`TileRange`] uses for chunk selection), so the half pixel is
/// subtracted here — the one place that feeds the interpolators. Before this
/// was done every COG read came back half a source pixel north-west of where
/// it belongs.
pub(crate) fn resample_to_tile<T, F>(
    bounds: &TileBounds,
    cog_bounds: &TileBounds,
    img_width: u32,
    img_height: u32,
    tile_size: u32,
    buffer_origin: (f64, f64),
    interpolate: F,
) -> Vec<T>
where
    F: Fn(f64, f64) -> T,
{
    let mut output = Vec::with_capacity((tile_size * tile_size) as usize);
    // Integral origins: subtracting them from a snapped coordinate is exact,
    // so the buffer coordinate has the same fraction the window was cut for.
    let xs: Vec<f64> = (0..tile_size)
        .map(|i| sample_x(bounds, cog_bounds, img_width, tile_size, i) - buffer_origin.0)
        .collect();
    for out_y in 0..tile_size {
        let buf_y = sample_y(bounds, cog_bounds, img_height, tile_size, out_y) - buffer_origin.1;
        for &buf_x in &xs {
            output.push(interpolate(buf_x, buf_y));
        }
    }
    output
}

/// Centre-based, snapped source-pixel x coordinate of output column `out_x`'s
/// centre. `geo_to_pixel_x` is edge-based (0 = west edge of pixel 0), hence
/// the half pixel.
fn sample_x(b: &TileBounds, cog: &TileBounds, img_width: u32, tile_size: u32, out_x: u32) -> f64 {
    let geo_x = b.west + (out_x as f64 + 0.5) / tile_size as f64 * (b.east - b.west);
    snap(geo_to_pixel_x(geo_x, cog, img_width) - 0.5)
}

/// Same for output row `out_y` (rows run north → south).
fn sample_y(b: &TileBounds, cog: &TileBounds, img_height: u32, tile_size: u32, out_y: u32) -> f64 {
    let geo_y = b.north - (out_y as f64 + 0.5) / tile_size as f64 * (b.north - b.south);
    snap(geo_to_pixel_y(geo_y, cog, img_height) - 0.5)
}

/// Chunk index range `[start, end)` covering the support of the two extreme
/// samples along one axis; empty when both lie outside the raster on the
/// same side.
fn chunk_span(c: [f64; 2], n: usize, chunk: u32, chunks: usize) -> (usize, usize) {
    let (lo, hi) = (c[0].min(c[1]), c[0].max(c[1]));
    if n == 0 || hi < -0.5 || lo >= n as f64 - 0.5 {
        return (0, 0);
    }
    // Clamp into the raster first: a sample in the outer half of an edge
    // pixel reads that pixel.
    let first = support(lo.max(-0.5), n).map(|s| s.0).unwrap_or(0);
    let last = support(hi.min(n as f64 - 0.5 - 1e-6), n)
        .map(|s| s.1)
        .unwrap_or(n - 1);
    let chunk = chunk as usize;
    ((first / chunk).min(chunks), (last / chunk + 1).min(chunks))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4096×4096 raster of 1-unit pixels in 512-px chunks, origin (0, 4096).
    fn cog() -> TileBounds {
        TileBounds {
            west: 0.0,
            east: 4096.0,
            north: 4096.0,
            south: 0.0,
        }
    }

    fn range(west: f64, north: f64, size: f64, tile_size: u32) -> TileRange {
        let b = TileBounds {
            west,
            east: west + size,
            north: 4096.0 - north,
            south: 4096.0 - north - size,
        };
        TileRange::from_bounds(&b, &cog(), 4096, 4096, 512, 512, (8, 8), tile_size)
    }

    fn chunks(r: &TileRange) -> usize {
        (r.x_end - r.x_start) * (r.y_end - r.y_start)
    }

    #[test]
    fn aligned_requests_read_only_their_own_chunks() {
        // 256 px 1:1 over columns/rows [512, 768): one chunk, (1, 1).
        let r = range(512.0, 512.0, 256.0, 256);
        assert_eq!((r.x_start, r.x_end, r.y_start, r.y_end), (1, 2, 1, 2));
        assert_eq!(chunks(&r), 1);
        // One full aligned chunk, 1:1.
        assert_eq!(chunks(&range(512.0, 1024.0, 512.0, 512)), 1);
        // Aligned 2:1 downsample over chunk 1 (samples on pixel edges).
        assert_eq!(chunks(&range(512.0, 512.0, 512.0, 256)), 1);
    }

    #[test]
    fn straddling_requests_read_the_neighbour_chunk() {
        // Crosses column 512.
        let r = range(384.0, 0.0, 256.0, 256);
        assert_eq!((r.x_start, r.x_end, r.y_start, r.y_end), (0, 2, 0, 1));
        // The 473120 seam: a sample on the edge between pixels 511 and 512
        // (centre-based 511.5) needs both, i.e. chunks 0 and 1. With 1
        // sample over [511, 513) the centre is that edge; with 2 samples the
        // centres are exactly pixels 511 and 512.
        let b = TileBounds {
            west: 511.0,
            east: 513.0,
            north: 4096.0,
            south: 4094.0,
        };
        let r = TileRange::from_bounds(&b, &cog(), 4096, 4096, 512, 512, (8, 8), 2);
        assert_eq!((r.x_start, r.x_end), (0, 2));
        let r = TileRange::from_bounds(&b, &cog(), 4096, 4096, 512, 512, (8, 8), 1);
        assert_eq!((r.x_start, r.x_end), (0, 2));
    }

    #[test]
    fn outside_and_edge_requests() {
        // Entirely west of the raster.
        assert!(range(-300.0, 0.0, 256.0, 256).is_empty());
        // Overhanging the north-west corner: clamped to chunk (0, 0).
        let r = range(-100.0, -100.0, 256.0, 256);
        assert_eq!((r.x_start, r.x_end, r.y_start, r.y_end), (0, 1, 0, 1));
        // Overhanging the south-east corner.
        let r = range(4000.0, 4000.0, 256.0, 256);
        assert_eq!((r.x_start, r.x_end, r.y_start, r.y_end), (7, 8, 7, 8));
    }

    /// Every pixel the interpolator reads lies inside the fetched window, for
    /// aligned, offset, up- and down-sampled requests.
    #[test]
    fn interpolator_never_reads_outside_the_window() {
        for (west, north, size, ts) in [
            (512.0, 512.0, 256.0, 256u32),
            (511.3, 700.7, 256.0, 256),
            (1000.25, 1500.75, 64.0, 256),
            (100.1, 3000.9, 1024.0, 256),
            (0.0, 0.0, 4096.0, 256),
            (4090.0, 4090.0, 6.0, 256),
        ] {
            let b = TileBounds {
                west,
                east: west + size,
                north: 4096.0 - north,
                south: 4096.0 - north - size,
            };
            let r = TileRange::from_bounds(&b, &cog(), 4096, 4096, 512, 512, (8, 8), ts);
            let (vw, vh) = r.valid_size(512, 512, 4096, 4096);
            let origin = ((r.x_start * 512) as f64, (r.y_start * 512) as f64);
            let (bw, bh) = r.buffer_size(512, 512);
            assert!(vw <= bw && vh <= bh);
            // The *unclamped* support of every sample must lie in the window:
            // only a sample in the raster's own outer half pixel may lean on
            // clamping. (Clamping to the window would otherwise hide a window
            // that is too small.)
            resample_to_tile(&b, &cog(), 4096, 4096, ts, origin, |x, y| {
                for (c, o, valid) in [(x, origin.0, vw), (y, origin.1, vh)] {
                    let c = snap(c);
                    let (f0, frac) = (c.floor(), c - c.floor());
                    let hi = f0 + if frac > 0.0 { 1.0 } else { 0.0 };
                    if c + o >= 0.0 {
                        assert!(f0 >= 0.0, "{west},{north},{ts}: {c} below window");
                    }
                    if c + o <= 4095.0 {
                        assert!(
                            hi <= (valid - 1) as f64,
                            "{west},{north},{ts}: {c} past window"
                        );
                    }
                }
            });
        }
    }
}
