//! Pre-rendered quantized-mesh mirror backed by object storage.
//!
//! Reads tiles + layer.json directly from an S3-compatible bucket populated
//! out-of-band by the `ion-terrain-mirror` crawler. No DEM, no Martini, no
//! geoid composition — the bytes are exactly what Cesium expects, served
//! pass-through with `Content-Encoding: gzip` preserved.
//!
//! The bucket URL is configured via `TERRAIN_MIRROR_URL` (e.g.
//! `r2://plateau-terrain/plateau-terrain-2024/`). Layout under the prefix
//! matches the Ion mirror exactly:
//!
//! ```text
//! {prefix}/layer.json
//! {prefix}/{z}/{x}/{y}.terrain   (gzip-compressed quantized-mesh-1.0)
//! ```

use std::sync::Arc;

use bytes::Bytes;
use object_store::{ObjectStore, path::Path as ObjectPath};

use crate::cache::{CacheStoreError, CacheStoreFactory};

/// One pre-rendered terrain mirror backed by object storage.
pub struct MirrorSource {
    store: Arc<dyn ObjectStore>,
    prefix: ObjectPath,
    /// Original URL string, kept for logging only.
    pub source_url: String,
}

impl MirrorSource {
    /// Build a mirror source from a URL understood by [`CacheStoreFactory`]
    /// (`file://`, `gs://`, `s3://`, `r2://`).
    pub fn from_url(url: &str) -> Result<Self, CacheStoreError> {
        let (store, prefix, _backend) = CacheStoreFactory::create(url)?;
        Ok(Self {
            store,
            prefix,
            source_url: url.to_string(),
        })
    }

    fn key_for(&self, sub: &str) -> ObjectPath {
        let prefix = self.prefix.as_ref();
        if prefix.is_empty() {
            ObjectPath::from(sub)
        } else {
            ObjectPath::from(format!("{prefix}/{sub}"))
        }
    }

    /// Fetch a single tile. `Ok(None)` for a 404 (tile not in mirror).
    pub async fn fetch_tile(
        &self,
        z: u32,
        x: u32,
        y: u32,
    ) -> Result<Option<Bytes>, object_store::Error> {
        let path = self.key_for(&format!("{z}/{x}/{y}.terrain"));
        match self.store.get(&path).await {
            Ok(r) => Ok(Some(r.bytes().await?)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Fetch the upstream `layer.json` verbatim. `Ok(None)` if it's missing
    /// (in which case the handler will synthesize a default that still
    /// points tiles at this mirror).
    pub async fn fetch_layer_json(&self) -> Result<Option<Bytes>, object_store::Error> {
        let path = self.key_for("layer.json");
        match self.store.get(&path).await {
            Ok(r) => Ok(Some(r.bytes().await?)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
