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

                            // Copy to buffer
                            let buf_x_offset = (tx - tile_range.x_start) * cog_tile_w as usize;
                            let buf_y_offset = (ty - tile_range.y_start) * cog_tile_h as usize;
                            let tile_px_x_start = tx * cog_tile_w as usize;
                            let tile_px_y_start = ty * cog_tile_h as usize;

                            for y in 0..cog_tile_h as usize {
                                let img_y = tile_px_y_start + y;
                                if img_y >= img_height as usize {
                                    continue;
                                }

                                for x in 0..cog_tile_w as usize {
                                    let img_x = tile_px_x_start + x;
                                    if img_x >= img_width as usize {
                                        continue;
                                    }

                                    let src_idx = (y * cog_tile_w as usize + x) * 4;
                                    let dst_idx = ((buf_y_offset + y) * buffer_width
                                        + (buf_x_offset + x))
                                        * 4;

                                    if src_idx + 3 < rgba.len() && dst_idx + 3 < pixel_buffer.len()
                                    {
                                        pixel_buffer[dst_idx..dst_idx + 4]
                                            .copy_from_slice(&rgba[src_idx..src_idx + 4]);
                                    }
                                }
                            }
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

                            // Copy to buffer
                            let buf_x_offset = (tx - tile_range.x_start) * cog_tile_w as usize;
                            let buf_y_offset = (ty - tile_range.y_start) * cog_tile_h as usize;
                            let tile_px_x_start = tx * cog_tile_w as usize;
                            let tile_px_y_start = ty * cog_tile_h as usize;

                            for y in 0..cog_tile_h as usize {
                                let img_y = tile_px_y_start + y;
                                if img_y >= img_height as usize {
                                    continue;
                                }

                                for x in 0..cog_tile_w as usize {
                                    let img_x = tile_px_x_start + x;
                                    if img_x >= img_width as usize {
                                        continue;
                                    }

                                    let src_idx = y * cog_tile_w as usize + x;
                                    let dst_idx =
                                        (buf_y_offset + y) * buffer_width + (buf_x_offset + x);

                                    if src_idx < elevations.len() && dst_idx < pixel_buffer.len() {
                                        pixel_buffer[dst_idx] = elevations[src_idx];
                                    }
                                }
                            }
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
