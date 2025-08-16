//! # elif-cache
//! 
//! A comprehensive multi-backend caching system for the elif.rs framework.
//! 
//! ## Features
//! 
//! - **Multi-backend support**: Memory, Redis, and file-based caching
//! - **Cache tagging**: Group related cache entries for batch invalidation
//! - **TTL support**: Time-based cache expiration
//! - **Async-first**: Built for modern async Rust applications
//! - **Type-safe**: Generic cache operations with serialization support
//! - **HTTP integration**: Response caching utilities for web applications
//! 
//! ## Quick Start
//! 
//! ```rust
//! use elif_cache::{Cache, MemoryBackend, CacheConfig};
//! use std::time::Duration;
//! 
//! # tokio_test::block_on(async {
//! // Create a memory-based cache
//! let cache = Cache::new(MemoryBackend::new(CacheConfig::default()));
//! 
//! // Store a value
//! cache.put("user:123", &"John Doe".to_string(), Duration::from_secs(3600)).await.unwrap();
//! 
//! // Retrieve a value
//! let user: Option<String> = cache.get("user:123").await.unwrap();
//! assert_eq!(user, Some("John Doe".to_string()));
//! 
//! // Use the remember pattern
//! let expensive_data = cache.remember(
//!     "expensive:computation",
//!     Duration::from_secs(3600),
//!     || async { "computed value".to_string() }
//! ).await.unwrap();
//! # });
//! ```

use std::time::Duration;
use serde::{Serialize, de::DeserializeOwned};
use async_trait::async_trait;
use thiserror::Error;

pub mod backends;
pub mod config;
pub mod tagging;
pub mod invalidation;

#[cfg(feature = "http-cache")]
pub mod http_cache;

pub use backends::*;
pub use config::*;
pub use tagging::*;
pub use invalidation::*;

