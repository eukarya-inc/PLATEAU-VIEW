//! Generic XYZ-tile DEM source.
//!
//! Fetches tiles from a `{z}/{x}/{y}` URL template and decodes them as either
//! Terrarium or Mapbox Terrain-RGB. Used for `dem` overlay layers in the
//! config JSON; `MapterhornSource` is the specialised default-base equivalent.

use async_trait::async_trait;
use image::GenericImageView;
use reqwest::StatusCode;
use serde::Deserialize;

use super::dem::{DemError, DemProvider, DemTile, GeoBounds};
use super::mapbox::mapbox_rgb_to_elevation;
use super::terrarium::rgb_to_elevation;

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum XyzDemEncoding {
    #[default]
    Terrarium,
    Mapbox,
}

pub struct XyzDemSource {
    url_template: String,
    encoding: XyzDemEncoding,
    max_zoom: u8,
    native_tile_size: u32,
    version: String,
    slug: String,
    bounds: Option<GeoBounds>,
    client: reqwest::Client,
}

impl XyzDemSource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slug: impl Into<String>,
        url_template: impl Into<String>,
        encoding: XyzDemEncoding,
        version: impl Into<String>,
        max_zoom: u8,
        native_tile_size: u32,
        bounds: Option<GeoBounds>,
    ) -> Self {
        Self {
            url_template: url_template.into(),
            encoding,
            max_zoom,
            native_tile_size,
            version: version.into(),
            slug: slug.into(),
            bounds,
            client: reqwest::Client::new(),
        }
    }

    fn build_url(&self, z: u8, x: u32, y: u32) -> String {
        self.url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string())
    }
}

#[async_trait]
impl DemProvider for XyzDemSource {
    async fn get_tile_elevations(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<DemTile, DemError> {
        if z > self.max_zoom {
            return Err(DemError::OutOfRange);
        }
        let url = self.build_url(z, x, y);
        let response = self.client.get(&url).send().await?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Err(DemError::NotFound);
        }
        if !status.is_success() {
            return Err(DemError::Http(format!("HTTP {status}")));
        }

        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_start_matches("W/").trim_matches('"').to_string())
            .filter(|s| !s.is_empty());

        let bytes = response.bytes().await?;
        let img = image::load_from_memory(&bytes)?;
        let (src_w, src_h) = img.dimensions();
        let rgba = img.to_rgba8();

        let decode: fn(u8, u8, u8) -> f64 = match self.encoding {
            XyzDemEncoding::Terrarium => |r, g, b| rgb_to_elevation(image::Rgb([r, g, b])),
            XyzDemEncoding::Mapbox => |r, g, b| mapbox_rgb_to_elevation(image::Rgb([r, g, b])),
        };

        let mut native = Vec::with_capacity((src_w * src_h) as usize);
        for p in rgba.pixels() {
            native.push(decode(p[0], p[1], p[2]));
        }

        let elevations = if src_w == tile_size && src_h == tile_size {
            native
        } else {
            super::resample_bilinear(&native, src_w, src_h, tile_size, tile_size)
        };

        Ok(DemTile { elevations, etag })
    }

    fn native_tile_size(&self) -> u32 {
        self.native_tile_size
    }
    fn max_zoom(&self) -> u8 {
        self.max_zoom
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn slug(&self) -> &str {
        &self.slug
    }
    fn bounds(&self) -> Option<GeoBounds> {
        self.bounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_interpolation() {
        let src = XyzDemSource::new(
            "test",
            "https://e.com/{z}/{x}/{y}.png",
            XyzDemEncoding::Terrarium,
            "v1",
            18,
            256,
            None,
        );
        assert_eq!(src.build_url(10, 100, 200), "https://e.com/10/100/200.png");
    }

    #[test]
    fn encoding_default_terrarium() {
        assert!(matches!(
            XyzDemEncoding::default(),
            XyzDemEncoding::Terrarium
        ));
    }
}
