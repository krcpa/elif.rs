//! Database Integration - Service providers for database connectivity
//! 
//! Provides service providers for PostgreSQL connection pooling and ORM integration
//! with the DI container system.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use elif_core::{ServiceProvider, Container, ContainerBuilder};
use crate::error::ModelError;

/// Database connection pool error types
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Connection acquisition failed: {0}")]
    AcquisitionFailed(#[from] sqlx::Error),
    
    #[error("Pool is closed")]
    PoolClosed,
    
    #[error("Connection timeout after {timeout}s")]
    ConnectionTimeout { timeout: u64 },
    
    #[error("Pool exhausted: all {max_connections} connections in use")]
    PoolExhausted { max_connections: u32 },
    
    #[error("Health check failed: {reason}")]
    HealthCheckFailed { reason: String },
    
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },
}

/// ORM-specific error for database operations
impl From<PoolError> for ModelError {
    fn from(err: PoolError) -> Self {
        match err {
            PoolError::AcquisitionFailed(sqlx_err) => {
                ModelError::Connection(format!("Database connection failed: {}", sqlx_err))
            },
            PoolError::PoolClosed => {
                ModelError::Connection("Database pool is closed".to_string())
            },
            PoolError::ConnectionTimeout { timeout } => {
                ModelError::Connection(format!("Database connection timeout after {}s", timeout))
            },
            PoolError::PoolExhausted { max_connections } => {
                ModelError::Connection(format!("Database pool exhausted: {} connections in use", max_connections))
            },
            PoolError::HealthCheckFailed { reason } => {
                ModelError::Connection(format!("Database health check failed: {}", reason))
            },
            PoolError::ConfigurationError { message } => {
                ModelError::Connection(format!("Database configuration error: {}", message))
            },
        }
    }
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: u64,
    pub idle_timeout: Option<u64>,
    pub max_lifetime: Option<u64>,
    pub test_before_acquire: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: 30,
            idle_timeout: Some(600), // 10 minutes
            max_lifetime: Some(1800), // 30 minutes
            test_before_acquire: true,
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
    pub acquire_count: u64,
    pub acquire_errors: u64,
    pub created_at: Instant,
}

/// Detailed pool health report
#[derive(Debug, Clone)]
pub struct PoolHealthReport {
    pub check_duration: Duration,
    pub total_check_time: Duration,
    pub pool_size: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
    pub total_acquires: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub created_at: Instant,
}

/// Managed connection pool wrapper with statistics and health monitoring
pub struct ManagedPool {
    pool: Arc<Pool<Postgres>>,
    config: PoolConfig,
    acquire_count: AtomicU64,
    acquire_errors: AtomicU64,
    created_at: Instant,
}

impl ManagedPool {
    pub fn new(pool: Arc<Pool<Postgres>>, config: PoolConfig) -> Self {
        Self {
            pool,
            config,
            acquire_count: AtomicU64::new(0),
            acquire_errors: AtomicU64::new(0),
            created_at: Instant::now(),
        }
    }

