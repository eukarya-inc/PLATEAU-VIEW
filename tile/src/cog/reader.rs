//! Cloud Optimized GeoTIFF reader.

use std::sync::Arc;

use async_tiff::{
    ImageFileDirectory, TIFF,
    metadata::{TiffMetadataReader, cache::ReadaheadMetadataCache},
    reader::ObjectReader,
    tags::SampleFormat,
};
use async_tiff::{TagValue, tags::Tag};
use object_store::{ObjectStore, path::Path as ObjectPath};

use super::{
    bounds::{CogCrs, TileBounds},
    decode::{decode_elevation, decode_rgba, get_pixel_values},
    error::CogError,
    interpolate::{bilinear_f64, bilinear_rgba},
    resample::{TileRange, resample_to_tile},
    webp::decoder_registry,
};
use crate::config::NoDataConfig;

/// Cloud Optimized GeoTIFF reader with HTTP range request support.
pub struct CogReader {
    tiff: TIFF,
    store: Arc<dyn ObjectStore>,
    path: ObjectPath,
    /// Bounds in the COG's native CRS units (degrees or Web Mercator meters).
    bounds: Option<TileBounds>,
    /// Coordinate reference system the bounds and pixel grid live in.
    crs: CogCrs,
    samples_per_pixel: u16,
}

impl CogReader {
    /// Open a COG file from an object store (HTTP, GCS, S3, local).
    /// This reads all IFDs (required for tile rendering with overview selection).
    pub async fn open(store: Arc<dyn ObjectStore>, path: ObjectPath) -> Result<Self, CogError> {
        let reader = ObjectReader::new(store.clone(), path.clone());
        let cached_reader = ReadaheadMetadataCache::new(reader);

        let mut metadata_reader = TiffMetadataReader::try_open(&cached_reader)
            .await
            .map_err(|e| CogError::OpenError(format!("{e:?}")))?;

        let tiff = metadata_reader
            .read(&cached_reader)
            .await
            .map_err(|e| CogError::OpenError(format!("{e:?}")))?;

        // Detect and validate CRS
        let crs = Self::detect_crs(&tiff)?;

        // Extract bounds from GeoTIFF metadata (in the COG's native CRS units)
        let bounds = Self::extract_bounds(&tiff);

        // Get samples per pixel
        let samples_per_pixel = tiff
            .ifds()
            .first()
            .map(|ifd| ifd.samples_per_pixel())
            .unwrap_or(1);

        // Log IFD info
        let ifds = tiff.ifds();
        tracing::info!("COG has {} IFD(s)", ifds.len());
        for (idx, ifd) in ifds.iter().enumerate() {
            tracing::debug!(
                "  IFD {}: {}x{}, samples_per_pixel={}, bits_per_sample={:?}, tile={}x{}",
                idx,
                ifd.image_width(),
                ifd.image_height(),
                ifd.samples_per_pixel(),
                ifd.bits_per_sample(),
                ifd.tile_width().unwrap_or(0),
                ifd.tile_height().unwrap_or(0)
            );
        }

        Ok(Self {
            tiff,
            store,
            path,
            bounds,
            crs,
            samples_per_pixel,
        })
    }

    /// Read only the first IFD to extract bounds (much faster than reading all IFDs).
    /// Use this for preloading bounds without reading the entire metadata.
    ///
    /// The returned bounds are always in **WGS84 degrees** (reprojected when the
    /// COG is Web Mercator), so callers can intersection-test against XYZ tiles.
    pub async fn read_bounds_only(
        store: Arc<dyn ObjectStore>,
        path: ObjectPath,
    ) -> Result<Option<TileBounds>, CogError> {
        let reader = ObjectReader::new(store, path);
        let cached_reader = ReadaheadMetadataCache::new(reader);

        let mut metadata_reader = TiffMetadataReader::try_open(&cached_reader)
            .await
            .map_err(|e| CogError::OpenError(format!("{e:?}")))?;

        // Read only the first IFD
        let first_ifd = metadata_reader
            .read_next_ifd(&cached_reader)
            .await
            .map_err(|e| CogError::OpenError(format!("{e:?}")))?;

        let Some(ifd) = first_ifd else {
            return Err(CogError::NoIfd);
        };

        // Detect and validate CRS
        let crs = Self::detect_crs_ifd(&ifd)?;

        // Extract bounds and normalize to WGS84 degrees for the caller.
        Ok(Self::extract_bounds_from_ifd(&ifd).map(|b| match crs {
            CogCrs::Geographic => b,
            CogCrs::WebMercator => b.mercator_to_wgs84(),
        }))
    }

