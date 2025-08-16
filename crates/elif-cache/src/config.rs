//! Cache configuration and builder

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Cache configuration for all backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default TTL for cache entries
    pub default_ttl: Option<Duration>,
    
    /// Maximum number of entries (for memory backend)
    pub max_entries: Option<usize>,
    
    /// Memory limit in bytes (for memory backend)
    pub max_memory: Option<usize>,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Operation timeout
    pub operation_timeout: Duration,
    
    /// Enable compression for large values
    pub compression: bool,
    
    /// Compression threshold (compress values larger than this)
    pub compression_threshold: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Some(Duration::from_secs(3600)), // 1 hour
            max_entries: Some(10_000),
            max_memory: Some(100 * 1024 * 1024), // 100MB
            connection_timeout: Duration::from_secs(5),
            operation_timeout: Duration::from_secs(1),
            compression: false,
            compression_threshold: 1024, // 1KB
        }
    }
}

impl CacheConfig {
    pub fn builder() -> CacheConfigBuilder {
        CacheConfigBuilder::default()
    }
}

/// Cache configuration builder
#[derive(Debug, Default)]
pub struct CacheConfigBuilder {
    config: CacheConfig,
}

impl CacheConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.config.default_ttl = Some(ttl);
        self
    }
    
    pub fn no_default_ttl(mut self) -> Self {
        self.config.default_ttl = None;
        self
    }
    
    pub fn max_entries(mut self, max: usize) -> Self {
        self.config.max_entries = Some(max);
        self
    }
    
    pub fn unlimited_entries(mut self) -> Self {
        self.config.max_entries = None;
        self
    }
    
    pub fn max_memory(mut self, bytes: usize) -> Self {
        self.config.max_memory = Some(bytes);
        self
    }
    
    pub fn unlimited_memory(mut self) -> Self {
        self.config.max_memory = None;
        self
    }
    
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.config.connection_timeout = timeout;
        self
    }
    
    pub fn operation_timeout(mut self, timeout: Duration) -> Self {
        self.config.operation_timeout = timeout;
        self
    }
    
    pub fn enable_compression(mut self, threshold: usize) -> Self {
        self.config.compression = true;
        self.config.compression_threshold = threshold;
        self
    }
    
    pub fn disable_compression(mut self) -> Self {
        self.config.compression = false;
        self
    }
    
    pub fn build(self) -> CacheConfig {
        self.config
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
            .default_ttl(Duration::from_secs(7200))
            .max_entries(5000)
            .enable_compression(2048)
            .build();
            
        assert_eq!(config.default_ttl, Some(Duration::from_secs(7200)));
        assert_eq!(config.max_entries, Some(5000));
        assert!(config.compression);
        assert_eq!(config.compression_threshold, 2048);
    }
}