    /// Get the underlying pool
    pub fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }

    /// Acquire a connection from the pool with statistics tracking and enhanced error handling
    pub async fn acquire(&self) -> Result<sqlx::pool::PoolConnection<Postgres>, PoolError> {
        if self.pool.is_closed() {
            return Err(PoolError::PoolClosed);
        }

        self.acquire_count.fetch_add(1, Ordering::Relaxed);
        
        match self.pool.acquire().await {
            Ok(conn) => {
                tracing::debug!("Database connection acquired successfully (total: {}, idle: {})", 
                    self.pool.size(), self.pool.num_idle());
                Ok(conn)
            },
            Err(e) => {
                self.acquire_errors.fetch_add(1, Ordering::Relaxed);
                let pool_error = self.classify_error(e);
                tracing::error!("Failed to acquire database connection: {}", pool_error);
                Err(pool_error)
            }
        }
    }

    /// Try to acquire a connection from the pool immediately (non-blocking)
    pub fn try_acquire(&self) -> Result<Option<sqlx::pool::PoolConnection<Postgres>>, PoolError> {
        if self.pool.is_closed() {
            return Err(PoolError::PoolClosed);
        }

        self.acquire_count.fetch_add(1, Ordering::Relaxed);
        
        match self.pool.try_acquire() {
            Some(conn) => {
                tracing::debug!("Database connection acquired immediately");
                Ok(Some(conn))
            },
            None => {
                // Check if pool is exhausted
                if self.pool.size() >= self.config.max_connections {
                    let active = self.pool.size().saturating_sub(self.pool.num_idle() as u32);
                    tracing::warn!("Database pool exhausted: {}/{} connections in use", 
                        active, self.config.max_connections);
                    Err(PoolError::PoolExhausted { max_connections: self.config.max_connections })
                } else {
                    tracing::debug!("No database connections available immediately (size: {}, idle: {})", 
                        self.pool.size(), self.pool.num_idle());
                    Ok(None)
                }
            },
        }
    }

    /// Begin a database transaction with statistics tracking
    pub async fn begin(&self) -> Result<sqlx::Transaction<'_, Postgres>, PoolError> {
        if self.pool.is_closed() {
            return Err(PoolError::PoolClosed);
        }

        self.acquire_count.fetch_add(1, Ordering::Relaxed);
        
        match self.pool.begin().await {
            Ok(tx) => {
                tracing::debug!("Database transaction started successfully");
                Ok(tx)
            },
            Err(e) => {
                self.acquire_errors.fetch_add(1, Ordering::Relaxed);
                let pool_error = self.classify_error(e);
                tracing::error!("Failed to begin database transaction: {}", pool_error);
                Err(pool_error)
            }
        }
    }

    /// Classify sqlx errors into pool-specific errors
    fn classify_error(&self, error: sqlx::Error) -> PoolError {
        match &error {
            sqlx::Error::PoolTimedOut => {
                PoolError::ConnectionTimeout { timeout: self.config.acquire_timeout }
            },
            sqlx::Error::PoolClosed => {
                PoolError::PoolClosed
            },
            _ => {
                PoolError::AcquisitionFailed(error)
            }
        }
    }

    /// Get current pool statistics
    pub fn stats(&self) -> PoolStats {
        let total = self.pool.size() as u32;
        let idle = self.pool.num_idle() as u32;
        let active = if total >= idle { total - idle } else { 0 };
        
        PoolStats {
            total_connections: total,
            idle_connections: idle,
            active_connections: active,
            acquire_count: self.acquire_count.load(Ordering::Relaxed),
            acquire_errors: self.acquire_errors.load(Ordering::Relaxed),
            created_at: self.created_at,
        }
    }

    /// Check pool health with comprehensive error reporting
    pub async fn health_check(&self) -> Result<Duration, PoolError> {
        if self.pool.is_closed() {
            return Err(PoolError::PoolClosed);
        }

        let start = Instant::now();
        
        // Try to acquire a connection for the health check
        let mut conn = self.pool.acquire().await
            .map_err(|e| PoolError::HealthCheckFailed { 
                reason: format!("Could not acquire connection: {}", e) 
            })?;
        
        // Execute a simple query to verify the connection works
        sqlx::query("SELECT 1 as health_check").execute(&mut *conn).await
            .map_err(|e| PoolError::HealthCheckFailed { 
                reason: format!("Health check query failed: {}", e) 
            })?;
        
        let duration = start.elapsed();
        tracing::debug!("Database health check passed in {:?}", duration);
        Ok(duration)
    }

    /// Check pool health and log detailed statistics
    pub async fn detailed_health_check(&self) -> Result<PoolHealthReport, PoolError> {
        let start = Instant::now();
        let _initial_stats = self.stats();
        
        // Perform the actual health check
        let check_duration = self.health_check().await?;
        
        // Get updated statistics 
        let final_stats = self.stats();
        
        let report = PoolHealthReport {
            check_duration,
            total_check_time: start.elapsed(),
            pool_size: final_stats.total_connections,
            idle_connections: final_stats.idle_connections,
            active_connections: final_stats.active_connections,
            total_acquires: final_stats.acquire_count,
            total_errors: final_stats.acquire_errors,
            error_rate: if final_stats.acquire_count > 0 {
                (final_stats.acquire_errors as f64 / final_stats.acquire_count as f64) * 100.0
            } else {
                0.0
            },
            created_at: final_stats.created_at,
        };
        
        tracing::info!("Database pool health report: {:?}", report);
        Ok(report)
    }

    /// Get connection pool configuration
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// Close the connection pool
    pub fn close(&self) {
        self.pool.close();
    }
}

/// Database service provider for PostgreSQL connection pool
pub struct DatabaseServiceProvider {
    database_url: String,
    config: PoolConfig,
    service_name: String,
}

