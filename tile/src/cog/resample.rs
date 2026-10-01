//! Tile resampling utilities for COG processing.

use super::bounds::{TileBounds, geo_to_pixel_x, geo_to_pixel_y};

/// Source pixels fetched beyond the requested bounds on every side, so that
/// bilinear interpolation near the window edge has its outer neighbour.
pub(crate) const INTERPOLATION_MARGIN_PX: f64 = 1.0;

/// Tile coordinate range for reading COG tiles.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TileRange {
    pub x_start: usize,
    pub x_end: usize,
    pub y_start: usize,
    pub y_end: usize,
}

impl TileRange {
    /// Calculate tile range from geographic bounds and COG parameters.
    ///
    /// The window is widened by [`INTERPOLATION_MARGIN_PX`] source pixels on
    /// every side: a sample half a pixel inside the requested bounds reads a
    /// neighbour whose centre lies outside them, and that neighbour may sit in
    /// the next chunk.
    pub fn from_bounds(
        bounds: &TileBounds,
        cog_bounds: &TileBounds,
        img_width: u32,
        img_height: u32,
        cog_tile_w: u32,
        cog_tile_h: u32,
        tile_count: (usize, usize),
    ) -> Self {
        // Edge-based pixel coordinates (0 = west / north edge of pixel 0).
        let m = INTERPOLATION_MARGIN_PX;
        let px_west = geo_to_pixel_x(bounds.west, cog_bounds, img_width) - m;
        let px_east = geo_to_pixel_x(bounds.east, cog_bounds, img_width) + m;
        let px_north = geo_to_pixel_y(bounds.north, cog_bounds, img_height) - m;
        let px_south = geo_to_pixel_y(bounds.south, cog_bounds, img_height) + m;

        let (tile_count_x, tile_count_y) = tile_count;

        let x_start = (px_west / cog_tile_w as f64)
            .floor()
            .max(0.0)
            .min(tile_count_x as f64) as usize;
        let x_end = (px_east / cog_tile_w as f64)
            .ceil()
            .max(0.0)
            .min(tile_count_x as f64) as usize;
        let y_start = (px_north / cog_tile_h as f64)
            .floor()
            .max(0.0)
            .min(tile_count_y as f64) as usize;
        let y_end = (px_south / cog_tile_h as f64)
            .ceil()
            .max(0.0)
            .min(tile_count_y as f64) as usize;

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

    for out_y in 0..tile_size {
        for out_x in 0..tile_size {
            // Convert output pixel to geo coordinate (center of pixel)
            let geo_x =
                bounds.west + (out_x as f64 + 0.5) / tile_size as f64 * (bounds.east - bounds.west);
            let geo_y = bounds.north
                - (out_y as f64 + 0.5) / tile_size as f64 * (bounds.north - bounds.south);

            // Convert to COG pixel coordinate
            let px_x = geo_to_pixel_x(geo_x, cog_bounds, img_width);
            let px_y = geo_to_pixel_y(geo_y, cog_bounds, img_height);

            // Edge-based pixel coordinate -> centre-based buffer coordinate.
            let buf_x = px_x - buffer_origin.0 - 0.5;
            let buf_y = px_y - buffer_origin.1 - 0.5;

            output.push(interpolate(buf_x, buf_y));
        }
    }

    output
}
