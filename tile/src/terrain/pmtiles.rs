//! PMTiles DEM source.
//!
//! Reads a PMTiles v3 archive and exposes its tiles as a [`DemProvider`].
//! Backed by the `object_store` crate (v0.13, see Cargo.toml note about
//! coexistence with the v0.12 used by `crate::cog`), so the URL can point at:
//!
//! - `https://...` — any HTTPS host (R2 public, GCS public, custom CDN, …)
//! - `gs://bucket/key` — Google Cloud Storage (uses ADC /
//!   `GOOGLE_APPLICATION_CREDENTIALS`; works for private buckets too)
//! - `s3://bucket/key` — AWS S3 (or any S3-compatible service)
//! - `r2://bucket/key` — Cloudflare R2 (S3-compatible; needs `R2_ACCOUNT_ID`,
//!   `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`)
//! - `file:///path/to.pmtiles` — local file
//!
//! The archive's HTTP / object ETag is fetched once on init and mixed into
//! every tile's downstream cache key, so updating the archive in place
//! invalidates serving caches without a CDN partial purge.

use std::sync::Arc;

use async_trait::async_trait;
use image::GenericImageView;
use object_store_pmtiles::aws::AmazonS3Builder;
use object_store_pmtiles::client::ClientConfigKey;
use object_store_pmtiles::gcp::GoogleCloudStorageBuilder;
use object_store_pmtiles::http::HttpBuilder;
use object_store_pmtiles::local::LocalFileSystem;
use object_store_pmtiles::path::Path as ObjectPath;
use object_store_pmtiles::{GetOptions, ObjectStore};
use pmtiles::{AsyncPmTilesReader, ObjectStoreBackend, TileCoord};
use serde::Deserialize;
use tokio::sync::OnceCell;
use url::Url;

use super::dem::{DemError, DemProvider, DemTile, GeoBounds};
use super::mapbox::mapbox_rgb_to_elevation;
use super::terrarium::rgb_to_elevation;

/// Encoding used by the PMTiles archive's tile payloads.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PmtilesEncoding {
    /// Terrarium (Mapterhorn / AWS-style): `h = (R*256 + G + B/256) - 32768`.
    #[default]
    Terrarium,
    /// Mapbox Terrain-RGB v1: `h = -10000 + ((R*65536 + G*256 + B) * 0.1)`.
    Mapbox,
}

pub struct PmtilesSource {
    url: String,
    encoding: PmtilesEncoding,
    max_zoom: u8,
    native_tile_size: u32,
    version: String,
    slug: String,
    reader: OnceCell<Arc<AsyncPmTilesReader<ObjectStoreBackend>>>,
    archive_etag: OnceCell<Option<String>>,
    bounds_cell: OnceCell<Option<GeoBounds>>,
}

impl PmtilesSource {
    pub fn new(
        url: impl Into<String>,
        encoding: PmtilesEncoding,
        version: impl Into<String>,
        max_zoom: u8,
        native_tile_size: u32,
    ) -> Self {
        Self {
            url: url.into(),
            encoding,
            max_zoom,
            native_tile_size,
            version: version.into(),
            slug: "pmtiles".to_string(),
            reader: OnceCell::new(),
            archive_etag: OnceCell::new(),
            bounds_cell: OnceCell::new(),
        }
    }

    async fn reader(&self) -> Result<Arc<AsyncPmTilesReader<ObjectStoreBackend>>, DemError> {
        self.reader
            .get_or_try_init(|| async {
                let (store, path) = build_object_store(&self.url)?;
                let backend = ObjectStoreBackend::new(store, path);
                AsyncPmTilesReader::try_from_source(backend)
                    .await
                    .map(Arc::new)
                    .map_err(|e| DemError::Http(format!("pmtiles open: {e}")))
            })
            .await
            .cloned()
    }

    /// Resolve and cache the upstream archive ETag (via object_store HEAD).
    /// Falls back to `None` if the backend doesn't surface one.
    async fn archive_etag(&self) -> &Option<String> {
        self.archive_etag
            .get_or_init(|| async {
                let (store, path) = match build_object_store(&self.url) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(error=%e, "pmtiles HEAD: build_object_store failed");
                        return None;
                    }
                };
                let opts = GetOptions {
                    head: true,
                    ..Default::default()
                };
                match store.get_opts(&path, opts).await {
                    Ok(result) => result
                        .meta
                        .e_tag
                        .as_ref()
                        .map(|s: &String| s.trim_start_matches("W/").trim_matches('"').to_string())
                        .filter(|s| !s.is_empty()),
                    Err(e) => {
                        tracing::warn!(error=%e, "pmtiles HEAD failed; ETag unavailable");
                        None
                    }
                }
            })
            .await
    }
}

#[async_trait]
impl DemProvider for PmtilesSource {
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

        let reader = self.reader().await?;
        let coord = TileCoord::new(z, x, y)
            .map_err(|e| DemError::Decode(format!("invalid tile coord {z}/{x}/{y}: {e}")))?;

        let bytes: bytes::Bytes = reader
            .get_tile_decompressed(coord)
            .await
            .map_err(|e| DemError::Http(format!("pmtiles get_tile: {e}")))?
            .ok_or(DemError::NotFound)?;

        let img = image::load_from_memory(&bytes)?;
        let (src_w, src_h) = img.dimensions();
        let rgba = img.to_rgba8();