impl DatabaseServiceProvider {
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            config: PoolConfig::default(),
            service_name: "database_pool".to_string(),
        }
    }

    pub fn with_config(mut self, config: PoolConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_max_connections(mut self, max_connections: u32) -> Self {
        self.config.max_connections = max_connections;
        self
    }

    pub fn with_min_connections(mut self, min_connections: u32) -> Self {
        self.config.min_connections = min_connections;
        self
    }

    pub fn with_acquire_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.acquire_timeout = timeout_seconds;
        self
    }

    pub fn with_idle_timeout(mut self, timeout_seconds: Option<u64>) -> Self {
        self.config.idle_timeout = timeout_seconds;
        self
    }

    pub fn with_max_lifetime(mut self, lifetime_seconds: Option<u64>) -> Self {
        self.config.max_lifetime = lifetime_seconds;
        self
    }

    pub fn with_test_before_acquire(mut self, enabled: bool) -> Self {
        self.config.test_before_acquire = enabled;
        self
    }

    pub fn with_service_name(mut self, service_name: String) -> Self {
        self.service_name = service_name;
        self
    }

    /// Create a database pool using this provider's configuration
    pub async fn create_pool(&self) -> Result<Arc<Pool<Postgres>>, ModelError> {
        create_database_pool_with_config(&self.database_url, &self.config).await
    }

    /// Create a managed database pool with statistics and health monitoring
    pub async fn create_managed_pool(&self) -> Result<ManagedPool, ModelError> {
        let pool = self.create_pool().await?;
        Ok(ManagedPool::new(pool, self.config.clone()))
    }

    /// Get the database URL (for diagnostic purposes)
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    /// Get the service name for this provider
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the pool configuration
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }
}

impl ServiceProvider for DatabaseServiceProvider {
    fn name(&self) -> &'static str {
        "DatabaseServiceProvider"
    }
    
    fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, elif_core::ProviderError> {
        // Store database configuration for later pool creation
        // The actual pool will be created during boot phase
        tracing::debug!("Registering database service with URL: {}", 
            self.database_url.split('@').last().unwrap_or("unknown"));
        Ok(builder)
    }
    
    fn boot(&self, _container: &Container) -> Result<(), elif_core::ProviderError> {
        tracing::info!("✅ Database service provider booted successfully");
        tracing::debug!("Database pool configuration: max_connections={}, min_connections={}, acquire_timeout={}s, idle_timeout={:?}s, max_lifetime={:?}s, test_before_acquire={}", 
            self.config.max_connections, self.config.min_connections, self.config.acquire_timeout,
            self.config.idle_timeout, self.config.max_lifetime, self.config.test_before_acquire);
        Ok(())
    }
}

/// Helper function to create a database pool directly with default configuration
pub async fn create_database_pool(database_url: &str) -> Result<Arc<Pool<Postgres>>, ModelError> {
    create_database_pool_with_config(database_url, &PoolConfig::default()).await
}

/// Helper function to create a database pool with custom configuration
pub async fn create_database_pool_with_config(
    database_url: &str,
    config: &PoolConfig
) -> Result<Arc<Pool<Postgres>>, ModelError> {
    tracing::debug!("Creating database pool with config: max={}, min={}, timeout={}s, idle_timeout={:?}s, max_lifetime={:?}s, test_before_acquire={}", 
        config.max_connections, config.min_connections, config.acquire_timeout,
        config.idle_timeout, config.max_lifetime, config.test_before_acquire);
    
    let mut options = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout))
        .test_before_acquire(config.test_before_acquire);

    // Set idle timeout if specified
    if let Some(idle_timeout) = config.idle_timeout {
        options = options.idle_timeout(Duration::from_secs(idle_timeout));
    }

    // Set max lifetime if specified
    if let Some(max_lifetime) = config.max_lifetime {
        options = options.max_lifetime(Duration::from_secs(max_lifetime));
    }
    
    let pool = options.connect(database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create database pool: {}", e);
            ModelError::Connection(format!("Failed to create database pool: {}", e))
        })?;
    
    tracing::info!("✅ Database pool created successfully with {} max connections", config.max_connections);
    Ok(Arc::new(pool))
}

/// Database pool registry for DI container integration
pub struct PoolRegistry {
    pools: std::collections::HashMap<String, Arc<ManagedPool>>,
}

/// Type alias for database pool used by migration runner
pub type DatabasePool = ManagedPool;

