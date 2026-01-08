//! Cloud Optimized GeoTIFF reader.

use std::sync::Arc;

use async_tiff::{TIFF, decoder::DecoderRegistry, reader::ObjectReader, tiff::tags::SampleFormat};
use object_store::{ObjectStore, path::Path as ObjectPath};
use thiserror::Error;

use super::{
    bounds::{TileBounds, geo_to_pixel_x, geo_to_pixel_y},
    decode::{decode_elevation, decode_rgba, get_pixel_values},
    interpolate::{bilinear_f64, bilinear_rgba},
};
use crate::config::NoDataConfig;

#[derive(Error, Debug)]
pub enum CogError {
    #[error("Failed to open COG file: {0}")]
    OpenError(String),
    #[error("Failed to read COG data: {0}")]
    ReadError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("TIFF error: {0}")]
    TiffError(String),
    #[error("No IFD available")]
    NoIfd,
    #[error("Unsupported CRS: expected WGS84 (EPSG:4326), got EPSG:{0}")]
    UnsupportedCrs(u16),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Object store error: {0}")]
    ObjectStoreError(String),
}

/// Cloud Optimized GeoTIFF reader with HTTP range request support.
pub struct CogReader {
    tiff: TIFF,
    store: Arc<dyn ObjectStore>,
    path: ObjectPath,
    bounds: Option<TileBounds>,
    samples_per_pixel: u16,
}