/// Cache operation errors
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Backend error: {0}")]
    Backend(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Cache configuration error: {0}")]
    Configuration(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Timeout error")]
    Timeout,
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, CacheError>;

/// Cache key type
pub type CacheKey = String;

/// Cache tags for grouping related entries
pub type CacheTag = String;

/// Core cache backend trait that all cache implementations must implement
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// Get a value from the cache
    async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>>;
    
    /// Put a value in the cache with optional TTL
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> CacheResult<()>;
    
    /// Remove a value from the cache
    async fn forget(&self, key: &str) -> CacheResult<bool>;
    
    /// Check if a key exists in the cache
    async fn exists(&self, key: &str) -> CacheResult<bool>;
    
    /// Clear all entries from the cache
    async fn flush(&self) -> CacheResult<()>;
    
    /// Get multiple values at once (optional optimization)
    async fn get_many(&self, keys: &[&str]) -> CacheResult<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }
    
    /// Put multiple values at once (optional optimization)
    async fn put_many(&self, entries: &[(&str, Vec<u8>, Option<Duration>)]) -> CacheResult<()> {
        for (key, value, ttl) in entries {
            self.put(key, value.clone(), *ttl).await?;
        }
        Ok(())
    }
    
    /// Remove multiple values from the cache (optional optimization)
    async fn forget_many(&self, keys: &[&str]) -> CacheResult<usize> {
        let mut removed_count = 0;
        for key in keys {
            if self.forget(key).await? {
                removed_count += 1;
            }
        }
        Ok(removed_count)
    }
    
    /// Get cache statistics (if supported)
    async fn stats(&self) -> CacheResult<CacheStats> {
        Ok(CacheStats::default())
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_keys: u64,
    pub memory_usage: u64,
}

impl CacheStats {
    pub fn hit_ratio(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// High-level cache interface with type-safe operations
pub struct Cache<B: CacheBackend> {
    backend: B,
    default_ttl: Option<Duration>,
}

impl<B: CacheBackend> Cache<B> {
    /// Create a new cache instance with the given backend
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            default_ttl: None,
        }
    }
    
    /// Create a new cache instance with a default TTL
    pub fn with_default_ttl(backend: B, ttl: Duration) -> Self {
        Self {
            backend,
            default_ttl: Some(ttl),
        }
    }
    
    /// Get a typed value from the cache
    pub async fn get<T>(&self, key: &str) -> CacheResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        match self.backend.get(key).await? {
            Some(bytes) => {
                let value = serde_json::from_slice(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
    
    /// Put a typed value in the cache
    pub async fn put<T>(&self, key: &str, value: &T, ttl: Duration) -> CacheResult<()>
    where
        T: Serialize,
    {
        let bytes = serde_json::to_vec(value)?;
        self.backend.put(key, bytes, Some(ttl)).await
    }
    
    /// Put a typed value in the cache using default TTL
    pub async fn put_default<T>(&self, key: &str, value: &T) -> CacheResult<()>
    where
        T: Serialize,
    {
        let bytes = serde_json::to_vec(value)?;
        self.backend.put(key, bytes, self.default_ttl).await
    }
    
    /// Remove a value from the cache
    pub async fn forget(&self, key: &str) -> CacheResult<bool> {
        self.backend.forget(key).await
    }
    
    /// Check if a key exists
    pub async fn exists(&self, key: &str) -> CacheResult<bool> {
        self.backend.exists(key).await
    }
    
    /// Clear all cache entries
    pub async fn flush(&self) -> CacheResult<()> {
        self.backend.flush().await
    }
    
    /// Remember pattern: get from cache or compute and store
    pub async fn remember<T, F, Fut>(&self, key: &str, ttl: Duration, compute: F) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        if let Some(cached) = self.get(key).await? {
            return Ok(cached);
        }
        
        let value = compute().await;
        self.put(key, &value, ttl).await?;
        Ok(value)
    }
    
    /// Remember pattern with default TTL
    pub async fn remember_default<T, F, Fut>(&self, key: &str, compute: F) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        if let Some(cached) = self.get(key).await? {
            return Ok(cached);
        }
        
        let value = compute().await;
        
        if let Some(ttl) = self.default_ttl {
            self.put(key, &value, ttl).await?;
        } else {
            return Err(CacheError::Configuration("No default TTL configured".to_string()));
        }
        
        Ok(value)
    }
    
    /// Get cache statistics
    pub async fn stats(&self) -> CacheResult<CacheStats> {
        self.backend.stats().await
    }
}

/// Global cache instance (can be set once during application initialization)
static GLOBAL_CACHE: std::sync::OnceLock<Box<dyn CacheBackend>> = std::sync::OnceLock::new();

/// Set the global cache instance
pub fn set_global_cache<B: CacheBackend + 'static>(backend: B) -> Result<(), Box<dyn CacheBackend>> {
    GLOBAL_CACHE.set(Box::new(backend))
}

/// Get the global cache instance
pub fn global_cache() -> Option<&'static Box<dyn CacheBackend>> {
    GLOBAL_CACHE.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::MemoryBackend;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_cache_basic_operations() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        // Test put and get
        cache.put("test_key", &"test_value", Duration::from_secs(60)).await.unwrap();
        let value: Option<String> = cache.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
        
        // Test exists
        assert!(cache.exists("test_key").await.unwrap());
        assert!(!cache.exists("nonexistent").await.unwrap());
        
        // Test forget
        assert!(cache.forget("test_key").await.unwrap());
        let value: Option<String> = cache.get("test_key").await.unwrap();
        assert_eq!(value, None);
    }
    
    #[tokio::test]
    async fn test_cache_remember_pattern() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        
        // First call should compute
        let result1 = cache.remember("remember_test", Duration::from_secs(60), move || {
            let count = call_count_clone.fetch_add(1, Ordering::Relaxed) + 1;
            async move { format!("computed_{}", count) }
        }).await.unwrap();
        assert_eq!(result1, "computed_1");
        
        // Second call should use cache
        let result2 = cache.remember("remember_test", Duration::from_secs(60), || async { "should_not_be_called".to_string() }).await.unwrap();
        assert_eq!(result2, "computed_1");
        
        // Verify compute function was called only once
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }
    
    #[tokio::test]
    async fn test_cache_with_default_ttl() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::with_default_ttl(backend, Duration::from_secs(3600));
        
        cache.put_default("default_ttl_test", &42i32).await.unwrap();
        let value: Option<i32> = cache.get("default_ttl_test").await.unwrap();
        assert_eq!(value, Some(42));
    }
}