impl PoolRegistry {
    pub fn new() -> Self {
        Self {
            pools: std::collections::HashMap::new(),
        }
    }

    /// Register a managed pool with a name
    pub fn register(&mut self, name: String, pool: Arc<ManagedPool>) {
        tracing::info!("Registering database pool: {}", name);
        self.pools.insert(name, pool);
    }

    /// Get a managed pool by name
    pub fn get(&self, name: &str) -> Option<Arc<ManagedPool>> {
        self.pools.get(name).cloned()
    }

    /// Get the default pool (usually named "database_pool")
    pub fn get_default(&self) -> Option<Arc<ManagedPool>> {
        self.get("database_pool")
    }

    /// List all registered pool names
    pub fn pool_names(&self) -> Vec<&String> {
        self.pools.keys().collect()
    }

    /// Get pool statistics for all registered pools
    pub fn get_all_stats(&self) -> std::collections::HashMap<String, PoolStats> {
        self.pools
            .iter()
            .map(|(name, pool)| (name.clone(), pool.stats()))
            .collect()
    }

    /// Perform health check on all pools
    pub async fn health_check_all(&self) -> std::collections::HashMap<String, Result<Duration, PoolError>> {
        let mut results = std::collections::HashMap::new();
        
        for (name, pool) in &self.pools {
            let result = pool.health_check().await;
            results.insert(name.clone(), result);
        }
        
        results
    }
}

impl Default for PoolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to get database pool from container (for future implementation)
pub async fn get_database_pool(_container: &Container) -> Result<Arc<Pool<Postgres>>, String> {
    // For now, return an error since the container doesn't have service registry yet
    // In future phases, this will integrate with the DI container to retrieve registered pools
    Err("Database pool not yet integrated with current Container implementation - use PoolRegistry for now".to_string())
}

/// Helper function to get custom named database pool from container (for future implementation)  
pub async fn get_named_database_pool(
    _container: &Container, 
    service_name: &str
) -> Result<Arc<Pool<Postgres>>, String> {
    // For now, return an error since the container doesn't have service registry yet
    // In future phases, this will integrate with the DI container to retrieve registered pools by name
    Err(format!("Database pool '{}' not yet integrated with current Container implementation - use PoolRegistry for now", service_name))
}

/// Create a pool registry with a default database pool
pub async fn create_default_pool_registry(database_url: &str) -> Result<PoolRegistry, ModelError> {
    let mut registry = PoolRegistry::new();
    
    let provider = DatabaseServiceProvider::new(database_url.to_string());
    let managed_pool = provider.create_managed_pool().await?;
    
    registry.register("database_pool".to_string(), Arc::new(managed_pool));
    
    tracing::info!("Created default pool registry with database_pool");
    Ok(registry)
}