    fn detect_crs(tiff: &TIFF) -> Result<CogCrs, CogError> {
        let ifd = tiff.ifds().first().ok_or(CogError::NoIfd)?;
        Self::detect_crs_ifd(ifd)
    }

    fn detect_crs_ifd(ifd: &ImageFileDirectory) -> Result<CogCrs, CogError> {
        if let Some(geo_keys) = ifd.geo_key_directory()
            && let Some(epsg) = geo_keys.epsg_code()
        {
            return CogCrs::from_epsg(epsg).ok_or(CogError::UnsupportedCrs(epsg));
        }

        // If no EPSG code found, assume WGS84
        tracing::warn!("No EPSG code found in GeoTIFF metadata, assuming WGS84");
        Ok(CogCrs::Geographic)
    }

    fn extract_bounds(tiff: &TIFF) -> Option<TileBounds> {
        let ifd = tiff.ifds().first()?;
        Self::extract_bounds_from_ifd(ifd)
    }

    fn extract_bounds_from_ifd(ifd: &ImageFileDirectory) -> Option<TileBounds> {
        let tiepoint = ifd.model_tiepoint()?;
        let pixel_scale = ifd.model_pixel_scale()?;

        let width = ifd.image_width() as f64;
        let height = ifd.image_height() as f64;

        if tiepoint.len() >= 6 && pixel_scale.len() >= 2 {
            let origin_x = tiepoint[3];
            let origin_y = tiepoint[4];
            let scale_x = pixel_scale[0];
            let scale_y = pixel_scale[1];

            Some(TileBounds {
                west: origin_x,
                north: origin_y,
                east: origin_x + width * scale_x,
                south: origin_y - height * scale_y,
            })
        } else {
            None
        }
    }

    /// Get the COG bounds in the COG's **native CRS units** (degrees for
    /// geographic, Web Mercator meters for Web Mercator). Used internally for
    /// sampling against the matching native requested-tile bounds.
    pub fn bounds(&self) -> Option<&TileBounds> {
        self.bounds.as_ref()
    }

    /// The COG's coordinate reference system. Callers use this to build the
    /// requested-tile bounds in the same space the COG was sampled in.
    pub fn crs(&self) -> CogCrs {
        self.crs
    }

    /// Get the COG bounds normalized to **WGS84 degrees**, regardless of the
    /// COG's native CRS. Use for intersection tests against XYZ tiles and for
    /// reporting the extent to the catalog/API.
    pub fn wgs84_bounds(&self) -> Option<TileBounds> {
        self.bounds.map(|b| match self.crs {
            CogCrs::Geographic => b,
            CogCrs::WebMercator => b.mercator_to_wgs84(),
        })
    }