        let decode: fn(u8, u8, u8) -> f64 = match self.encoding {
            PmtilesEncoding::Terrarium => {
                |r: u8, g: u8, b: u8| rgb_to_elevation(image::Rgb([r, g, b]))
            }
            PmtilesEncoding::Mapbox => {
                |r: u8, g: u8, b: u8| mapbox_rgb_to_elevation(image::Rgb([r, g, b]))
            }
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

        let etag = self
            .archive_etag()
            .await
            .clone()
            .or_else(|| Some(self.version.clone()));

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

    async fn preload(&self) -> Result<(), DemError> {
        // Initializes reader (which parses header) + archive ETag + bounds.
        let reader = self.reader().await?;
        let header = reader.get_header();
        let bounds = GeoBounds::new(
            header.min_longitude,
            header.min_latitude,
            header.max_longitude,
            header.max_latitude,
        );
        let _ = self.bounds_cell.set(Some(bounds));
        let _ = self.archive_etag().await;
        Ok(())
    }

    fn bounds(&self) -> Option<GeoBounds> {
        self.bounds_cell.get().and_then(|b| *b)
    }
}

/// Build an `(ObjectStore, ObjectPath)` pair for the URL scheme. Public so
/// the raster `tile::PmtilesTileSource` can share it.
pub fn build_object_store_for(
    url: &str,
) -> Result<(Box<dyn ObjectStore>, ObjectPath), crate::tile::TileError> {
    build_object_store(url).map_err(|e| crate::tile::TileError::Internal(e.to_string()))
}

/// Build an `(ObjectStore, ObjectPath)` pair for the URL scheme.
fn build_object_store(url: &str) -> Result<(Box<dyn ObjectStore>, ObjectPath), DemError> {
    let parsed =
        Url::parse(url).map_err(|e| DemError::Decode(format!("invalid pmtiles URL: {e}")))?;

    match parsed.scheme() {
        "http" | "https" => {
            let base = format!(
                "{}://{}{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or(""),
                parsed.port().map(|p| format!(":{p}")).unwrap_or_default(),
            );
            let store = HttpBuilder::new()
                .with_url(&base)
                .with_config(ClientConfigKey::AllowHttp, "true")
                .build()
                .map_err(|e| DemError::Http(format!("HTTP store init: {e}")))?;
            Ok((Box::new(store), ObjectPath::from(parsed.path())))
        }
        "gs" => {
            let bucket = parsed
                .host_str()
                .ok_or_else(|| DemError::Decode("gs:// URL missing bucket".to_string()))?;
            let store = GoogleCloudStorageBuilder::from_env()
                .with_bucket_name(bucket)
                .build()
                .map_err(|e| DemError::Http(format!("GCS store init: {e}")))?;
            Ok((Box::new(store), ObjectPath::from(parsed.path())))
        }
        "s3" => {
            let bucket = parsed
                .host_str()
                .ok_or_else(|| DemError::Decode("s3:// URL missing bucket".to_string()))?;
            let store = AmazonS3Builder::from_env()
                .with_bucket_name(bucket)
                .build()
                .map_err(|e| DemError::Http(format!("S3 store init: {e}")))?;
            Ok((Box::new(store), ObjectPath::from(parsed.path())))
        }
        "r2" => {
            // R2 is S3-compatible. R2_ACCOUNT_ID is required to derive the
            // endpoint; access keys are read from R2_ACCESS_KEY_ID /
            // R2_SECRET_ACCESS_KEY.
            let bucket = parsed
                .host_str()
                .ok_or_else(|| DemError::Decode("r2:// URL missing bucket".to_string()))?;
            let account = std::env::var("R2_ACCOUNT_ID")
                .map_err(|_| DemError::Decode("r2:// requires R2_ACCOUNT_ID".to_string()))?;
            let endpoint = format!("https://{account}.r2.cloudflarestorage.com");
            let mut builder = AmazonS3Builder::new()
                .with_bucket_name(bucket)
                .with_endpoint(endpoint)
                .with_region("auto")
                .with_allow_http(false);
            if let Ok(k) = std::env::var("R2_ACCESS_KEY_ID") {
                builder = builder.with_access_key_id(k);
            }
            if let Ok(s) = std::env::var("R2_SECRET_ACCESS_KEY") {
                builder = builder.with_secret_access_key(s);
            }
            let store = builder
                .build()
                .map_err(|e| DemError::Http(format!("R2 store init: {e}")))?;
            Ok((Box::new(store), ObjectPath::from(parsed.path())))
        }
        "file" => {
            let path = parsed
                .to_file_path()
                .map_err(|_| DemError::Decode("file:// URL is not a valid path".to_string()))?;
            let parent = path
                .parent()
                .ok_or_else(|| DemError::Decode("file:// URL has no parent dir".to_string()))?;
            let file_name = path
                .file_name()
                .ok_or_else(|| DemError::Decode("file:// URL has no filename".to_string()))?;
            let store = LocalFileSystem::new_with_prefix(parent)
                .map_err(|e| DemError::Http(format!("Local FS store init: {e}")))?;
            Ok((
                Box::new(store),
                ObjectPath::from(file_name.to_string_lossy().as_ref()),
            ))
        }
        other => Err(DemError::Decode(format!(
            "unsupported pmtiles URL scheme: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_default_is_terrarium() {
        assert!(matches!(
            PmtilesEncoding::default(),
            PmtilesEncoding::Terrarium
        ));
    }

    #[test]
    fn rejects_unknown_scheme() {
        let res = build_object_store("ftp://example.com/x.pmtiles");
        assert!(res.is_err());
    }
}
