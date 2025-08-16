//! Redis cache backend with connection pooling and failover

use crate::{CacheBackend, CacheConfig, CacheError, CacheResult, CacheStats};
use async_trait::async_trait;
use redis::{AsyncCommands, Client, Connection};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

/// Redis connection pool configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    
    /// Connection pool size
    pub pool_size: u32,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Command timeout
    pub command_timeout: Duration,
    
    /// Key prefix for all cache keys
    pub key_prefix: Option<String>,
    
    /// Enable compression for large values
    pub compression: bool,
    
    /// Compression threshold in bytes
    pub compression_threshold: usize,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            connection_timeout: Duration::from_secs(5),
            command_timeout: Duration::from_secs(1),
            key_prefix: Some("elif_cache:".to_string()),
            compression: false,
            compression_threshold: 1024,
        }
    }
}

impl RedisConfig {
    pub fn builder() -> RedisConfigBuilder {
        RedisConfigBuilder::default()
    }
}

/// Redis configuration builder
#[derive(Debug, Default)]
pub struct RedisConfigBuilder {
    config: RedisConfig,
}

impl RedisConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn url<S: Into<String>>(mut self, url: S) -> Self {
        self.config.url = url.into();
        self
    }
    
    pub fn pool_size(mut self, size: u32) -> Self {
        self.config.pool_size = size;
        self
    }
    
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.config.connection_timeout = timeout;
        self
    }
    
    pub fn command_timeout(mut self, timeout: Duration) -> Self {
        self.config.command_timeout = timeout;
        self
    }
    
    pub fn key_prefix<S: Into<String>>(mut self, prefix: Option<S>) -> Self {
        self.config.key_prefix = prefix.map(|p| p.into());
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
    
    pub fn build(self) -> RedisConfig {
        self.config
    }
}

/// Redis connection manager
struct RedisConnectionManager {
    client: Client,
    connections: Arc<Mutex<Vec<redis::aio::Connection>>>,
    config: RedisConfig,
}

impl RedisConnectionManager {
    async fn new(config: RedisConfig) -> Result<Self, CacheError> {
        let client = Client::open(config.url.as_str())
            .map_err(|e| CacheError::Backend(format!("Failed to create Redis client: {}", e)))?;
        
        // Test connection
        let mut test_conn = client
            .get_async_connection()
            .await
            .map_err(|e| CacheError::Backend(format!("Failed to connect to Redis: {}", e)))?;
        
        // Ping to verify connection
        let _: String = test_conn
            .ping()
            .await
            .map_err(|e| CacheError::Backend(format!("Redis ping failed: {}", e)))?;
        
        debug!("Redis connection established successfully");
        
        Ok(Self {
            client,
            connections: Arc::new(Mutex::new(vec![])),
            config,
        })
    }
    
    async fn get_connection(&self) -> Result<redis::aio::Connection, CacheError> {
        // Try to reuse an existing connection
        {
            let mut connections = self.connections.lock().await;
            if let Some(conn) = connections.pop() {
                return Ok(conn);
            }
        }
        
        // Create a new connection
        let conn = tokio::time::timeout(
            self.config.connection_timeout,
            self.client.get_async_connection(),
        )
        .await
        .map_err(|_| CacheError::Timeout)?
        .map_err(|e| CacheError::Backend(format!("Failed to get Redis connection: {}", e)))?;
        
        Ok(conn)
    }
    
    async fn return_connection(&self, conn: redis::aio::Connection) {
        let mut connections = self.connections.lock().await;
        if connections.len() < self.config.pool_size as usize {
            connections.push(conn);
        }
        // If pool is full, just drop the connection
    }
}

/// Redis cache backend
pub struct RedisBackend {
    connection_manager: RedisConnectionManager,
    stats: Arc<Mutex<CacheStats>>,
}

impl RedisBackend {
    /// Create a new Redis backend with the given configuration
    pub async fn new(config: RedisConfig) -> CacheResult<Self> {
        let connection_manager = RedisConnectionManager::new(config).await?;
        
        Ok(Self {
            connection_manager,
            stats: Arc::new(Mutex::new(CacheStats::default())),
        })
    }
    
    /// Create a Redis backend from URL
    pub async fn from_url<S: Into<String>>(url: S) -> CacheResult<Self> {
        let config = RedisConfig {
            url: url.into(),
            ..Default::default()
        };
        Self::new(config).await
    }
    
    /// Format key with optional prefix
    fn format_key(&self, key: &str) -> String {
        match &self.connection_manager.config.key_prefix {
            Some(prefix) => format!("{}{}", prefix, key),
            None => key.to_string(),
        }
    }
    
