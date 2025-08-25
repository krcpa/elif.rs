//! Queue configuration types and builders

use serde::{Deserialize, Serialize};
use service_builder::builder;
use std::time::Duration;

/// Configuration for queue backends
#[derive(Debug, Clone, Serialize, Deserialize)]
#[builder]
pub struct QueueConfig {
    /// Maximum number of concurrent workers
    #[builder(default = "4", getter)]
    pub max_workers: usize,

    /// Default job timeout
    #[builder(default = "Duration::from_secs(300)", getter)]
    pub default_timeout: Duration,

    /// Polling interval for checking new jobs
    #[builder(default = "Duration::from_millis(100)", getter)]
    pub poll_interval: Duration,

    /// Maximum queue size (0 = unlimited)
    #[builder(default = "0", getter)]
    pub max_queue_size: usize,

    /// Enable job persistence
    #[builder(default = "true", getter)]
    pub enable_persistence: bool,

    /// Cleanup interval for completed/failed jobs
    #[builder(default = "Duration::from_secs(3600)")]
    pub cleanup_interval: Duration,

    /// How long to keep completed jobs before cleanup
    #[builder(default = "Duration::from_secs(86400)")]
    pub completed_job_ttl: Duration,

    /// How long to keep failed jobs before cleanup
    #[builder(default = "Duration::from_secs(604800)")]
    pub failed_job_ttl: Duration,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_workers: 4,
            default_timeout: Duration::from_secs(300),
            poll_interval: Duration::from_millis(100),
            max_queue_size: 0,
            enable_persistence: true,
            cleanup_interval: Duration::from_secs(3600),
            completed_job_ttl: Duration::from_secs(86400), // 1 day
            failed_job_ttl: Duration::from_secs(604800),   // 1 week
        }
    }
}

/// Redis-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[builder]
pub struct RedisConfig {
    /// Redis connection URL
    #[builder(default = "\"redis://localhost:6379\".to_string()", getter)]
    pub url: String,

    /// Connection pool size
    #[builder(default = "10", getter)]
    pub pool_size: u32,

    /// Connection timeout
    #[builder(default = "Duration::from_secs(5)")]
    pub connect_timeout: Duration,

    /// Command timeout
    #[builder(default = "Duration::from_secs(30)")]
    pub command_timeout: Duration,

    /// Queue key prefix
    #[builder(default = "\"elif_queue\".to_string()")]
    pub key_prefix: String,

    /// Enable Redis persistence
    #[builder(default = "true")]
    pub enable_persistence: bool,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            connect_timeout: Duration::from_secs(5),
            command_timeout: Duration::from_secs(30),
            key_prefix: "elif_queue".to_string(),
            enable_persistence: true,
        }
    }
}

impl QueueConfigBuilder {
    /// Create a development configuration with fast polling and small timeouts
    pub fn development() -> Self {
        QueueConfigBuilder::new()
            .max_workers(2)
            .poll_interval(Duration::from_millis(50))
            .default_timeout(Duration::from_secs(60))
            .enable_persistence(false)
    }

    /// Create a production configuration with conservative defaults
    pub fn production() -> Self {
        QueueConfigBuilder::new()
            .max_workers(8)
            .poll_interval(Duration::from_millis(500))
            .default_timeout(Duration::from_secs(600))
            .enable_persistence(true)
            .cleanup_interval(Duration::from_secs(1800)) // 30 minutes
    }

    /// Create a testing configuration with minimal overhead
    pub fn testing() -> Self {
        QueueConfigBuilder::new()
            .max_workers(1)
            .poll_interval(Duration::from_millis(10))
            .default_timeout(Duration::from_secs(10))
            .enable_persistence(false)
            .cleanup_interval(Duration::from_secs(1))
            .completed_job_ttl(Duration::from_secs(1))
            .failed_job_ttl(Duration::from_secs(1))
    }
}

impl RedisConfigBuilder {
    /// Create a development Redis configuration
    pub fn development() -> Self {
        RedisConfigBuilder::new()
            .url("redis://localhost:6379".to_string())
            .pool_size(5)
            .key_prefix("elif_queue_dev".to_string())
    }

    /// Create a production Redis configuration
    pub fn production() -> Self {
        RedisConfigBuilder::new()
            .pool_size(20)
            .connect_timeout(Duration::from_secs(10))
            .command_timeout(Duration::from_secs(60))
            .key_prefix("elif_queue_prod".to_string())
    }

    /// Create a testing Redis configuration
    pub fn testing() -> Self {
        RedisConfigBuilder::new()
            .url("redis://localhost:6379".to_string())
            .pool_size(2)
            .key_prefix("elif_queue_test".to_string())
            .enable_persistence(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_queue_config() {
        let config = QueueConfig::default();
        assert_eq!(config.max_workers, 4);
        assert_eq!(config.default_timeout, Duration::from_secs(300));
        assert_eq!(config.poll_interval, Duration::from_millis(100));
        assert!(config.enable_persistence);
    }

    #[test]
    fn test_queue_config_builder() {
        let config = QueueConfigBuilder::new()
            .max_workers(8)
            .default_timeout(Duration::from_secs(600))
            .build()
            .expect("Failed to build config");

        assert_eq!(*config.get_max_workers(), 8);
        assert_eq!(*config.get_default_timeout(), Duration::from_secs(600));
        assert_eq!(*config.get_poll_interval(), Duration::from_millis(100)); // Default
    }

    #[test]
    fn test_development_config() {
        let config = QueueConfigBuilder::development()
            .build()
            .expect("Failed to build config");
        assert_eq!(*config.get_max_workers(), 2);
        assert_eq!(*config.get_poll_interval(), Duration::from_millis(50));
        assert!(!*config.get_enable_persistence());
    }

    #[test]
    fn test_production_config() {
        let config = QueueConfigBuilder::production()
            .build()
            .expect("Failed to build config");
        assert_eq!(*config.get_max_workers(), 8);
        assert_eq!(*config.get_poll_interval(), Duration::from_millis(500));
        assert!(*config.get_enable_persistence());
    }

    #[test]
    fn test_redis_config_builder() {
        let config = RedisConfigBuilder::new()
            .url("redis://custom:6380".to_string())
            .pool_size(15)
            .build()
            .expect("Failed to build config");

        assert_eq!(config.get_url(), "redis://custom:6380");
        assert_eq!(*config.get_pool_size(), 15);
    }
}
