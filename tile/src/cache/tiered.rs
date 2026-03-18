//! Tiered cache combining memory and persistent storage.

use std::future::Future;
use std::sync::Arc;

use super::memory::{CacheStats, MemoryCache};
use super::persistent::{CacheObjectMeta, PersistentCache};

/// Cache mode for persistent storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheMode {
    /// Read-write mode (default): check persistent on memory miss, write on generation.
    /// Use when this server is the primary cache source.
    #[default]
    ReadWrite,
    /// Read-only mode: check persistent on memory miss, but don't write.
    /// Use when persistent storage is read-only or managed externally.
    ReadOnly,
    /// Write-only mode: skip persistent read, only write after tile generation.
    /// Use when a CDN/Worker (e.g., Cloudflare Worker + R2) handles cache reads,
    /// and this server only generates tiles on cache miss.
    WriteOnly,
    /// None mode: disable persistent cache entirely (memory only).
    /// Persistent URL is ignored even if configured.
    None,
}

impl CacheMode {
    /// Parse cache mode from string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "read-only" | "readonly" | "read_only" => Self::ReadOnly,
            "write-only" | "writeonly" | "write_only" => Self::WriteOnly,
            "none" | "disabled" | "off" => Self::None,
            _ => Self::ReadWrite,
        }
    }

    /// Returns true if this mode allows reading from persistent cache.
    pub fn allows_read(&self) -> bool {
        matches!(self, Self::ReadWrite | Self::ReadOnly)
    }

    /// Returns true if this mode allows writing to persistent cache.
    pub fn allows_write(&self) -> bool {
        matches!(self, Self::ReadWrite | Self::WriteOnly)
    }
}

/// Two-tier cache: fast in-memory cache backed by persistent storage.
///
/// Read flow (ReadWrite mode):
/// 1. Check memory cache
/// 2. On miss, check persistent cache
/// 3. On hit from persistent, write back to memory
///
/// Read flow (WriteOnly mode):
/// 1. Check memory cache only (skip persistent)
///
/// Write flow:
/// 1. Write to memory (synchronous)
/// 2. Write to persistent (background, fire-and-forget)
pub struct TieredCache {
    memory: MemoryCache,
    persistent: Option<PersistentCache>,
    mode: CacheMode,
}

impl TieredCache {
    /// Create a new tiered cache.
    ///
    /// # Arguments
    /// * `memory_size_mb` - Size of in-memory cache in MB
    /// * `persistent_url` - Optional URL for persistent storage (file://, gs://, s3://, r2://)
    /// * `mode` - Cache mode (ReadWrite or WriteOnly)
    /// * `object_cache_control` - Optional Cache-Control header to set on stored objects
    pub fn new(
        memory_size_mb: u64,
        persistent_url: Option<&str>,
        mode: CacheMode,
        object_cache_control: Option<String>,
    ) -> Self {
        let memory = MemoryCache::new(memory_size_mb);

        let persistent =
            persistent_url.and_then(
                |url| match PersistentCache::new(url, object_cache_control) {
                    Ok(cache) => {
                        tracing::info!(
                            url = %url,
                            backend = %cache.backend(),
                            mode = ?mode,
                            "Persistent cache enabled"
                        );
                        Some(cache)
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            url = %url,
                            "Failed to initialize persistent cache, continuing with memory only"
                        );
                        None
                    }
                },
            );

