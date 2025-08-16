//! Cache configuration and builder

use std::time::Duration;
use serde::{Deserialize, Serialize};
use service_builder::builder;

/// Cache configuration for all backends
#[derive(Debug, Clone, Serialize, Deserialize)]
#[builder]
pub struct CacheConfig {
    /// Default TTL for cache entries
    #[builder(getter, default = "Some(Duration::from_secs(3600))")]
    pub default_ttl: Option<Duration>,
    
    /// Maximum number of entries (for memory backend)
    #[builder(getter, default = "Some(10_000)")]
    pub max_entries: Option<usize>,
    
    /// Memory limit in bytes (for memory backend)
    #[builder(getter, default = "Some(100 * 1024 * 1024)")]
    pub max_memory: Option<usize>,
    
    /// Connection timeout
    #[builder(default = "Duration::from_secs(5)")]
    pub connection_timeout: Duration,
    
    /// Operation timeout
    #[builder(default = "Duration::from_secs(1)")]
    pub operation_timeout: Duration,
    
    /// Enable compression for large values
    #[builder(default)]
    pub compression: bool,
    
    /// Compression threshold (compress values larger than this)
    #[builder(default = "1024")]
    pub compression_threshold: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        // Use the builder with defaults to ensure consistency
        CacheConfig::builder()
            .default_ttl(Some(Duration::from_secs(3600))) // 1 hour
            .max_entries(Some(10_000))
            .max_memory(Some(100 * 1024 * 1024)) // 100MB
            .build()
            .unwrap()
    }
}

// Add convenience methods to the generated builder
impl CacheConfigBuilder {
    pub fn default_ttl_duration(self, ttl: Duration) -> Self {
        self.default_ttl(Some(ttl))
    }
    
    pub fn no_default_ttl(self) -> Self {
        self.default_ttl(None)
    }
    
    pub fn max_entries_limit(self, max: usize) -> Self {
        self.max_entries(Some(max))
    }
    
    pub fn unlimited_entries(self) -> Self {
        self.max_entries(None)
    }
    
    pub fn max_memory_bytes(self, bytes: usize) -> Self {
        self.max_memory(Some(bytes))
    }
    
    pub fn unlimited_memory(self) -> Self {
        self.max_memory(None)
    }
    
    pub fn enable_compression(self, threshold: usize) -> Self {
        self.compression(true).compression_threshold(threshold)
    }
    
    pub fn disable_compression(self) -> Self {
        self.compression(false)
    }
    
    pub fn build_config(self) -> CacheConfig {
        self.build_with_defaults().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert_eq!(config.default_ttl, Some(Duration::from_secs(3600)));
        assert_eq!(config.max_entries, Some(10_000));
        assert!(!config.compression);
    }
    
    #[test]
    fn test_config_builder() {
        let config = CacheConfig::builder()
            .default_ttl_duration(Duration::from_secs(7200))
            .max_entries_limit(5000)
            .enable_compression(2048)
            .build_config();
            
        assert_eq!(config.default_ttl, Some(Duration::from_secs(7200)));
        assert_eq!(config.max_entries, Some(5000));
        assert!(config.compression);
        assert_eq!(config.compression_threshold, 2048);
    }
}