    /// Read the `GDAL_NODATA` tag (TIFF tag 42113) from the first IFD.
    ///
    /// GDAL writes the tag as ASCII (e.g. `"-9999\0"`), which is the convention
    /// for COG DEMs from QGIS / `gdal_translate`. Returning the parsed numeric
    /// value lets the elevation reader treat those pixels as NaN without the
    /// caller having to know the per-file sentinel — which matters for DEM
    /// overlays added via CMS where no explicit `nodata` config is supplied.
    pub fn nodata_from_metadata(&self) -> Option<f64> {
        let ifd = self.tiff.ifds().first()?;
        let value = ifd.other_tags().get(&Tag::GdalNodata)?;
        match value {
            TagValue::Ascii(s) => s.trim_end_matches('\0').trim().parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Get the image dimensions (width, height).
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.tiff
            .ifds()
            .first()
            .map(|ifd| (ifd.image_width(), ifd.image_height()))
    }

    /// Get samples per pixel (1=grayscale, 3=RGB, 4=RGBA).
    pub fn samples_per_pixel(&self) -> u16 {
        self.samples_per_pixel
    }

    /// Check if the COG intersects with the given bounds.
    pub fn intersects(&self, bounds: &TileBounds) -> bool {
        match &self.bounds {
            Some(cog_bounds) => cog_bounds.intersects(bounds),
            None => true, // If we don't know bounds, assume it might intersect
        }
    }

    /// Select the best IFD (overview) for the requested tile.
    fn select_best_ifd(&self, bounds: &TileBounds, tile_size: u32) -> usize {
        let cog_bounds = match &self.bounds {
            Some(b) => b,
            None => return 0,
        };

        let ifds = self.tiff.ifds();
        let ifd_count = ifds.len();

        if ifd_count <= 1 {
            return 0;
        }

        // Calculate required resolution
        let tile_width_deg = bounds.east - bounds.west;
        let tile_height_deg = bounds.north - bounds.south;
        let required_res_x = tile_size as f64 / tile_width_deg;
        let required_res_y = tile_size as f64 / tile_height_deg;
        let required_res = required_res_x.max(required_res_y);

        // Get COG extent
        let cog_width_deg = cog_bounds.east - cog_bounds.west;
        let cog_height_deg = cog_bounds.north - cog_bounds.south;

        // Find the smallest IFD with sufficient resolution
        let mut best_ifd = 0;
        for (idx, ifd) in ifds.iter().enumerate() {
            let ifd_res_x = ifd.image_width() as f64 / cog_width_deg;
            let ifd_res_y = ifd.image_height() as f64 / cog_height_deg;
            let ifd_res = ifd_res_x.max(ifd_res_y);

            // Use 1.5x margin for quality
            if ifd_res >= required_res * 1.5 {
                best_ifd = idx;
            } else {
                break;
            }
        }

        best_ifd
    }

    /// Read a tile as RGBA image data.
    ///
    /// `bounds` must be in the COG's **native CRS** (see [`Self::crs`]): degrees
    /// for geographic COGs, Web Mercator meters for Web Mercator COGs. Build it
    /// with [`crate::tile::xyz_to_bounds`] or
    /// [`crate::cog::mercator_tile_bounds`] accordingly.
    pub async fn read_tile_rgba(
        &self,
        bounds: &TileBounds,
        tile_size: u32,
        nodata: Option<&NoDataConfig>,
    ) -> Result<Vec<u8>, CogError> {
        let cog_bounds = self
            .bounds
            .as_ref()
            .ok_or_else(|| CogError::ReadError("COG has no geographic bounds".to_string()))?;

        // Select best IFD
        let ifd_idx = self.select_best_ifd(bounds, tile_size);
        let ifd = self.tiff.ifds().get(ifd_idx).ok_or(CogError::NoIfd)?;

        let img_width = ifd.image_width();
        let img_height = ifd.image_height();
        let cog_tile_w = ifd.tile_width().unwrap_or(256);
        let cog_tile_h = ifd.tile_height().unwrap_or(256);
        let samples_per_pixel = ifd.samples_per_pixel();
        let bits_per_sample = ifd.bits_per_sample().first().copied().unwrap_or(8);

        tracing::debug!(
            ifd_idx = ifd_idx,
            img_size = %format!("{}x{}", img_width, img_height),
            cog_tile_size = %format!("{}x{}", cog_tile_w, cog_tile_h),
            samples_per_pixel = samples_per_pixel,
            bits_per_sample = bits_per_sample,
            "Selected IFD for tile generation"
        );

        // Calculate tile range
        let tile_range = TileRange::from_bounds(
            bounds,
            cog_bounds,
            img_width,
            img_height,
            cog_tile_w,
            cog_tile_h,
            ifd.tile_count().unwrap_or((1, 1)),
        );

        // No intersection
        if tile_range.is_empty() {
            return Ok(vec![0; (tile_size * tile_size * 4) as usize]);
        }

        // Allocate pixel buffer
        let (buffer_width, buffer_height) = tile_range.buffer_size(cog_tile_w, cog_tile_h);
        let mut pixel_buffer: Vec<u8> = vec![0; buffer_width * buffer_height * 4];

        // Fetch and decode tiles
        let reader = ObjectReader::new(self.store.clone(), self.path.clone());
        let decoder_registry = decoder_registry();

        for ty in tile_range.y_start..tile_range.y_end {
            for tx in tile_range.x_start..tile_range.x_end {
                match ifd.fetch_tile(tx, ty, &reader).await {
                    Ok(tile) => match tile.decode(&decoder_registry) {
                        Ok(decoded_bytes) => {
                            let mut rgba = decode_rgba(
                                &decoded_bytes,
                                cog_tile_w,
                                cog_tile_h,
                                samples_per_pixel,
                                bits_per_sample,
                            );

                            // Apply nodata -> transparent
                            if let Some(nodata_config) = nodata {
                                apply_nodata_rgba(
                                    &mut rgba,
                                    cog_tile_w as usize,
                                    cog_tile_h as usize,
                                    samples_per_pixel,
                                    nodata_config,
                                );
                            }

                            // Copy to buffer. Edge chunks may decode to their
                            // real (cropped) extent rather than the padded
                            // block — see `decoded_chunk_stride`.
                            let (chunk_w, chunk_h) =
                                chunk_extent(tx, ty, cog_tile_w, cog_tile_h, img_width, img_height);
                            let bytes_per_pixel = (bits_per_sample as usize).div_ceil(8)
                                * samples_per_pixel.max(1) as usize;
                            let src_stride = decoded_chunk_stride(
                                decoded_bytes.len(),
                                bytes_per_pixel,
                                cog_tile_w as usize,
                                cog_tile_h as usize,
                                chunk_w,
                            );

                            let buf_x_offset = (tx - tile_range.x_start) * cog_tile_w as usize;
                            let buf_y_offset = (ty - tile_range.y_start) * cog_tile_h as usize;

                            blit_chunk(
                                &rgba,
                                src_stride,
                                4,
                                chunk_w,
                                chunk_h,
                                &mut pixel_buffer,
                                buffer_width,
                                buf_x_offset,
                                buf_y_offset,
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to decode tile ({}, {}): {:?}", tx, ty, e);
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Failed to fetch tile ({}, {}): {:?}", tx, ty, e);
                    }
                }
            }
        }

        // Resample to output tile size with bilinear interpolation
        let buffer_origin = (
            tile_range.x_start as f64 * cog_tile_w as f64,
            tile_range.y_start as f64 * cog_tile_h as f64,
        );

        let pixels: Vec<[u8; 4]> = resample_to_tile(
            bounds,
            cog_bounds,
            img_width,
            img_height,
            tile_size,
            buffer_origin,
            |buf_x, buf_y| bilinear_rgba(&pixel_buffer, buffer_width, buffer_height, buf_x, buf_y),
        );

        // Flatten [u8; 4] to Vec<u8>
        let output: Vec<u8> = pixels.into_iter().flatten().collect();

        Ok(output)
    }

    /// Read a tile as elevation (f64) data.
    ///
    /// `bounds` must be in the COG's **native CRS** (see [`Self::crs`]): degrees
    /// for geographic COGs, Web Mercator meters for Web Mercator COGs.
    pub async fn read_tile_elevation(
        &self,
        bounds: &TileBounds,
        tile_size: u32,
        nodata: Option<f64>,
    ) -> Result<Vec<f64>, CogError> {
        let cog_bounds = self
            .bounds
            .as_ref()
            .ok_or_else(|| CogError::ReadError("COG has no geographic bounds".to_string()))?;

        let ifd_idx = self.select_best_ifd(bounds, tile_size);
        let ifd = self.tiff.ifds().get(ifd_idx).ok_or(CogError::NoIfd)?;

        let img_width = ifd.image_width();
        let img_height = ifd.image_height();
        let cog_tile_w = ifd.tile_width().unwrap_or(256);
        let cog_tile_h = ifd.tile_height().unwrap_or(256);
        let samples_per_pixel = ifd.samples_per_pixel();
        let sample_format = ifd
            .sample_format()
            .first()
            .copied()
            .unwrap_or(SampleFormat::Uint);
        let bits_per_sample = ifd.bits_per_sample().first().copied().unwrap_or(32);

        // Calculate tile range
        let tile_range = TileRange::from_bounds(
            bounds,
            cog_bounds,
            img_width,
            img_height,
            cog_tile_w,
            cog_tile_h,
            ifd.tile_count().unwrap_or((1, 1)),
        );

        // No intersection
        if tile_range.is_empty() {
            return Ok(vec![f64::NAN; (tile_size * tile_size) as usize]);
        }

        let (buffer_width, buffer_height) = tile_range.buffer_size(cog_tile_w, cog_tile_h);
        let mut pixel_buffer: Vec<f64> = vec![f64::NAN; buffer_width * buffer_height];

        let reader = ObjectReader::new(self.store.clone(), self.path.clone());
        let decoder_registry = decoder_registry();

        for ty in tile_range.y_start..tile_range.y_end {
            for tx in tile_range.x_start..tile_range.x_end {
                match ifd.fetch_tile(tx, ty, &reader).await {
                    Ok(tile) => match tile.decode(&decoder_registry) {
                        Ok(decoded_bytes) => {
                            let mut elevations = decode_elevation(
                                &decoded_bytes,
                                cog_tile_w,
                                cog_tile_h,
                                sample_format,
                                bits_per_sample,
                            );

                            // Apply nodata -> NaN.
                            //
                            // Use a scaled tolerance instead of strict equality:
                            // DEM mosaics that were built with anything other
                            // than nearest-neighbour (e.g. gdalwarp's default
                            // bilinear) blend real elevations with the nodata
                            // sentinel at every mask/data boundary, producing
                            // near-but-not-equal sentinels.
                            //
                            // - Small sentinels (255, -9999): blend gap is at
                            //   most ~0.5 m off, so a flat 0.5 m floor catches
                            //   `254.99996` next to `255.0`.
                            // - Huge sentinels (f32::MIN ≈ −3.4 × 10³⁸): blends
                            //   can land *anywhere* between the sentinel and
                            //   the real value (e.g. `−2.7 × 10³⁷`), so a fixed
                            //   0.5 m floor is useless. Scale the tolerance
                            //   to `|nodata| · 1e-3` (≈ 3.4 × 10³⁵ for
                            //   f32::MIN), which still leaves seven orders of
                            //   magnitude between "blended fringe" and any
                            //   real elevation.
                            //
                            // Defense in depth: `decode_elevation` also drops
                            // anything beyond `MAX_PHYSICAL_ELEVATION_M`, so
                            // even values that slip the tolerance check are
                            // caught before they reach the mesh.
                            if let Some(nodata_val) = nodata {
                                let tol = (nodata_val.abs() * 1e-3).max(0.5);
                                for v in elevations.iter_mut() {
                                    if (*v - nodata_val).abs() < tol {
                                        *v = f64::NAN;
                                    }
                                }
                            }

                            // Copy to buffer. Edge chunks may decode to their
                            // real (cropped) extent rather than the padded
                            // block — see `decoded_chunk_stride`.
                            let (chunk_w, chunk_h) =
                                chunk_extent(tx, ty, cog_tile_w, cog_tile_h, img_width, img_height);
                            let bytes_per_pixel = (bits_per_sample as usize).div_ceil(8)
                                * samples_per_pixel.max(1) as usize;
                            let src_stride = decoded_chunk_stride(
                                decoded_bytes.len(),
                                bytes_per_pixel,
                                cog_tile_w as usize,
                                cog_tile_h as usize,
                                chunk_w,
                            );

                            let buf_x_offset = (tx - tile_range.x_start) * cog_tile_w as usize;
                            let buf_y_offset = (ty - tile_range.y_start) * cog_tile_h as usize;

                            blit_chunk(
                                &elevations,
                                src_stride,
                                1,
                                chunk_w,
                                chunk_h,
                                &mut pixel_buffer,
                                buffer_width,
                                buf_x_offset,
                                buf_y_offset,
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to decode tile ({}, {}): {:?}", tx, ty, e);
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Failed to fetch tile ({}, {}): {:?}", tx, ty, e);
                    }
                }
            }
        }

        // Resample using bilinear interpolation
        let buffer_origin = (
            tile_range.x_start as f64 * cog_tile_w as f64,
            tile_range.y_start as f64 * cog_tile_h as f64,
        );

        let output = resample_to_tile(
            bounds,
            cog_bounds,
            img_width,
            img_height,
            tile_size,
            buffer_origin,
            |buf_x, buf_y| bilinear_f64(&pixel_buffer, buffer_width, buffer_height, buf_x, buf_y),
        );

        Ok(output)
    }
}

/// Real pixel extent of chunk `(tx, ty)`.
///
/// TIFF chunks (tiles) are written padded out to `tile_w × tile_h`, but the
/// chunks along the right/bottom edge only cover `image_size - offset` real
/// pixels. Returns `(0, _)` / `(_, 0)` for chunks entirely outside the image.
fn chunk_extent(
    tx: usize,
    ty: usize,
    tile_w: u32,
    tile_h: u32,
    img_width: u32,
    img_height: u32,
) -> (usize, usize) {
    let w = (img_width as usize).saturating_sub(tx * tile_w as usize);
    let h = (img_height as usize).saturating_sub(ty * tile_h as usize);
    (w.min(tile_w as usize), h.min(tile_h as usize))
}

/// Row stride, in pixels, of a decoded chunk buffer.
///
/// `Tile::decode` applies the TIFF predictor, and for a **partial-width** edge
/// chunk the horizontal-differencing unpredictors return only the chunk's real
/// extent (`chunk_w × chunk_h`) — the padding columns are stripped. With
/// `Predictor::None` the full padded `tile_w × tile_h` block comes back
/// instead. Rather than assuming either behaviour (it differs per predictor,
/// and could change with the library), decide from the decoded byte length: if
/// there are enough bytes for the padded block, the padded stride applies,
/// otherwise the buffer is cropped and the stride is `chunk_w`.
fn decoded_chunk_stride(
    decoded_len: usize,
    bytes_per_pixel: usize,
    tile_w: usize,
    tile_h: usize,
    chunk_w: usize,
) -> usize {
    if bytes_per_pixel == 0 {
        return tile_w;
    }
    let decoded_pixels = decoded_len / bytes_per_pixel;
    if decoded_pixels >= tile_w * tile_h {
        tile_w
    } else {
        chunk_w
    }
}

/// Copy the `chunk_w × chunk_h` valid region of a decoded chunk into the
/// mosaic buffer at `(dst_x, dst_y)`. Source rows are `src_stride` pixels
/// apart, destination rows `dst_stride` pixels apart; each pixel is
/// `components` elements wide.
#[allow(clippy::too_many_arguments)]
fn blit_chunk<T: Copy>(
    src: &[T],
    src_stride: usize,
    components: usize,
    chunk_w: usize,
    chunk_h: usize,
    dst: &mut [T],
    dst_stride: usize,
    dst_x: usize,
    dst_y: usize,
) {
    for y in 0..chunk_h {
        for x in 0..chunk_w {
            let src_idx = (y * src_stride + x) * components;
            let dst_idx = ((dst_y + y) * dst_stride + (dst_x + x)) * components;

            if src_idx + components <= src.len() && dst_idx + components <= dst.len() {
                dst[dst_idx..dst_idx + components]
                    .copy_from_slice(&src[src_idx..src_idx + components]);
            }
        }
    }
}

/// Apply nodata configuration to RGBA buffer, making matching pixels transparent.
fn apply_nodata_rgba(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    samples_per_pixel: u16,
    nodata: &NoDataConfig,
) {
    for y in 0..height {
        for x in 0..width {
            let values = get_pixel_values(rgba, width, x, y, samples_per_pixel);
            if nodata.is_nodata(&values) {
                let idx = (y * width + x) * 4;
                if idx + 3 < rgba.len() {
                    rgba[idx + 3] = 0; // Set alpha to 0 (transparent)
                }
            }
        }
    }
}

#[cfg(test)]
mod chunk_tests {
    use super::*;

    // Synthetic layout: 300 x 200 image in 128 x 128 chunks, so the last
    // chunk column is 300 - 2*128 = 44 px wide and the last chunk row is
    // 200 - 128 = 72 px tall.
    const IMG_W: u32 = 300;
    const IMG_H: u32 = 200;
    const TILE: u32 = 128;

    #[test]
    fn chunk_extent_clips_edges() {
        assert_eq!(chunk_extent(0, 0, TILE, TILE, IMG_W, IMG_H), (128, 128));
        assert_eq!(chunk_extent(2, 0, TILE, TILE, IMG_W, IMG_H), (44, 128));
        assert_eq!(chunk_extent(0, 1, TILE, TILE, IMG_W, IMG_H), (128, 72));
        assert_eq!(chunk_extent(2, 1, TILE, TILE, IMG_W, IMG_H), (44, 72));
        // Entirely outside the image.
        assert_eq!(chunk_extent(9, 9, TILE, TILE, IMG_W, IMG_H), (0, 0));
    }

    #[test]
    fn stride_follows_decoded_length() {
        let bpp = 4; // f32 elevation
        // Cropped buffer (predictor applied): 44 x 128 samples.
        assert_eq!(
            decoded_chunk_stride(44 * 128 * bpp, bpp, 128, 128, 44),
            44,
            "cropped edge chunk must use the real chunk width as stride"
        );
        // Padded buffer (no predictor): 128 x 128 samples.
        assert_eq!(
            decoded_chunk_stride(128 * 128 * bpp, bpp, 128, 128, 44),
            128,
            "padded edge chunk must use the padded tile width as stride"
        );
        // Interior chunk: both interpretations agree.
        assert_eq!(
            decoded_chunk_stride(128 * 128 * bpp, bpp, 128, 128, 128),
            128
        );
    }

    /// Regression test for the edge-chunk stride bug: a partial-width chunk
    /// whose decoded buffer is cropped used to be read with the padded stride,
    /// scrambling rows and leaving the bottom of the block untouched.
    #[test]
    fn edge_chunk_cropped_buffer_lands_row_aligned() {
        let (chunk_w, chunk_h) = chunk_extent(2, 0, TILE, TILE, IMG_W, IMG_H);
        assert_eq!((chunk_w, chunk_h), (44, 128));

        // Decoded (cropped) elevation samples: value encodes (x, y).
        let src: Vec<f64> = (0..chunk_h)
            .flat_map(|y| (0..chunk_w).map(move |x| (y * 1000 + x) as f64))
            .collect();
        let stride = decoded_chunk_stride(src.len() * 4, 4, TILE as usize, TILE as usize, chunk_w);

        // One-chunk-wide mosaic buffer in padded-block space.
        let buffer_width = TILE as usize;
        let mut dst = vec![f64::NAN; buffer_width * TILE as usize];
        blit_chunk(
            &src,
            stride,
            1,
            chunk_w,
            chunk_h,
            &mut dst,
            buffer_width,
            0,
            0,
        );

        for y in 0..chunk_h {
            for x in 0..chunk_w {
                assert_eq!(
                    dst[y * buffer_width + x],
                    (y * 1000 + x) as f64,
                    "pixel ({x}, {y}) misplaced"
                );
            }
            // Padding columns stay untouched.
            assert!(dst[y * buffer_width + chunk_w].is_nan());
        }
    }

    /// The same edge chunk decoded *without* a predictor comes back padded;
    /// it must still land row-aligned.
    #[test]
    fn edge_chunk_padded_buffer_lands_row_aligned() {
        let (chunk_w, chunk_h) = chunk_extent(2, 0, TILE, TILE, IMG_W, IMG_H);
        let padded = TILE as usize;
        let src: Vec<f64> = (0..padded)
            .flat_map(|y| (0..padded).map(move |x| (y * 1000 + x) as f64))
            .collect();
        let stride = decoded_chunk_stride(src.len() * 4, 4, padded, padded, chunk_w);
        assert_eq!(stride, padded);

        let mut dst = vec![f64::NAN; padded * padded];
        blit_chunk(&src, stride, 1, chunk_w, chunk_h, &mut dst, padded, 0, 0);

        for y in 0..chunk_h {
            for x in 0..chunk_w {
                assert_eq!(dst[y * padded + x], (y * 1000 + x) as f64);
            }
            assert!(dst[y * padded + chunk_w].is_nan());
        }
    }

    /// RGBA path: 4 components per pixel, cropped edge chunk.
    #[test]
    fn edge_chunk_rgba_cropped_buffer() {
        let (chunk_w, chunk_h) = chunk_extent(2, 0, TILE, TILE, IMG_W, IMG_H);
        let src: Vec<u8> = (0..chunk_h)
            .flat_map(|y| (0..chunk_w).flat_map(move |x| [x as u8, y as u8, 0, 255].into_iter()))
            .collect();
        // 8-bit RGBA => 4 bytes per pixel.
        let stride = decoded_chunk_stride(src.len(), 4, TILE as usize, TILE as usize, chunk_w);
        assert_eq!(stride, chunk_w);

        let buffer_width = TILE as usize;
        let mut dst = vec![0u8; buffer_width * TILE as usize * 4];
        blit_chunk(
            &src,
            stride,
            4,
            chunk_w,
            chunk_h,
            &mut dst,
            buffer_width,
            0,
            0,
        );

        for y in 0..chunk_h {
            for x in 0..chunk_w {
                let i = (y * buffer_width + x) * 4;
                assert_eq!(&dst[i..i + 4], &[x as u8, y as u8, 0, 255]);
            }
        }
    }

    /// Interior (full) chunks must keep working, and land at their offset.
    #[test]
    fn interior_chunk_offset_copy() {
        let (chunk_w, chunk_h) = chunk_extent(0, 0, TILE, TILE, IMG_W, IMG_H);
        assert_eq!((chunk_w, chunk_h), (128, 128));
        let src: Vec<f64> = (0..chunk_h)
            .flat_map(|y| (0..chunk_w).map(move |x| (y * 1000 + x) as f64))
            .collect();
        let stride = decoded_chunk_stride(src.len() * 4, 4, TILE as usize, TILE as usize, chunk_w);

        let buffer_width = 2 * TILE as usize;
        let mut dst = vec![f64::NAN; buffer_width * TILE as usize];
        blit_chunk(
            &src,
            stride,
            1,
            chunk_w,
            chunk_h,
            &mut dst,
            buffer_width,
            TILE as usize,
            0,
        );

        for y in 0..chunk_h {
            for x in 0..chunk_w {
                assert_eq!(
                    dst[y * buffer_width + TILE as usize + x],
                    (y * 1000 + x) as f64
                );
                assert!(dst[y * buffer_width + x].is_nan());
            }
        }
    }
}