        Self {
            memory,
            persistent,
            mode,
        }
    }

    /// Get a cached tile.
    ///
    /// In ReadWrite/ReadOnly mode: checks memory first, then persistent storage.
    /// In WriteOnly/None mode: checks memory only (persistent read is skipped).
    /// On persistent hit, writes back to memory for faster subsequent access.
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.get_validated(key, None).await
    }

    /// Get a cached tile with etag_hash validation.
    ///
    /// If `expected_etag_hash` is provided, validates the cached data's etag_hash
    /// against it. Returns None if the hash doesn't match (stale cache).
    ///
    /// Memory cache is assumed to be current (volatile, cleared on restart).
    /// Persistent cache is validated using stored metadata.
    pub async fn get_validated(
        &self,
        key: &str,
        expected_etag_hash: Option<&str>,
    ) -> Option<Vec<u8>> {
        // 1. Check memory cache (no validation needed - volatile)
        if let Some(data) = self.memory.get(key).await {
            tracing::info!(key = %key, size = data.len(), "Memory cache hit");
            return Some(data);
        }

        // 2. Check persistent cache (only if mode allows reading)
        if self.mode.allows_read()
            && let Some(ref persistent) = self.persistent
        {
            let backend = persistent.backend();
            match persistent.get_with_meta(key).await {
                Ok(Some((data, meta))) => {
                    // Validate etag_hash if expected
                    if let Some(expected) = expected_etag_hash {
                        let stored_hash = meta.as_ref().and_then(|m| m.etag_hash.as_deref());
                        if stored_hash != Some(expected) {
                            tracing::info!(
                                key = %key,
                                backend = %backend,
                                expected = %expected,
                                stored = ?stored_hash,
                                "Persistent cache stale (etag_hash mismatch)"
                            );
                            return None;
                        }
                    }

                    tracing::info!(key = %key, backend = %backend, size = data.len(), "Persistent cache hit");
                    // Write back to memory for faster access
                    self.memory.put(key, data.clone()).await;
                    return Some(data);
                }
                Ok(None) => {
                    tracing::info!(key = %key, backend = %backend, "Persistent cache miss");
                }
                Err(e) => {
                    tracing::warn!(
                        key = %key,
                        backend = %backend,
                        error = %e,
                        "Persistent cache read failed"
                    );
                }
            }
        }

        None
    }

    /// Put a tile in the cache.
    ///
    /// Writes to memory synchronously, then writes to persistent storage
    /// in the background (fire-and-forget) if mode allows writing.
    ///
    /// # Arguments
    /// * `key` - Cache key
    /// * `data` - Tile data
    /// * `meta` - Optional metadata (etag, content_type) to store with the object
    pub async fn put(&self, key: &str, data: Vec<u8>, meta: Option<CacheObjectMeta>) {
        // 1. Write to memory (synchronous)
        self.memory.put(key, data.clone()).await;

        // 2. Write to persistent (background, only if mode allows writing)
        if self.mode.allows_write()
            && let Some(ref persistent) = self.persistent
        {
            let persistent = persistent.clone();
            let backend = persistent.backend();
            let key = key.to_string();
            let data_len = data.len();
            tokio::spawn(async move {
                if let Err(e) = persistent.put(&key, data, meta).await {
                    tracing::warn!(
                        key = %key,
                        backend = %backend,
                        error = %e,
                        "Persistent cache write failed"
                    );
                } else {
                    tracing::info!(key = %key, backend = %backend, size = data_len, "Persistent cache write");
                }
            });
        }
    }

    /// Remove a tile from both caches.
    #[allow(dead_code)]
    pub async fn remove(&self, key: &str) {
        self.memory.remove(key).await;

        if let Some(ref persistent) = self.persistent {
            let persistent = persistent.clone();
            let key = key.to_string();
            tokio::spawn(async move {
                if let Err(e) = persistent.remove(&key).await {
                    tracing::warn!(
                        key = %key,
                        error = %e,
                        "Failed to remove from persistent cache"
                    );
                }
            });
        }
    }

    /// Clear all cached tiles from memory.
    /// Note: Does not clear persistent storage.
    #[allow(dead_code)]
    pub async fn clear(&self) {
        self.memory.clear().await;
    }

    /// Get memory cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.memory.stats()
    }

    /// Check if persistent cache is enabled.
    pub fn has_persistent(&self) -> bool {
        self.persistent.is_some()
    }

    /// Get a tile from cache or generate it with single-flight deduplication.
    ///
    /// If multiple concurrent requests come in for the same key, only one will
    /// execute the generate closure and others will wait for the result.
    ///
    /// Flow:
    /// 1. Check memory cache (moka handles single-flight here)
    /// 2. On miss, check persistent cache (with etag validation)
    /// 3. If still miss, call generate closure
    /// 4. Store result to both caches
    ///
    /// # Arguments
    /// * `key` - Cache key
    /// * `expected_etag_hash` - Expected etag hash for cache validation
    /// * `meta` - Metadata to store with persistent cache
    /// * `generate` - Closure to generate the tile if not cached
    pub async fn get_or_generate<F, Fut, E>(
        &self,
        key: &str,
        expected_etag_hash: Option<&str>,
        meta: Option<CacheObjectMeta>,
        generate: F,
    ) -> Result<Vec<u8>, Arc<E>>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<Vec<u8>, E>> + Send,
        E: Send + Sync + 'static,
    {
        // 1. Check memory cache first (for logging - moka handles actual caching)
        if let Some(data) = self.memory.get(key).await {
            tracing::info!(key = %key, size = data.len(), "Memory cache hit");
            return Ok(data);
        }

        // Capture values for use in closure
        let persistent = self.persistent.clone();
        let backend = persistent.as_ref().map(|p| p.backend());
        let mode = self.mode;
        let expected_hash = expected_etag_hash.map(|s| s.to_string());
        let key_owned = key.to_string();

        tracing::info!(key = %key, "Memory cache miss");

        self.memory
            .get_or_try_insert_with(key, || async move {
                // 2. Check persistent cache (if mode allows reading)
                if mode.allows_read()
                    && let Some(ref persistent) = persistent
                    && let Some(backend) = backend
                {
                    match persistent.get_with_meta(&key_owned).await {
                        Ok(Some((data, stored_meta))) => {
                            // Validate etag_hash if expected
                            let is_valid = match &expected_hash {
                                Some(expected) => {
                                    let stored_hash =
                                        stored_meta.as_ref().and_then(|m| m.etag_hash.as_deref());
                                    stored_hash == Some(expected.as_str())
                                }
                                None => true,
                            };

                            if is_valid {
                                tracing::info!(key = %key_owned, backend = %backend, size = data.len(), "Persistent cache hit");
                                return Ok(data);
                            } else {
                                let stored_hash =
                                    stored_meta.as_ref().and_then(|m| m.etag_hash.as_deref());
                                tracing::info!(
                                    key = %key_owned,
                                    backend = %backend,
                                    expected = ?expected_hash,
                                    stored = ?stored_hash,
                                    "Persistent cache stale (etag_hash mismatch)"
                                );
                            }
                        }
                        Ok(None) => {
                            tracing::info!(key = %key_owned, backend = %backend, "Persistent cache miss");
                        }
                        Err(e) => {
                            tracing::warn!(
                                key = %key_owned,
                                backend = %backend,
                                error = %e,
                                "Persistent cache read failed"
                            );
                        }
                    }
                }

                // 3. Generate tile
                tracing::trace!(key = %key_owned, "Generating tile (single-flight)");
                let data = generate().await?;

                // 4. Write to persistent cache (background, only if mode allows writing)
                if mode.allows_write()
                    && let Some(ref persistent) = persistent
                    && let Some(backend) = backend
                {
                    let persistent = persistent.clone();
                    let key = key_owned.clone();
                    let data_len = data.len();
                    let data = data.clone();
                    tokio::spawn(async move {
                        if let Err(e) = persistent.put(&key, data, meta).await {
                            tracing::warn!(
                                key = %key,
                                backend = %backend,
                                error = %e,
                                "Persistent cache write failed"
                            );
                        } else {
                            tracing::info!(key = %key, backend = %backend, size = data_len, "Persistent cache write");
                        }
                    });
                }

                Ok(data)
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_tiered_cache_memory_only() {
        let cache = TieredCache::new(64, None, CacheMode::default(), None);

        let key = "test/10/100/200";
        let data = vec![1, 2, 3, 4, 5];

        // Put and get
        cache.put(key, data.clone(), None).await;
        let result = cache.get(key).await;
        assert_eq!(result, Some(data));

        // Miss
        let result = cache.get("nonexistent/0/0/0").await;
        assert_eq!(result, None);

        assert!(!cache.has_persistent());
    }

    #[tokio::test]
    async fn test_tiered_cache_with_persistent() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        let cache = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);
        assert!(cache.has_persistent());

        let key = "test/10/100/200";
        let data = vec![1, 2, 3, 4, 5];

        // Put to both caches
        cache.put(key, data.clone(), None).await;

        // Wait for background write
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Create new cache (simulating restart)
        let cache2 = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);

        // Should get from persistent cache
        let result = cache2.get(key).await;
        assert_eq!(result, Some(data));
    }

    #[tokio::test]
    async fn test_tiered_cache_writeback() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        // First cache: write to persistent
        let cache1 = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);
        let key = "test/10/100/200";
        let data = vec![1, 2, 3, 4, 5];
        cache1.put(key, data.clone(), None).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Second cache: should read from persistent and write back to memory
        let cache2 = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);

        // First get: from persistent
        let result = cache2.get(key).await;
        assert_eq!(result, Some(data.clone()));

        // Second get: should now be in memory (fast path)
        let result = cache2.get(key).await;
        assert_eq!(result, Some(data));
    }

    #[tokio::test]
    async fn test_tiered_cache_write_only_mode() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        // First cache: write to persistent (in any mode, writes go to persistent)
        let cache1 = TieredCache::new(64, Some(&url), CacheMode::WriteOnly, None);
        let key = "test/10/100/200";
        let data = vec![1, 2, 3, 4, 5];
        cache1.put(key, data.clone(), None).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Second cache in WriteOnly mode: should NOT read from persistent
        let cache2 = TieredCache::new(64, Some(&url), CacheMode::WriteOnly, None);
        let result = cache2.get(key).await;
        assert_eq!(result, None); // Miss because WriteOnly skips persistent read

        // Third cache in ReadWrite mode: should read from persistent
        let cache3 = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);
        let result = cache3.get(key).await;
        assert_eq!(result, Some(data)); // Hit because ReadWrite checks persistent
    }

    #[tokio::test]
    async fn test_tiered_cache_read_only_mode() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        // First, write data using ReadWrite mode
        let cache1 = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);
        let key = "test/10/100/200";
        let data = vec![1, 2, 3, 4, 5];
        cache1.put(key, data.clone(), None).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Second cache in ReadOnly mode: should read from persistent
        let cache2 = TieredCache::new(64, Some(&url), CacheMode::ReadOnly, None);
        let result = cache2.get(key).await;
        assert_eq!(result, Some(data.clone())); // Hit because ReadOnly reads persistent

        // Try to write with ReadOnly mode
        let key2 = "test/10/100/201";
        let data2 = vec![6, 7, 8, 9, 10];
        cache2.put(key2, data2.clone(), None).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // New cache should NOT find key2 in persistent (ReadOnly doesn't write)
        let cache3 = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);
        let result = cache3.get(key2).await;
        assert_eq!(result, None); // Miss because ReadOnly didn't write to persistent
    }

    #[tokio::test]
    async fn test_tiered_cache_none_mode() {
        let temp_dir = TempDir::new().unwrap();
        let url = format!("file://{}", temp_dir.path().display());

        // First, write data using ReadWrite mode
        let cache1 = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);
        let key = "test/10/100/200";
        let data = vec![1, 2, 3, 4, 5];
        cache1.put(key, data.clone(), None).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Cache with None mode: should NOT read from persistent
        let cache2 = TieredCache::new(64, Some(&url), CacheMode::None, None);
        let result = cache2.get(key).await;
        assert_eq!(result, None); // Miss because None mode skips persistent

        // Write with None mode
        let key2 = "test/10/100/201";
        let data2 = vec![6, 7, 8, 9, 10];
        cache2.put(key2, data2.clone(), None).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // New cache should NOT find key2 in persistent (None mode doesn't write)
        let cache3 = TieredCache::new(64, Some(&url), CacheMode::ReadWrite, None);
        let result = cache3.get(key2).await;
        assert_eq!(result, None); // Miss because None mode didn't write to persistent

        // But cache2 should still have it in memory
        let result = cache2.get(key2).await;
        assert_eq!(result, Some(data2)); // Hit from memory
    }

    #[test]
    fn test_cache_mode_parse() {
        assert_eq!(CacheMode::parse("read-write"), CacheMode::ReadWrite);
        assert_eq!(CacheMode::parse("readwrite"), CacheMode::ReadWrite);
        assert_eq!(CacheMode::parse("READ-WRITE"), CacheMode::ReadWrite);

        assert_eq!(CacheMode::parse("read-only"), CacheMode::ReadOnly);
        assert_eq!(CacheMode::parse("readonly"), CacheMode::ReadOnly);
        assert_eq!(CacheMode::parse("READ_ONLY"), CacheMode::ReadOnly);

        assert_eq!(CacheMode::parse("write-only"), CacheMode::WriteOnly);
        assert_eq!(CacheMode::parse("writeonly"), CacheMode::WriteOnly);
        assert_eq!(CacheMode::parse("WRITE_ONLY"), CacheMode::WriteOnly);

        assert_eq!(CacheMode::parse("none"), CacheMode::None);
        assert_eq!(CacheMode::parse("disabled"), CacheMode::None);
        assert_eq!(CacheMode::parse("off"), CacheMode::None);
        assert_eq!(CacheMode::parse("NONE"), CacheMode::None);

        // Unknown defaults to ReadWrite
        assert_eq!(CacheMode::parse("unknown"), CacheMode::ReadWrite);
        assert_eq!(CacheMode::parse(""), CacheMode::ReadWrite);
    }

    #[test]
    fn test_cache_mode_helpers() {
        assert!(CacheMode::ReadWrite.allows_read());
        assert!(CacheMode::ReadWrite.allows_write());

        assert!(CacheMode::ReadOnly.allows_read());
        assert!(!CacheMode::ReadOnly.allows_write());

        assert!(!CacheMode::WriteOnly.allows_read());
        assert!(CacheMode::WriteOnly.allows_write());

        assert!(!CacheMode::None.allows_read());
        assert!(!CacheMode::None.allows_write());
    }

    #[tokio::test]
    async fn test_single_flight_deduplication() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache = TieredCache::new(64, None, CacheMode::default(), None);
        let key = "single-flight/test";

        // Counter to track how many times the generate function is called
        let call_count = Arc::new(AtomicUsize::new(0));

        // Spawn 10 concurrent requests for the same key
        let mut handles = Vec::new();
        for _ in 0..10 {
            let cache = &cache;
            let call_count = call_count.clone();
            handles.push(async move {
                cache
                    .get_or_generate::<_, _, std::io::Error>(key, None, None, || {
                        let call_count = call_count.clone();
                        async move {
                            // Increment counter
                            call_count.fetch_add(1, Ordering::SeqCst);
                            // Simulate slow generation
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            Ok(vec![1, 2, 3, 4, 5])
                        }
                    })
                    .await
            });
        }

        // Wait for all requests to complete
        let results: Vec<_> = futures::future::join_all(handles).await;

        // All requests should succeed with the same data
        for result in &results {
            assert!(result.is_ok());
            assert_eq!(result.as_ref().unwrap(), &vec![1, 2, 3, 4, 5]);
        }

        // The generate function should only be called once due to single-flight
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "Generate function should only be called once"
        );
    }

    #[tokio::test]
    async fn test_get_or_generate_basic() {
        let cache = TieredCache::new(64, None, CacheMode::default(), None);
        let key = "gen/test";

        // First call generates
        let result = cache
            .get_or_generate::<_, _, std::io::Error>(key, None, None, || async {
                Ok(vec![1, 2, 3])
            })
            .await;
        assert_eq!(result.unwrap(), vec![1, 2, 3]);

        // Second call returns cached value (doesn't call generate)
        let result = cache
            .get_or_generate::<_, _, std::io::Error>(key, None, None, || async {
                Ok(vec![9, 9, 9]) // Different data, but shouldn't be used
            })
            .await;
        assert_eq!(result.unwrap(), vec![1, 2, 3]); // Still returns original
    }

    #[tokio::test]
    async fn test_get_or_generate_with_error() {
        let cache = TieredCache::new(64, None, CacheMode::default(), None);
        let key = "gen/error";

        // Generate returns error
        let result = cache
            .get_or_generate::<_, _, std::io::Error>(key, None, None, || async {
                Err(std::io::Error::other("test error"))
            })
            .await;
        assert!(result.is_err());

        // Error should not be cached - next call tries again
        let result = cache
            .get_or_generate::<_, _, std::io::Error>(key, None, None, || async {
                Ok(vec![1, 2, 3])
            })
            .await;
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }
}
