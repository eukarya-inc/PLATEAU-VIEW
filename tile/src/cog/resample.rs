//! Tile resampling utilities for COG processing.

use super::bounds::{TileBounds, geo_to_pixel_x, geo_to_pixel_y};

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
    pub fn from_bounds(
        bounds: &TileBounds,
        cog_bounds: &TileBounds,
        img_width: u32,
        img_height: u32,
        cog_tile_w: u32,
        cog_tile_h: u32,
        tile_count: (usize, usize),
    ) -> Self {
        let px_west = geo_to_pixel_x(bounds.west, cog_bounds, img_width);
        let px_east = geo_to_pixel_x(bounds.east, cog_bounds, img_width);
        let px_north = geo_to_pixel_y(bounds.north, cog_bounds, img_height);
        let px_south = geo_to_pixel_y(bounds.south, cog_bounds, img_height);

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

    /// Calculate buffer dimensions for this tile range.
    pub fn buffer_size(&self, cog_tile_w: u32, cog_tile_h: u32) -> (usize, usize) {
        let width = (self.x_end - self.x_start) * cog_tile_w as usize;
        let height = (self.y_end - self.y_start) * cog_tile_h as usize;
        (width, height)
    }
}

/// Resample a buffer to output tile using bilinear interpolation.
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

            // Convert to buffer coordinate
            let buf_x = px_x - buffer_origin.0;
            let buf_y = px_y - buffer_origin.1;

            output.push(interpolate(buf_x, buf_y));
        }
    }

    output
}
