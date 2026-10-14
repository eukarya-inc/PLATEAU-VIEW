//! Composite tile source implementation.

use async_trait::async_trait;
use futures::stream::{self, StreamExt, TryStreamExt};
use image::{RgbaImage, imageops};

use super::source::{TileError, TileSource};

/// How many layers are read at once for one tile. A mosaic can hold tens of
/// thousands of COGs; reading them one by one took minutes per tile.
const MAX_CONCURRENT_FETCHES: usize = 16;

/// How many layers preload their bounds at once at startup. Bounds reads are
/// small; this only stops a 30k-COG mosaic opening 30k connections at once.
const MAX_CONCURRENT_PRELOADS: usize = 256;

/// Composite tile source that overlays multiple sources.
pub struct CompositeTileSource {
    /// Base layer (bottom)
    base: Option<std::sync::Arc<dyn TileSource>>,
    /// Overlay layers (sorted by order, lower index = bottom)
    overlays: Vec<std::sync::Arc<dyn TileSource>>,
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

    pub fn with_base(mut self, base: std::sync::Arc<dyn TileSource>) -> Self {
        self.base = Some(base);
        self
    }

    pub fn with_overlay(mut self, overlay: std::sync::Arc<dyn TileSource>) -> Self {
        self.overlays.push(overlay);
        self
    }

    pub fn with_overlays(mut self, overlays: Vec<std::sync::Arc<dyn TileSource>>) -> Self {
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
        // Preload all sources with bounded concurrency; first error wins.
        // Index-driven so the closures take a plain `usize` (an `Arc<dyn …>`
        // argument trips async_trait's higher-ranked lifetime check).
        let sources: Vec<_> = self.base.iter().chain(&self.overlays).collect();
        stream::iter((0..sources.len()).map(|i| {
            let s = sources[i].clone();
            async move { s.preload().await }
        }))
        .buffer_unordered(MAX_CONCURRENT_PRELOADS)
        .try_collect::<()>()
        .await
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

        // Apply overlays. Skip those whose cached bounds miss the tile without
        // any I/O, then read the rest a few at a time; `buffered` yields in
        // input order, so paint order is kept.
        // ponytail: linear bounds scan per tile (microseconds even at 30k
        // COGs); an R-tree like the terrain composite's if it ever shows up.
        let hits: Vec<usize> = (0..self.overlays.len())
            .filter(|&i| self.overlays[i].covers(z, x, y))
            .collect();
        let mut fetches = stream::iter(hits.into_iter().map(|i| {
            let o = self.overlays[i].clone();
            async move { (i, o.get_tile(z, x, y).await) }
        }))
        .buffered(MAX_CONCURRENT_FETCHES);

        let mut overlay_count = 0;
        while let Some((i, fetched)) = fetches.next().await {
            if let Some(overlay_img) = fetched? {
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

    fn etag_keys(&self, z: u32, x: u32, y: u32) -> Vec<String> {
        let mut keys = Vec::new();

        // Collect keys from base layer
        if let Some(base) = &self.base {
            keys.extend(base.etag_keys(z, x, y));
        }

        // Collect keys from overlays
        for overlay in &self.overlays {
            keys.extend(overlay.etag_keys(z, x, y));
        }

        keys
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

    /// Solid-colour layer that records whether it was read.
    struct Solid {
        rgba: [u8; 4],
        covers: bool,
        read: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl TileSource for Solid {
        async fn get_tile(&self, _: u32, _: u32, _: u32) -> Result<Option<RgbaImage>, TileError> {
            self.read.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(Some(RgbaImage::from_pixel(
                256,
                256,
                image::Rgba(self.rgba),
            )))
        }
        fn covers(&self, _: u32, _: u32, _: u32) -> bool {
            self.covers
        }
        fn etag_keys(&self, _: u32, _: u32, _: u32) -> Vec<String> {
            vec![]
        }
    }

    fn solid(rgba: [u8; 4], covers: bool) -> std::sync::Arc<Solid> {
        std::sync::Arc::new(Solid {
            rgba,
            covers,
            read: Default::default(),
        })
    }

    /// Overlays outside the tile are never read, and the rest still paint in
    /// config order even though they are fetched concurrently.
    #[tokio::test]
    async fn skips_uncovered_overlays_and_keeps_paint_order() {
        let far = solid([0, 0, 255, 255], false);
        let mut overlays: Vec<std::sync::Arc<dyn TileSource>> = vec![far.clone()];
        for i in 0..40u8 {
            overlays.push(solid([i, 0, 0, 255], true));
        }
        overlays.push(solid([0, 255, 0, 255], true));
        let comp = CompositeTileSource::new().with_overlays(overlays);

        let img = comp.get_tile(1, 0, 0).await.unwrap().unwrap();
        assert_eq!(img.get_pixel(0, 0).0, [0, 255, 0, 255]);
        assert!(!far.read.load(std::sync::atomic::Ordering::SeqCst));
    }
}