    /// Execute a Redis command with timeout and connection management
    async fn with_connection<F, Fut, R>(&self, operation: F) -> CacheResult<R>
    where
        F: FnOnce(redis::aio::Connection) -> Fut,
        Fut: std::future::Future<Output = (redis::RedisResult<R>, redis::aio::Connection)>,
        R: Send + 'static,
    {
        let conn = self.connection_manager.get_connection().await?;
        
        let (redis_result, conn) = tokio::time::timeout(
            self.connection_manager.config.command_timeout,
            operation(conn),
        )
        .await
        .map_err(|_| CacheError::Timeout)?;
        
        // Return connection to pool
        self.connection_manager.return_connection(conn).await;
        
        redis_result.map_err(|e| CacheError::Backend(format!("Redis operation failed: {}", e)))
    }
    
    /// Compress data if enabled and above threshold
    fn maybe_compress(&self, data: &[u8]) -> Vec<u8> {
        if self.connection_manager.config.compression 
            && data.len() > self.connection_manager.config.compression_threshold 
        {
            // Simple compression using gzip would go here
            // For now, just return the original data
            debug!("Compression enabled but not implemented yet");
        }
        data.to_vec()
    }
    
    /// Decompress data if it was compressed
    fn maybe_decompress(&self, data: &[u8]) -> CacheResult<Vec<u8>> {
        // For now, assume no compression
        Ok(data.to_vec())
    }
}

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
        let formatted_key = self.format_key(key);
        
        let result = self
            .with_connection(|mut conn| async move {
                let result = conn.get::<String, Option<Vec<u8>>>(formatted_key).await;
                (result, conn)
            })
            .await;
        
        match result {
            Ok(Some(data)) => {
                // Update stats
                self.stats.lock().await.hits += 1;
                
                // Decompress if necessary
                let decompressed = self.maybe_decompress(&data)?;
                Ok(Some(decompressed))
            }
            Ok(None) => {
                // Update stats
                self.stats.lock().await.misses += 1;
                Ok(None)
            }
            Err(e) => {
                error!("Redis GET error for key '{}': {}", key, e);
                self.stats.lock().await.misses += 1;
                Err(e)
            }
        }
    }
    
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> CacheResult<()> {
        let formatted_key = self.format_key(key);
        let compressed_value = self.maybe_compress(&value);
        
        let result = self
            .with_connection(|mut conn| async move {
                let result = match ttl {
                    Some(ttl) => {
                        conn.set_ex::<String, Vec<u8>, ()>(formatted_key, compressed_value, ttl.as_secs()).await
                    }
                    None => {
                        conn.set::<String, Vec<u8>, ()>(formatted_key, compressed_value).await
                    }
                };
                (result, conn)
            })
            .await;
        
        match result {
            Ok(()) => {
                debug!("Successfully cached key: {}", key);
                Ok(())
            }
            Err(e) => {
                error!("Redis SET error for key '{}': {}", key, e);
                Err(e)
            }
        }
    }
    
    async fn forget(&self, key: &str) -> CacheResult<bool> {
        let formatted_key = self.format_key(key);
        
        let result = self
            .with_connection(|mut conn| async move {
                let result = conn.del::<String, i32>(formatted_key).await;
                (result, conn)
            })
            .await;
        
        match result {
            Ok(count) => {
                let removed = count > 0;
                if removed {
                    debug!("Successfully removed key: {}", key);
                }
                Ok(removed)
            }
            Err(e) => {
                error!("Redis DEL error for key '{}': {}", key, e);
                Err(e)
            }
        }
    }
    
    async fn exists(&self, key: &str) -> CacheResult<bool> {
        let formatted_key = self.format_key(key);
        
        let result = self
            .with_connection(|mut conn| async move {
                let result = conn.exists::<String, bool>(formatted_key).await;
                (result, conn)
            })
            .await;
        
        match result {
            Ok(exists) => Ok(exists),
            Err(e) => {
                error!("Redis EXISTS error for key '{}': {}", key, e);
                Err(e)
            }
        }
    }
    
    async fn flush(&self) -> CacheResult<()> {
        let prefix = self.connection_manager.config.key_prefix.clone();
        
        let result = self
            .with_connection(|mut conn| async move {
                let result = match prefix {
                    Some(prefix) => {
                        // Delete only keys with our prefix
                        let pattern = format!("{}*", prefix);
                        let keys: Vec<String> = conn.keys(pattern).await?;
                        if !keys.is_empty() {
                            conn.del::<Vec<String>, ()>(keys).await?;
                        }
                        Ok(())
                    }
                    None => {
                        // Flush entire database (dangerous!)
                        warn!("Flushing entire Redis database - no prefix configured");
                        conn.flushdb().await
                    }
                };
                (result, conn)
            })
            .await;
        
        match result {
            Ok(()) => {
                // Reset stats
                let mut stats = self.stats.lock().await;
                stats.total_keys = 0;
                debug!("Cache flushed successfully");
                Ok(())
            }
            Err(e) => {
                error!("Redis FLUSH error: {}", e);
                Err(e)
            }
        }
    }
    
    async fn get_many(&self, keys: &[&str]) -> CacheResult<Vec<Option<Vec<u8>>>> {
        if keys.is_empty() {
            return Ok(vec![]);
        }
        
        let formatted_keys: Vec<String> = keys.iter().map(|k| self.format_key(k)).collect();
        
        let result = self
            .with_connection(|mut conn| async move {
                let result = conn.mget::<Vec<String>, Vec<Option<Vec<u8>>>>(formatted_keys).await;
                (result, conn)
            })
            .await;
        
        match result {
            Ok(values) => {
                // Update stats
                let mut stats = self.stats.lock().await;
                for value in &values {
                    if value.is_some() {
                        stats.hits += 1;
                    } else {
                        stats.misses += 1;
                    }
                }
                
                // Decompress values if necessary
                let mut decompressed_values = Vec::with_capacity(values.len());
                for value in values {
                    match value {
                        Some(data) => {
                            let decompressed = self.maybe_decompress(&data)?;
                            decompressed_values.push(Some(decompressed));
                        }
                        None => decompressed_values.push(None),
                    }
                }
                
                Ok(decompressed_values)
            }
            Err(e) => {
                error!("Redis MGET error: {}", e);
                // Update miss stats for all keys
                self.stats.lock().await.misses += keys.len() as u64;
                Err(e)
            }
        }
    }
    
    async fn put_many(&self, entries: &[(&str, Vec<u8>, Option<Duration>)]) -> CacheResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        
        // Redis doesn't have a native MSET with different TTLs, so we use a pipeline
        for (key, value, ttl) in entries {
            self.put(key, value.clone(), *ttl).await?;
        }
        
        Ok(())
    }
    
    async fn stats(&self) -> CacheResult<CacheStats> {
        let stats = self.stats.lock().await.clone();
        
        // Try to get additional stats from Redis INFO command
        let redis_info = self
            .with_connection(|mut conn| async move {
                let result = conn.info::<String>("memory").await;
                (result, conn)
            })
            .await;
        
        match redis_info {
            Ok(info) => {
                // Parse memory usage from info (simplified)
                let memory_usage = info
                    .lines()
                    .find(|line| line.starts_with("used_memory:"))
                    .and_then(|line| line.split(':').nth(1))
                    .and_then(|mem| mem.parse::<u64>().ok())
                    .unwrap_or(0);
                
                Ok(CacheStats {
                    hits: stats.hits,
                    misses: stats.misses,
                    total_keys: stats.total_keys,
                    memory_usage,
                })
            }
            Err(_) => {
                // Fallback to local stats
                Ok(stats)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    // Note: These tests require a running Redis instance
    
    #[tokio::test]
    #[ignore] // Ignore by default since it requires Redis
    async fn test_redis_backend_basic_operations() {
        let config = RedisConfig::builder()
            .url("redis://127.0.0.1:6379")
            .key_prefix(Some("test:".to_string()))
            .build();
        
        let backend = RedisBackend::new(config).await.unwrap();
        
        // Clean up any existing test data
        backend.flush().await.unwrap();
        
        // Test put and get
        backend.put("test_key", b"test_value".to_vec(), Some(Duration::from_secs(60))).await.unwrap();
        let result = backend.get("test_key").await.unwrap();
        assert_eq!(result, Some(b"test_value".to_vec()));
        
        // Test exists
        assert!(backend.exists("test_key").await.unwrap());
        assert!(!backend.exists("nonexistent").await.unwrap());
        
        // Test forget
        assert!(backend.forget("test_key").await.unwrap());
        assert!(!backend.exists("test_key").await.unwrap());
    }
    
    #[tokio::test]
    #[ignore]
    async fn test_redis_backend_ttl() {
        let config = RedisConfig::builder()
            .url("redis://127.0.0.1:6379")
            .key_prefix(Some("test_ttl:".to_string()))
            .build();
        
        let backend = RedisBackend::new(config).await.unwrap();
        
        // Clean up
        backend.flush().await.unwrap();
        
        // Put with very short TTL
        backend.put("ttl_test", b"value".to_vec(), Some(Duration::from_secs(1))).await.unwrap();
        
        // Should exist initially
        assert!(backend.exists("ttl_test").await.unwrap());
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Should be expired
        assert!(!backend.exists("ttl_test").await.unwrap());
        let result = backend.get("ttl_test").await.unwrap();
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_redis_config_builder() {
        let config = RedisConfig::builder()
            .url("redis://localhost:6380")
            .pool_size(20)
            .connection_timeout(Duration::from_secs(10))
            .key_prefix(Some("myapp:".to_string()))
            .enable_compression(2048)
            .build();
        
        assert_eq!(config.url, "redis://localhost:6380");
        assert_eq!(config.pool_size, 20);
        assert_eq!(config.connection_timeout, Duration::from_secs(10));
        assert_eq!(config.key_prefix, Some("myapp:".to_string()));
        assert!(config.compression);
        assert_eq!(config.compression_threshold, 2048);
    }
}