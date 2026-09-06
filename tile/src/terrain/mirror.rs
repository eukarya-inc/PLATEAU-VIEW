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
use object_store::{
    ObjectStore,
    path::{Path as ObjectPath, PathPart},
};

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

    /// `{prefix}/{sub}` as an object key.
    ///
    /// `self.prefix` already holds the **decoded** segments (see
    /// `crate::object_url`), and `object_store` percent-encodes a key once when
    /// it builds the request URL. Re-running the prefix through
    /// `ObjectPath::from` would encode it a second time, so it is composed from
    /// its already-parsed parts; only `sub` — a plain string that was never
    /// encoded — goes through `PathPart`. For an ASCII prefix this is
    /// byte-identical to the previous `ObjectPath::from(format!(..))` form.
    fn key_for(&self, sub: &str) -> ObjectPath {
        ObjectPath::from_iter(
            self.prefix
                .parts()
                .chain(sub.split('/').map(PathPart::from)),
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a mirror with an arbitrary prefix, bypassing `from_url` so the
    /// test doesn't need R2 credentials.
    fn mirror_with_prefix(prefix: ObjectPath) -> MirrorSource {
        MirrorSource {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix,
            source_url: "memory://test".to_string(),
        }
    }

    /// A non-ASCII prefix must stay decoded in the key; `object_store` encodes
    /// it exactly once per request.
    #[test]
    fn key_for_non_ascii_prefix_is_single_encoded() {
        let prefix = ObjectPath::from_url_path("/terrain/静岡（葵）").unwrap();
        let mirror = mirror_with_prefix(prefix.clone());

        let path = mirror.key_for("9/455/203.terrain");
        assert_eq!(path.as_ref(), "terrain/静岡（葵）/9/455/203.terrain");
        assert!(!path.as_ref().contains('%'));
        // The old form re-encoded the already-decoded prefix.
        assert_ne!(
            path,
            ObjectPath::from(format!("{}/9/455/203.terrain", prefix.as_ref()))
        );
    }

    /// ASCII prefixes (everything used in production, e.g.
    /// `plateau-terrain-2024`) must be byte-identical to the previous
    /// `ObjectPath::from(format!(..))` behaviour.
    #[test]
    fn key_for_ascii_prefix_matches_old_behaviour() {
        let prefix = ObjectPath::from("plateau-terrain-2024");
        let mirror = mirror_with_prefix(prefix.clone());

        for sub in ["layer.json", "9/455/203.terrain"] {
            assert_eq!(
                mirror.key_for(sub).as_ref(),
                ObjectPath::from(format!("{}/{sub}", prefix.as_ref())).as_ref()
            );
        }

        // ...including the empty-prefix branch.
        let root = mirror_with_prefix(ObjectPath::from(""));
        assert_eq!(
            root.key_for("layer.json").as_ref(),
            ObjectPath::from("layer.json").as_ref()
        );
    }
}