/// Create a pool registry with custom configuration
pub async fn create_custom_pool_registry(
    pools: Vec<(String, String, PoolConfig)>
) -> Result<PoolRegistry, ModelError> {
    let mut registry = PoolRegistry::new();
    
    for (name, database_url, config) in pools {
        let provider = DatabaseServiceProvider::new(database_url)
            .with_config(config);
        let managed_pool = provider.create_managed_pool().await?;
        
        registry.register(name, Arc::new(managed_pool));
    }
    
    tracing::info!("Created custom pool registry with {} pools", registry.pool_names().len());
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_defaults() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.acquire_timeout, 30);
        assert_eq!(config.idle_timeout, Some(600));
        assert_eq!(config.max_lifetime, Some(1800));
        assert!(config.test_before_acquire);
    }

    #[test]
    fn test_database_service_provider_creation() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string());
        assert_eq!(provider.database_url(), "postgresql://test");
        assert_eq!(provider.config().max_connections, 10);
        assert_eq!(provider.config().min_connections, 1);
        assert_eq!(provider.config().acquire_timeout, 30);
        assert_eq!(provider.service_name(), "database_pool");
    }

    #[test]
    fn test_database_service_provider_configuration() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string())
            .with_max_connections(20)
            .with_min_connections(5)
            .with_acquire_timeout(60)
            .with_idle_timeout(Some(300))
            .with_max_lifetime(Some(900))
            .with_test_before_acquire(false)
            .with_service_name("custom_db".to_string());

        assert_eq!(provider.config().max_connections, 20);
        assert_eq!(provider.config().min_connections, 5);
        assert_eq!(provider.config().acquire_timeout, 60);
        assert_eq!(provider.config().idle_timeout, Some(300));
        assert_eq!(provider.config().max_lifetime, Some(900));
        assert!(!provider.config().test_before_acquire);
        assert_eq!(provider.service_name(), "custom_db");
    }

    #[test]
    fn test_provider_name() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string());
        assert_eq!(provider.name(), "DatabaseServiceProvider");
    }

    #[test]
    fn test_database_service_provider_accessors() {
        let provider = DatabaseServiceProvider::new("postgresql://test_db".to_string())
            .with_service_name("custom_service".to_string());
        
        assert_eq!(provider.database_url(), "postgresql://test_db");
        assert_eq!(provider.service_name(), "custom_service");
    }

    #[test] 
    fn test_database_service_provider_defaults() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string());
        
        assert_eq!(provider.config().max_connections, 10);
        assert_eq!(provider.config().min_connections, 1);
        assert_eq!(provider.config().acquire_timeout, 30);
        assert_eq!(provider.config().idle_timeout, Some(600));
        assert_eq!(provider.config().max_lifetime, Some(1800));
        assert!(provider.config().test_before_acquire);
        assert_eq!(provider.service_name(), "database_pool");
    }

    #[test]
    fn test_database_service_provider_fluent_configuration() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string())
            .with_max_connections(50)
            .with_min_connections(10)
            .with_acquire_timeout(120)
            .with_idle_timeout(None)
            .with_max_lifetime(Some(3600))
            .with_service_name("production_db".to_string());

        assert_eq!(provider.config().max_connections, 50);
        assert_eq!(provider.config().min_connections, 10);
        assert_eq!(provider.config().acquire_timeout, 120);
        assert_eq!(provider.config().idle_timeout, None);
        assert_eq!(provider.config().max_lifetime, Some(3600));
        assert_eq!(provider.service_name(), "production_db");
        assert_eq!(provider.database_url(), "postgresql://test");
    }

    #[test]
    fn test_pool_config_creation() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.acquire_timeout, 30);
        assert_eq!(config.idle_timeout, Some(600));
        assert_eq!(config.max_lifetime, Some(1800));
        assert!(config.test_before_acquire);
    }

    #[test] 
    fn test_managed_pool_config_access() {
        let config = PoolConfig {
            max_connections: 5,
            min_connections: 2,
            acquire_timeout: 60,
            idle_timeout: None,
            max_lifetime: Some(3600),
            test_before_acquire: false,
        };
        
        // Verify the config values
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.acquire_timeout, 60);
        assert_eq!(config.idle_timeout, None);
        assert_eq!(config.max_lifetime, Some(3600));
        assert!(!config.test_before_acquire);
    }

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig {
            max_connections: 20,
            min_connections: 2,
            acquire_timeout: 45,
            idle_timeout: Some(300),
            max_lifetime: Some(1200),
            test_before_acquire: false,
        };
        
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string())
            .with_config(config.clone());
        
        assert_eq!(provider.config().max_connections, 20);
        assert_eq!(provider.config().min_connections, 2);
        assert_eq!(provider.config().acquire_timeout, 45);
        assert_eq!(provider.config().idle_timeout, Some(300));
        assert_eq!(provider.config().max_lifetime, Some(1200));
        assert!(!provider.config().test_before_acquire);
    }

    #[test]
    fn test_pool_registry_creation() {
        let mut registry = PoolRegistry::new();
        assert!(registry.get_default().is_none());
        assert!(registry.pool_names().is_empty());
        
        // Test that registry methods don't panic on empty registry
        let stats = registry.get_all_stats();
        assert!(stats.is_empty());
    }

    #[test]
    fn test_pool_error_types() {
        let timeout_error = PoolError::ConnectionTimeout { timeout: 30 };
        let pool_closed_error = PoolError::PoolClosed;
        let exhausted_error = PoolError::PoolExhausted { max_connections: 10 };
        
        // Test error display
        assert!(timeout_error.to_string().contains("timeout"));
        assert!(pool_closed_error.to_string().contains("closed"));
        assert!(exhausted_error.to_string().contains("exhausted"));
    }

    #[test]
    fn test_pool_error_model_conversion() {
        let pool_error = PoolError::PoolExhausted { max_connections: 5 };
        let model_error: ModelError = pool_error.into();
        
        // Verify it converts to ModelError
        assert!(matches!(model_error, ModelError::Connection(_)));
    }
}