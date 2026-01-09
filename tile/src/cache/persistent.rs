//! Persistent tile cache using object_store.

use std::borrow::Cow;
use std::sync::Arc;

use bytes::Bytes;
use object_store::{Attribute, Attributes, ObjectStore, PutOptions, path::Path as ObjectPath};
use thiserror::Error;

use super::store::{CacheStoreError, CacheStoreFactory};

/// Custom metadata key for etag_hash.
const ETAG_HASH_META_KEY: &str = "etag_hash";
/// Custom metadata key for full ETag.
const ETAG_META_KEY: &str = "etag";

/// Errors related to persistent cache operations.
#[derive(Error, Debug)]
pub enum PersistentCacheError {
    #[error("Store creation error: {0}")]
    StoreError(#[from] CacheStoreError),
    #[error("Read error: {0}")]
    ReadError(String),
    #[error("Write error: {0}")]
    WriteError(String),
}

/// Metadata to attach to cached objects.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CacheObjectMeta {
    /// Content-Type for the object (e.g., "image/png")
    pub content_type: Option<String>,
    /// Hash of etag_keys for cache invalidation
    pub etag_hash: Option<String>,
    /// Full ETag for HTTP caching (e.g., W/"abc123")
    pub etag: Option<String>,
}

/// Persistent tile cache backed by object storage.
#[derive(Clone)]
pub struct PersistentCache {
    store: Arc<dyn ObjectStore>,
    prefix: ObjectPath,
    /// Cache-Control header to set on stored objects
    cache_control: Option<String>,
}

impl PersistentCache {
    /// Create a new persistent cache from a URL.
    ///
    /// Supported URL schemes:
    /// - `file:///path/to/cache` - Local filesystem
    /// - `gs://bucket/prefix` - Google Cloud Storage
    /// - `s3://bucket/prefix` - Amazon S3
    /// - `r2://bucket/prefix` - Cloudflare R2
    ///
    /// # Arguments
    /// * `url` - Storage URL
    /// * `cache_control` - Optional Cache-Control header to set on stored objects
    pub fn new(url: &str, cache_control: Option<String>) -> Result<Self, PersistentCacheError> {
        let (store, prefix) = CacheStoreFactory::create(url)?;
        Ok(Self {
            store,
            prefix,
            cache_control,
        })
    }

    /// Convert a cache key to an object path.
    fn key_to_path(&self, key: &str) -> ObjectPath {
        // Key format: {source}/{format}/{z}/{x}/{y}.{ext}
        // Path format: {prefix}/{source}/{format}/{z}/{x}/{y}.{ext}
        if self.prefix.as_ref().is_empty() {
            ObjectPath::from(key)
        } else {
            ObjectPath::from(format!("{}/{key}", self.prefix.as_ref()))
        }
    }

