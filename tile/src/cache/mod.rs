//! Tile caching module.
//!
//! Provides two-tier caching:
//! - **Memory cache**: Fast in-memory cache using moka
//! - **Persistent cache**: Durable storage using object_store (file, GCS, S3, R2)

mod memory;
mod persistent;
mod store;
mod tiered;

pub use memory::{CacheStats, MemoryCache};
pub use persistent::{PersistentCache, PersistentCacheError};
pub use store::{CacheStoreError, CacheStoreFactory};
pub use tiered::{CacheMode, TieredCache};

// Re-export TieredCache as TileCache for backward compatibility
pub type TileCache = TieredCache;
