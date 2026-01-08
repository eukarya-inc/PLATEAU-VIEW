//! Tiered cache combining memory and persistent storage.

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
            persistent_url.and_then(|url| match PersistentCache::new(url, object_cache_control) {
                Ok(cache) => {
                    tracing::info!(
                        url = %url,
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
            });

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
        // 1. Check memory cache
        if let Some(data) = self.memory.get(key).await {
            tracing::trace!(key = %key, "Memory cache hit");
            return Some(data);
        }

        // 2. Check persistent cache (only if mode allows reading)
        if self.mode.allows_read()
            && let Some(ref persistent) = self.persistent
        {
            match persistent.get(key).await {
                Ok(Some(data)) => {
                    tracing::trace!(key = %key, "Persistent cache hit");
                    // Write back to memory for faster access
                    self.memory.put(key, data.clone()).await;
                    return Some(data);
                }
                Ok(None) => {
                    tracing::trace!(key = %key, "Persistent cache miss");
                }
                Err(e) => {
                    tracing::warn!(
                        key = %key,
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
            let key = key.to_string();
            tokio::spawn(async move {
                if let Err(e) = persistent.put(&key, data, meta).await {
                    tracing::warn!(
                        key = %key,
                        error = %e,
                        "Failed to write to persistent cache"
                    );
                } else {
                    tracing::trace!(key = %key, "Written to persistent cache");
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
}