    /// Get a cached tile with its metadata.
    ///
    /// Metadata is read from native object attributes (x-amz-meta-* for S3/R2).
    pub async fn get_with_meta(
        &self,
        key: &str,
    ) -> Result<Option<(Vec<u8>, Option<CacheObjectMeta>)>, PersistentCacheError> {
        let path = self.key_to_path(key);

        // Get the data and attributes
        let result = match self.store.get(&path).await {
            Ok(r) => r,
            Err(object_store::Error::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(PersistentCacheError::ReadError(e.to_string())),
        };

        // Extract metadata from attributes
        let etag_hash = result
            .attributes
            .get(&Attribute::Metadata(Cow::Borrowed(ETAG_HASH_META_KEY)))
            .map(|v| v.to_string());
        let etag = result
            .attributes
            .get(&Attribute::Metadata(Cow::Borrowed(ETAG_META_KEY)))
            .map(|v| v.to_string());

        let meta = if etag_hash.is_some() || etag.is_some() {
            Some(CacheObjectMeta {
                content_type: None, // Content-Type is handled by HTTP headers
                etag_hash,
                etag,
            })
        } else {
            None
        };

        let bytes = result
            .bytes()
            .await
            .map_err(|e| PersistentCacheError::ReadError(e.to_string()))?;

        Ok(Some((bytes.to_vec(), meta)))
    }

    /// Get a cached tile (without metadata, for backward compatibility).
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, PersistentCacheError> {
        Ok(self.get_with_meta(key).await?.map(|(data, _)| data))
    }

    /// Put a tile in the cache with optional metadata.
    ///
    /// Metadata is stored as native object attributes (x-amz-meta-* for S3/R2).
    pub async fn put(
        &self,
        key: &str,
        data: Vec<u8>,
        meta: Option<CacheObjectMeta>,
    ) -> Result<(), PersistentCacheError> {
        let path = self.key_to_path(key);
        let bytes = Bytes::from(data);

        // Build attributes from cache_control and metadata
        let mut attrs = Attributes::new();

        if let Some(ref cc) = self.cache_control {
            attrs.insert(Attribute::CacheControl, cc.clone().into());
        }

        if let Some(ref meta) = meta {
            if let Some(ref ct) = meta.content_type {
                attrs.insert(Attribute::ContentType, ct.clone().into());
            }
            // Store etag_hash as custom metadata (x-amz-meta-etag_hash for S3/R2)
            if let Some(ref eh) = meta.etag_hash {
                attrs.insert(
                    Attribute::Metadata(Cow::Borrowed(ETAG_HASH_META_KEY)),
                    eh.clone().into(),
                );
            }
            // Store full ETag as custom metadata (x-amz-meta-etag for S3/R2)
            if let Some(ref et) = meta.etag {
                attrs.insert(
                    Attribute::Metadata(Cow::Borrowed(ETAG_META_KEY)),
                    et.clone().into(),
                );
            }
        }

        // Try put_opts with attributes first; fallback to regular put if not supported
        // (LocalFileSystem doesn't support attributes)
        let data_written = if !attrs.is_empty() {
            let opts = PutOptions {
                attributes: attrs,
                ..Default::default()
            };

            match self.store.put_opts(&path, bytes.clone().into(), opts).await {
                Ok(_) => true,
                Err(e) if e.to_string().contains("not yet implemented") => {
                    // Fallback to regular put for stores that don't support attributes
                    tracing::debug!("put_opts not supported, falling back to put");
                    false
                }
                Err(e) => return Err(PersistentCacheError::WriteError(e.to_string())),
            }
        } else {
            false
        };

        // Regular put without attributes (if not already written)
        if !data_written {
            self.store
                .put(&path, bytes.into())
                .await
                .map_err(|e| PersistentCacheError::WriteError(e.to_string()))?;
        }

        Ok(())
    }

    /// Remove a tile from the cache.
    #[allow(dead_code)]
    pub async fn remove(&self, key: &str) -> Result<(), PersistentCacheError> {
        let path = self.key_to_path(key);

        match self.store.delete(&path).await {
            Ok(()) => Ok(()),
            Err(object_store::Error::NotFound { .. }) => Ok(()), // Already deleted
            Err(e) => Err(PersistentCacheError::WriteError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_persistent_cache_file() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        let cache = PersistentCache::new(&url, None).unwrap();

        // Test put and get (key includes format and extension)
        let key = "test-source/png/10/100/200.png";
        let data = vec![1, 2, 3, 4, 5];

        cache.put(key, data.clone(), None).await.unwrap();

        let result = cache.get(key).await.unwrap();
        assert_eq!(result, Some(data));

        // Test cache miss
        let result = cache.get("nonexistent/png/0/0/0.png").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_persistent_cache_with_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        let cache =
            PersistentCache::new(&url, Some("public, max-age=31536000".to_string())).unwrap();

        let key = "test-source/png/10/100/200.png";
        let data = vec![1, 2, 3, 4, 5];
        let meta = CacheObjectMeta {
            content_type: Some("image/png".to_string()),
            ..Default::default()
        };

        cache.put(key, data.clone(), Some(meta)).await.unwrap();

        let result = cache.get(key).await.unwrap();
        assert_eq!(result, Some(data));
    }

    #[tokio::test]
    async fn test_key_to_path() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        let cache = PersistentCache::new(&url, None).unwrap();

        // Key now includes format and extension
        let path = cache.key_to_path("source/webp/10/100/200.webp");
        assert_eq!(path.as_ref(), "source/webp/10/100/200.webp");
    }
}