impl CogReader {
    /// Open a COG file from an object store (HTTP, GCS, S3, local).
    pub async fn open(store: Arc<dyn ObjectStore>, path: ObjectPath) -> Result<Self, CogError> {
        let reader = ObjectReader::new(store.clone(), path.clone());

        let tiff = TIFF::try_open(Box::new(reader))
            .await
            .map_err(|e| CogError::OpenError(format!("{e:?}")))?;

        // Validate CRS
        Self::check_crs(&tiff)?;

        // Extract bounds from GeoTIFF metadata
        let bounds = Self::extract_bounds(&tiff);

        // Get samples per pixel
        let samples_per_pixel = tiff
            .ifds()
            .as_ref()
            .first()
            .map(|ifd| ifd.samples_per_pixel())
            .unwrap_or(1);

        // Log IFD info
        let ifds = tiff.ifds();
        tracing::info!("COG has {} IFD(s)", ifds.as_ref().len());
        for (idx, ifd) in ifds.as_ref().iter().enumerate() {
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
            samples_per_pixel,
        })
    }

    fn check_crs(tiff: &TIFF) -> Result<(), CogError> {
        let ifd = tiff.ifds().as_ref().first().ok_or(CogError::NoIfd)?;

        if let Some(geo_keys) = ifd.geo_key_directory()
            && let Some(epsg) = geo_keys.epsg_code()
        {
            if epsg == 4326 {
                return Ok(());
            }
            return Err(CogError::UnsupportedCrs(epsg));
        }

        // If no EPSG code found, assume WGS84
        tracing::warn!("No EPSG code found in GeoTIFF metadata, assuming WGS84");
        Ok(())
    }

    fn extract_bounds(tiff: &TIFF) -> Option<TileBounds> {
        let ifd = tiff.ifds().as_ref().first()?;
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

    /// Get the geographic bounds of the COG.
    pub fn bounds(&self) -> Option<&TileBounds> {
        self.bounds.as_ref()
    }

    /// Get the image dimensions (width, height).
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.tiff
            .ifds()
            .as_ref()
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
        let ifd_count = ifds.as_ref().len();

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
        for (idx, ifd) in ifds.as_ref().iter().enumerate() {
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
        let ifd = self
            .tiff
            .ifds()
            .as_ref()
            .get(ifd_idx)
            .ok_or(CogError::NoIfd)?;

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

        // Convert geo bounds to pixel coordinates
        let px_west = geo_to_pixel_x(bounds.west, cog_bounds, img_width);
        let px_east = geo_to_pixel_x(bounds.east, cog_bounds, img_width);
        let px_north = geo_to_pixel_y(bounds.north, cog_bounds, img_height);
        let px_south = geo_to_pixel_y(bounds.south, cog_bounds, img_height);

        let (tile_count_x, tile_count_y) = ifd.tile_count().unwrap_or((1, 1));

        // Calculate tile range
        let tile_x_start = (px_west / cog_tile_w as f64)
            .floor()
            .max(0.0)
            .min(tile_count_x as f64) as usize;
        let tile_x_end = (px_east / cog_tile_w as f64)
            .ceil()
            .max(0.0)
            .min(tile_count_x as f64) as usize;
        let tile_y_start = (px_north / cog_tile_h as f64)
            .floor()
            .max(0.0)
            .min(tile_count_y as f64) as usize;
        let tile_y_end = (px_south / cog_tile_h as f64)
            .ceil()
            .max(0.0)
            .min(tile_count_y as f64) as usize;

        // No intersection
        if tile_x_end <= tile_x_start || tile_y_end <= tile_y_start {
            return Ok(vec![0; (tile_size * tile_size * 4) as usize]);
        }

        // Allocate pixel buffer
        let buffer_width = (tile_x_end - tile_x_start) * cog_tile_w as usize;
        let buffer_height = (tile_y_end - tile_y_start) * cog_tile_h as usize;
        let mut pixel_buffer: Vec<u8> = vec![0; buffer_width * buffer_height * 4];

        // Fetch and decode tiles
        let reader = ObjectReader::new(self.store.clone(), self.path.clone());
        let mut boxed_reader: Box<dyn async_tiff::reader::AsyncFileReader> = Box::new(reader);
        let decoder_registry = DecoderRegistry::default();

        for ty in tile_y_start..tile_y_end {
            for tx in tile_x_start..tile_x_end {
                match ifd.fetch_tile(tx, ty, boxed_reader.as_mut()).await {
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
                            let buf_x_offset = (tx - tile_x_start) * cog_tile_w as usize;
                            let buf_y_offset = (ty - tile_y_start) * cog_tile_h as usize;
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
        let buffer_origin_x = tile_x_start as f64 * cog_tile_w as f64;
        let buffer_origin_y = tile_y_start as f64 * cog_tile_h as f64;
        let mut output = Vec::with_capacity((tile_size * tile_size * 4) as usize);

        for out_y in 0..tile_size {
            for out_x in 0..tile_size {
                // Convert output pixel to geo coordinate
                let geo_x = bounds.west
                    + (out_x as f64 + 0.5) / tile_size as f64 * (bounds.east - bounds.west);
                let geo_y = bounds.north
                    - (out_y as f64 + 0.5) / tile_size as f64 * (bounds.north - bounds.south);

                // Convert to COG pixel coordinate
                let px_x = geo_to_pixel_x(geo_x, cog_bounds, img_width);
                let px_y = geo_to_pixel_y(geo_y, cog_bounds, img_height);

                // Convert to buffer coordinate
                let buf_x = px_x - buffer_origin_x;
                let buf_y = px_y - buffer_origin_y;

                // Bilinear interpolation
                let pixel = bilinear_rgba(&pixel_buffer, buffer_width, buffer_height, buf_x, buf_y);
                output.extend_from_slice(&pixel);
            }
        }

        Ok(output)
    }

    /// Read a tile as elevation (f64) data.
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
        let ifd = self
            .tiff
            .ifds()
            .as_ref()
            .get(ifd_idx)
            .ok_or(CogError::NoIfd)?;

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

        // Convert geo bounds to pixel coordinates
        let px_west = geo_to_pixel_x(bounds.west, cog_bounds, img_width);
        let px_east = geo_to_pixel_x(bounds.east, cog_bounds, img_width);
        let px_north = geo_to_pixel_y(bounds.north, cog_bounds, img_height);
        let px_south = geo_to_pixel_y(bounds.south, cog_bounds, img_height);

        let (tile_count_x, tile_count_y) = ifd.tile_count().unwrap_or((1, 1));

        let tile_x_start = (px_west / cog_tile_w as f64)
            .floor()
            .max(0.0)
            .min(tile_count_x as f64) as usize;
        let tile_x_end = (px_east / cog_tile_w as f64)
            .ceil()
            .max(0.0)
            .min(tile_count_x as f64) as usize;
        let tile_y_start = (px_north / cog_tile_h as f64)
            .floor()
            .max(0.0)
            .min(tile_count_y as f64) as usize;
        let tile_y_end = (px_south / cog_tile_h as f64)
            .ceil()
            .max(0.0)
            .min(tile_count_y as f64) as usize;

        if tile_x_end <= tile_x_start || tile_y_end <= tile_y_start {
            return Ok(vec![f64::NAN; (tile_size * tile_size) as usize]);
        }

        let buffer_width = (tile_x_end - tile_x_start) * cog_tile_w as usize;
        let buffer_height = (tile_y_end - tile_y_start) * cog_tile_h as usize;
        let mut pixel_buffer: Vec<f64> = vec![f64::NAN; buffer_width * buffer_height];

        let reader = ObjectReader::new(self.store.clone(), self.path.clone());
        let mut boxed_reader: Box<dyn async_tiff::reader::AsyncFileReader> = Box::new(reader);
        let decoder_registry = DecoderRegistry::default();

        for ty in tile_y_start..tile_y_end {
            for tx in tile_x_start..tile_x_end {
                match ifd.fetch_tile(tx, ty, boxed_reader.as_mut()).await {
                    Ok(tile) => match tile.decode(&decoder_registry) {
                        Ok(decoded_bytes) => {
                            let mut elevations = decode_elevation(
                                &decoded_bytes,
                                cog_tile_w,
                                cog_tile_h,
                                sample_format,
                                bits_per_sample,
                            );

                            // Apply nodata -> NaN
                            if let Some(nodata_val) = nodata {
                                for v in elevations.iter_mut() {
                                    if (*v - nodata_val).abs() < 1e-6 {
                                        *v = f64::NAN;
                                    }
                                }
                            }

                            // Copy to buffer
                            let buf_x_offset = (tx - tile_x_start) * cog_tile_w as usize;
                            let buf_y_offset = (ty - tile_y_start) * cog_tile_h as usize;
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

        // Resample
        let buffer_origin_x = tile_x_start as f64 * cog_tile_w as f64;
        let buffer_origin_y = tile_y_start as f64 * cog_tile_h as f64;
        let mut output = Vec::with_capacity((tile_size * tile_size) as usize);

        for out_y in 0..tile_size {
            for out_x in 0..tile_size {
                let geo_x = bounds.west
                    + (out_x as f64 + 0.5) / tile_size as f64 * (bounds.east - bounds.west);
                let geo_y = bounds.north
                    - (out_y as f64 + 0.5) / tile_size as f64 * (bounds.north - bounds.south);

                let px_x = geo_to_pixel_x(geo_x, cog_bounds, img_width);
                let px_y = geo_to_pixel_y(geo_y, cog_bounds, img_height);

                let buf_x = px_x - buffer_origin_x;
                let buf_y = px_y - buffer_origin_y;

                let elevation =
                    bilinear_f64(&pixel_buffer, buffer_width, buffer_height, buf_x, buf_y);
                output.push(elevation);
            }
        }

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
