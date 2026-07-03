//! WebP tile decoder for COGs.
//!
//! `async_tiff`'s [`DecoderRegistry::default`] ships decoders for the standard
//! TIFF compression methods (uncompressed, Deflate, LZW, JPEG, ZSTD) but not
//! WebP. GDAL writes WebP-compressed tiles with the private TIFF compression
//! tag `50001`, so ortho-imagery COGs built with `-co COMPRESS=WEBP` otherwise
//! fail to decode with an "unknown compression method" error.
//!
//! `async_tiff` preserves unrecognised compression codes as
//! [`CompressionMethod::Unknown`] rather than rejecting them, and
//! [`DecoderRegistry`] is explicitly user-extensible, so we can register a WebP
//! decoder here — backed by the (already-vendored) `image` crate — without
//! patching the upstream crate.

use async_tiff::decoder::{Decoder, DecoderRegistry};
use async_tiff::error::AsyncTiffResult;
use async_tiff::tags::{CompressionMethod, PhotometricInterpretation};
use bytes::Bytes;
use image::DynamicImage;

/// TIFF compression tag that GDAL/libtiff use for WebP-compressed tiles
/// (`COMPRESSION_WEBP`).
const COMPRESSION_WEBP: u16 = 50001;

/// A [`Decoder`] for WebP-compressed TIFF/COG tiles, backed by the `image` crate.
#[derive(Debug, Clone)]
pub struct WebpDecoder;

impl Decoder for WebpDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
    ) -> AsyncTiffResult<Bytes> {
        let img = image::load_from_memory_with_format(&buffer, image::ImageFormat::WebP)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Return interleaved samples matching the source channel count so the
        // caller's `samples_per_pixel`-based unpacking (see
        // [`crate::cog::decode::decode_rgba`]) lines up: RGB WebP -> 3 bytes/px,
        // RGBA WebP -> 4 bytes/px. Any other colour type is normalised to RGBA.
        let raw = match img {
            DynamicImage::ImageRgb8(b) => b.into_raw(),
            DynamicImage::ImageRgba8(b) => b.into_raw(),
            other => other.into_rgba8().into_raw(),
        };
        Ok(Bytes::from(raw))
    }
}

/// Build a [`DecoderRegistry`] with the `async_tiff` defaults plus WebP support.
///
/// Use this in place of [`DecoderRegistry::default`] so COGs compressed with
/// `-co COMPRESS=WEBP` (e.g. ortho imagery) decode correctly.
pub fn decoder_registry() -> DecoderRegistry {
    let mut registry = DecoderRegistry::default();
    registry.as_mut().insert(
        CompressionMethod::Unknown(COMPRESSION_WEBP),
        Box::new(WebpDecoder),
    );
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_tiff::tags::PhotometricInterpretation;
    use image::{ImageFormat, Rgb, RgbImage};
    use std::io::Cursor;

    #[test]
    fn decodes_webp_to_interleaved_rgb() {
        // Build a tiny image and encode it as (lossless) WebP in-memory.
        let mut img = RgbImage::new(2, 2);
        img.put_pixel(0, 0, Rgb([10, 20, 30]));
        img.put_pixel(1, 0, Rgb([40, 50, 60]));
        img.put_pixel(0, 1, Rgb([70, 80, 90]));
        img.put_pixel(1, 1, Rgb([100, 110, 120]));

        let mut webp = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(img.clone())
            .write_to(&mut webp, ImageFormat::WebP)
            .expect("encode webp");

        let decoded = WebpDecoder
            .decode_tile(
                Bytes::from(webp.into_inner()),
                PhotometricInterpretation::RGB,
                None,
            )
            .expect("decode webp");

        // Lossless round-trip: 2x2 RGB -> 12 interleaved bytes, values preserved.
        assert_eq!(decoded.len(), 2 * 2 * 3);
        assert_eq!(&decoded[..], img.as_raw().as_slice());
    }

    #[test]
    fn registry_has_webp_decoder() {
        let registry = decoder_registry();
        assert!(
            registry
                .as_ref()
                .contains_key(&CompressionMethod::Unknown(COMPRESSION_WEBP))
        );
    }
}
