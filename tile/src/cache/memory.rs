//! In-memory tile cache using moka.

use std::future::Future;
use std::sync::Arc;

use moka::future::Cache;

/// In-memory tile cache using moka for fast access.
pub struct MemoryCache {
    cache: Cache<String, Vec<u8>>,
}

impl MemoryCache {
    /// Create a new memory cache with the specified size in MB.
    pub fn new(size_mb: u64) -> Self {
        // Estimate: average tile is ~50KB, so calculate max entries
        let estimated_tile_size = 50 * 1024; // 50KB
        let max_entries = (size_mb * 1024 * 1024) / estimated_tile_size;

        let cache = Cache::builder().max_capacity(max_entries).build();

        tracing::info!(
            "Memory cache initialized: {}MB (~{} tiles)",
            size_mb,
            max_entries
        );

        Self { cache }
    }

    /// Get a cached tile.
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.get(key).await
    }

    /// Put a tile in the cache.
    pub async fn put(&self, key: &str, data: Vec<u8>) {
        self.cache.insert(key.to_string(), data).await;
    }

    /// Get or generate a tile with single-flight deduplication.
    ///
    /// If multiple concurrent requests come in for the same key,
    /// only one will execute the init closure and others will wait.
    pub async fn get_or_try_insert_with<F, Fut, E>(
        &self,
        key: &str,
        init: F,
    ) -> Result<Vec<u8>, Arc<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Vec<u8>, E>>,
        E: Send + Sync + 'static,
    {
        self.cache
            .try_get_with(key.to_string(), async { init().await })
            .await
    }

    /// Remove a tile from the cache.
    #[allow(dead_code)]
    pub async fn remove(&self, key: &str) {
        self.cache.remove(key).await;
    }

    /// Clear all cached tiles.
    #[allow(dead_code)]
    pub async fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.cache.entry_count(),
            weighted_size: self.cache.weighted_size(),
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: u64,
    pub weighted_size: u64,
}
