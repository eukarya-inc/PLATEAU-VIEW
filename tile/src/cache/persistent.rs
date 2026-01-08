//! Persistent tile cache using object_store.

use std::sync::Arc;

use bytes::Bytes;
use object_store::{ObjectStore, path::Path as ObjectPath};
use thiserror::Error;

use super::store::{CacheStoreError, CacheStoreFactory};

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

/// Persistent tile cache backed by object storage.
#[derive(Clone)]
pub struct PersistentCache {
    store: Arc<dyn ObjectStore>,
    prefix: ObjectPath,
}

impl PersistentCache {
    /// Create a new persistent cache from a URL.
    ///
    /// Supported URL schemes:
    /// - `file:///path/to/cache` - Local filesystem
    /// - `gs://bucket/prefix` - Google Cloud Storage
    /// - `s3://bucket/prefix` - Amazon S3
    /// - `r2://bucket/prefix` - Cloudflare R2
    pub fn new(url: &str) -> Result<Self, PersistentCacheError> {
        let (store, prefix) = CacheStoreFactory::create(url)?;
        Ok(Self { store, prefix })
    }

    /// Convert a cache key to an object path.
    fn key_to_path(&self, key: &str) -> ObjectPath {
        // Key format: {source}/{z}/{x}/{y}
        // Path format: {prefix}/{source}/{z}/{x}/{y}.png
        if self.prefix.as_ref().is_empty() {
            ObjectPath::from(format!("{key}.png"))
        } else {
            ObjectPath::from(format!("{}/{key}.png", self.prefix.as_ref()))
        }
    }

    /// Get a cached tile.
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, PersistentCacheError> {
        let path = self.key_to_path(key);

        match self.store.get(&path).await {
            Ok(result) => {
                let bytes = result
                    .bytes()
                    .await
                    .map_err(|e| PersistentCacheError::ReadError(e.to_string()))?;
                Ok(Some(bytes.to_vec()))
            }
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(PersistentCacheError::ReadError(e.to_string())),
        }
    }

    /// Put a tile in the cache.
    pub async fn put(&self, key: &str, data: Vec<u8>) -> Result<(), PersistentCacheError> {
        let path = self.key_to_path(key);
        let bytes = Bytes::from(data);

        self.store
            .put(&path, bytes.into())
            .await
            .map_err(|e| PersistentCacheError::WriteError(e.to_string()))?;

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

        let cache = PersistentCache::new(&url).unwrap();

        // Test put and get
        let key = "test-source/10/100/200";
        let data = vec![1, 2, 3, 4, 5];

        cache.put(key, data.clone()).await.unwrap();

        let result = cache.get(key).await.unwrap();
        assert_eq!(result, Some(data));

        // Test cache miss
        let result = cache.get("nonexistent/0/0/0").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_key_to_path() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        let cache = PersistentCache::new(&url).unwrap();

        let path = cache.key_to_path("source/10/100/200");
        assert_eq!(path.as_ref(), "source/10/100/200.png");
    }
}
