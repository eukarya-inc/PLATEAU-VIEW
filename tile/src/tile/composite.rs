//! Composite tile source implementation.

use async_trait::async_trait;
use futures::future::join_all;
use image::{RgbaImage, imageops};

use super::source::{TileError, TileSource};

/// Composite tile source that overlays multiple sources.
pub struct CompositeTileSource {
    /// Base layer (bottom)
    base: Option<Box<dyn TileSource>>,
    /// Overlay layers (sorted by order, lower index = bottom)
    overlays: Vec<Box<dyn TileSource>>,
    /// Tile size
    tile_size: u32,
}

impl CompositeTileSource {
    pub fn new() -> Self {
        Self {
            base: None,
            overlays: Vec::new(),
            tile_size: 256,
        }
    }

    pub fn with_base(mut self, base: Box<dyn TileSource>) -> Self {
        self.base = Some(base);
        self
    }

    pub fn with_overlay(mut self, overlay: Box<dyn TileSource>) -> Self {
        self.overlays.push(overlay);
        self
    }

    pub fn with_overlays(mut self, overlays: Vec<Box<dyn TileSource>>) -> Self {
        self.overlays.extend(overlays);
        self
    }

    pub fn with_tile_size(mut self, tile_size: u32) -> Self {
        self.tile_size = tile_size;
        self
    }
}

impl Default for CompositeTileSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TileSource for CompositeTileSource {
    async fn preload(&self) -> Result<(), TileError> {
        // Preload all sources in parallel
        let mut futures: Vec<_> = self.overlays.iter().map(|o| o.preload()).collect();

        if let Some(base) = &self.base {
            futures.push(base.preload());
        }

        let results = join_all(futures).await;

        // Return first error if any
        for result in results {
            result?;
        }

        Ok(())
    }

    async fn get_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<RgbaImage>, TileError> {
        // Start with base layer
        let mut result: Option<RgbaImage> = match &self.base {
            Some(base) => {
                let base_tile = base.get_tile(z, x, y).await?;
                tracing::debug!(
                    z = z,
                    x = x,
                    y = y,
                    has_base = base_tile.is_some(),
                    "Base layer fetched"
                );
                base_tile
            }
            None => None,
        };

        // Apply overlays
        let mut overlay_count = 0;
        for (i, overlay) in self.overlays.iter().enumerate() {
            if let Some(overlay_img) = overlay.get_tile(z, x, y).await? {
                tracing::debug!(
                    z = z,
                    x = x,
                    y = y,
                    overlay_index = i,
                    "Overlay layer fetched"
                );
                result = Some(composite_images(result, overlay_img, self.tile_size));
                overlay_count += 1;
            }
        }

        tracing::debug!(
            z = z,
            x = x,
            y = y,
            overlay_count = overlay_count,
            has_result = result.is_some(),
            "Composite tile generated"
        );

        Ok(result)
    }

    fn covers(&self, z: u32, x: u32, y: u32) -> bool {
        // Check if any layer covers this tile
        let base_covers = self
            .base
            .as_ref()
            .map(|b| b.covers(z, x, y))
            .unwrap_or(false);
        let overlay_covers = self.overlays.iter().any(|o| o.covers(z, x, y));
        base_covers || overlay_covers
    }
}

/// Composite two images, placing overlay on top of base.
fn composite_images(base: Option<RgbaImage>, overlay: RgbaImage, tile_size: u32) -> RgbaImage {
    match base {
        Some(mut base_img) => {
            // Ensure base is the correct size
            if base_img.width() != tile_size || base_img.height() != tile_size {
                base_img = imageops::resize(
                    &base_img,
                    tile_size,
                    tile_size,
                    imageops::FilterType::Lanczos3,
                );
            }

            // Resize overlay if needed
            let overlay = if overlay.width() != tile_size || overlay.height() != tile_size {
                imageops::resize(
                    &overlay,
                    tile_size,
                    tile_size,
                    imageops::FilterType::Lanczos3,
                )
            } else {
                overlay
            };

            // Overlay using alpha blending
            imageops::overlay(&mut base_img, &overlay, 0, 0);
            base_img
        }
        None => {
            // No base, just return overlay (resized if needed)
            if overlay.width() != tile_size || overlay.height() != tile_size {
                imageops::resize(
                    &overlay,
                    tile_size,
                    tile_size,
                    imageops::FilterType::Lanczos3,
                )
            } else {
                overlay
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_images() {
        // Create a red base
        let mut base = RgbaImage::new(256, 256);
        for pixel in base.pixels_mut() {
            *pixel = image::Rgba([255, 0, 0, 255]);
        }

        // Create a semi-transparent green overlay
        let mut overlay = RgbaImage::new(256, 256);
        for pixel in overlay.pixels_mut() {
            *pixel = image::Rgba([0, 255, 0, 128]);
        }

        let result = composite_images(Some(base), overlay, 256);

        // Result should be a blend of red and green
        let pixel = result.get_pixel(128, 128);
        assert!(pixel.0[0] > 0); // Some red
        assert!(pixel.0[1] > 0); // Some green
        assert!(pixel.0[3] >= 254); // Nearly fully opaque (allow for rounding)
    }

    #[test]
    fn test_composite_no_base() {
        let mut overlay = RgbaImage::new(256, 256);
        for pixel in overlay.pixels_mut() {
            *pixel = image::Rgba([0, 255, 0, 255]);
        }

        let result = composite_images(None, overlay, 256);
        let pixel = result.get_pixel(128, 128);
        assert_eq!(pixel.0, [0, 255, 0, 255]);
    }
}
