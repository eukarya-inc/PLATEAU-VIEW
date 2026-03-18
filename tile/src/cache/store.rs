//! ObjectStore factory for cache storage backends.

use std::sync::Arc;

use object_store::{
    ObjectStore, aws::AmazonS3Builder, gcp::GoogleCloudStorageBuilder, local::LocalFileSystem,
    path::Path as ObjectPath,
};
use thiserror::Error;
use url::Url;

/// Errors related to cache storage operations.
#[derive(Error, Debug)]
pub enum CacheStoreError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Unsupported scheme: {0}")]
    UnsupportedScheme(String),
    #[error("Missing configuration: {0}")]
    MissingConfig(String),
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Cache storage backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheBackend {
    /// Local filesystem
    File,
    /// Google Cloud Storage
    Gcs,
    /// Amazon S3
    S3,
    /// Cloudflare R2
    R2,
}

impl std::fmt::Display for CacheBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheBackend::File => write!(f, "file"),
            CacheBackend::Gcs => write!(f, "gcs"),
            CacheBackend::S3 => write!(f, "s3"),
            CacheBackend::R2 => write!(f, "r2"),
        }
    }
}

/// Factory for creating ObjectStore instances from URLs.
pub struct CacheStoreFactory;

impl CacheStoreFactory {
    /// Create an ObjectStore from a URL.
    ///
    /// Supported schemes:
    /// - `file://` - Local filesystem
    /// - `gs://` - Google Cloud Storage
    /// - `s3://` - Amazon S3
    /// - `r2://` - Cloudflare R2 (S3-compatible)
    ///
    /// Returns (store, path_prefix, backend_type).
    pub fn create(
        url: &str,
    ) -> Result<(Arc<dyn ObjectStore>, ObjectPath, CacheBackend), CacheStoreError> {
        let parsed = Url::parse(url).map_err(|e| CacheStoreError::InvalidUrl(e.to_string()))?;

        match parsed.scheme() {
            "file" => Self::create_local_store(&parsed).map(|(s, p)| (s, p, CacheBackend::File)),
            "gs" => Self::create_gcs_store(&parsed).map(|(s, p)| (s, p, CacheBackend::Gcs)),
            "s3" => Self::create_s3_store(&parsed).map(|(s, p)| (s, p, CacheBackend::S3)),
            "r2" => Self::create_r2_store(&parsed).map(|(s, p)| (s, p, CacheBackend::R2)),
            scheme => Err(CacheStoreError::UnsupportedScheme(scheme.to_string())),
        }
    }

    /// Create a local filesystem store.
    fn create_local_store(
        url: &Url,
    ) -> Result<(Arc<dyn ObjectStore>, ObjectPath), CacheStoreError> {
        let path = url.path();

        // Create the directory if it doesn't exist
        std::fs::create_dir_all(path).map_err(|e| {
            CacheStoreError::StorageError(format!("Failed to create cache directory: {e}"))
        })?;

        let store = LocalFileSystem::new_with_prefix(path).map_err(|e| {
            CacheStoreError::StorageError(format!("Failed to create local store: {e}"))
        })?;

        Ok((Arc::new(store), ObjectPath::from("")))
    }

    /// Create a Google Cloud Storage store.
    fn create_gcs_store(url: &Url) -> Result<(Arc<dyn ObjectStore>, ObjectPath), CacheStoreError> {
        let bucket = url
            .host_str()
            .ok_or_else(|| CacheStoreError::InvalidUrl("Missing GCS bucket".to_string()))?;

        let store = GoogleCloudStorageBuilder::new()
            .with_bucket_name(bucket)
            .build()
            .map_err(|e| {
                CacheStoreError::StorageError(format!("Failed to create GCS store: {e}"))
            })?;

        let prefix = ObjectPath::from(url.path().trim_start_matches('/'));

        Ok((Arc::new(store), prefix))
    }

    /// Create an Amazon S3 store.
    fn create_s3_store(url: &Url) -> Result<(Arc<dyn ObjectStore>, ObjectPath), CacheStoreError> {
        let bucket = url
            .host_str()
            .ok_or_else(|| CacheStoreError::InvalidUrl("Missing S3 bucket".to_string()))?;

        let store = AmazonS3Builder::new()
            .with_bucket_name(bucket)
            .build()
            .map_err(|e| {
                CacheStoreError::StorageError(format!("Failed to create S3 store: {e}"))
            })?;

        let prefix = ObjectPath::from(url.path().trim_start_matches('/'));

        Ok((Arc::new(store), prefix))
    }

    /// Create a Cloudflare R2 store (S3-compatible).
    fn create_r2_store(url: &Url) -> Result<(Arc<dyn ObjectStore>, ObjectPath), CacheStoreError> {
        let bucket = url
            .host_str()
            .ok_or_else(|| CacheStoreError::InvalidUrl("Missing R2 bucket".to_string()))?;

        // Get R2-specific configuration from environment variables
        let account_id = std::env::var("R2_ACCOUNT_ID").map_err(|_| {
            CacheStoreError::MissingConfig("R2_ACCOUNT_ID is required for R2".to_string())
        })?;
        let access_key = std::env::var("R2_ACCESS_KEY_ID").map_err(|_| {
            CacheStoreError::MissingConfig("R2_ACCESS_KEY_ID is required for R2".to_string())
        })?;
        let secret_key = std::env::var("R2_SECRET_ACCESS_KEY").map_err(|_| {
            CacheStoreError::MissingConfig("R2_SECRET_ACCESS_KEY is required for R2".to_string())
        })?;

        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);

        let store = AmazonS3Builder::new()
            .with_bucket_name(bucket)
            .with_endpoint(endpoint)
            .with_access_key_id(access_key)
            .with_secret_access_key(secret_key)
            .with_region("auto")
            .build()
            .map_err(|e| {
                CacheStoreError::StorageError(format!("Failed to create R2 store: {e}"))
            })?;

        let prefix = ObjectPath::from(url.path().trim_start_matches('/'));

        Ok((Arc::new(store), prefix))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsupported_scheme() {
        let result = CacheStoreFactory::create("ftp://example.com/cache");
        assert!(matches!(result, Err(CacheStoreError::UnsupportedScheme(_))));
    }

    #[test]
    fn test_invalid_url() {
        let result = CacheStoreFactory::create("not a url");
        assert!(matches!(result, Err(CacheStoreError::InvalidUrl(_))));
    }
}
