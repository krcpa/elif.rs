//! Database Integration - Service providers for database connectivity
//! 
//! Provides service providers for database connection pooling and ORM integration
//! with the DI container system using database abstractions.

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use elif_core::{ServiceProvider, Container, ContainerBuilder};
use crate::error::ModelError;
use crate::backends::{
    DatabasePool as DatabasePoolTrait, DatabaseBackend, DatabaseBackendRegistry,
    DatabasePoolConfig, DatabasePoolStats, DatabaseBackendType
};

/// Database connection pool error types
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Connection acquisition failed: {0}")]
    AcquisitionFailed(String),
    
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
            PoolError::AcquisitionFailed(err_msg) => {
                ModelError::Connection(format!("Database connection failed: {}", err_msg))
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

/// Legacy alias for DatabasePoolConfig for backward compatibility
pub type PoolConfig = DatabasePoolConfig;

/// Legacy alias for DatabasePoolStats for backward compatibility
pub type PoolStats = DatabasePoolStats;

/// Extended pool statistics with additional metrics
#[derive(Debug, Clone)]
pub struct ExtendedPoolStats {
    pub pool_stats: DatabasePoolStats,
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
    pool: Arc<dyn DatabasePoolTrait>,
    config: DatabasePoolConfig,
    acquire_count: AtomicU64,
    acquire_errors: AtomicU64,
    created_at: Instant,
}

impl ManagedPool {
    pub fn new(pool: Arc<dyn DatabasePoolTrait>, config: DatabasePoolConfig) -> Self {
        Self {
            pool,
            config,
            acquire_count: AtomicU64::new(0),
            acquire_errors: AtomicU64::new(0),
            created_at: Instant::now(),
        }
    }

    /// Get the underlying pool
    pub fn pool(&self) -> &dyn DatabasePoolTrait {
        &*self.pool
    }

    /// Acquire a connection from the pool with statistics tracking and enhanced error handling
    pub async fn acquire(&self) -> Result<Box<dyn crate::backends::DatabaseConnection>, PoolError> {
        self.acquire_count.fetch_add(1, Ordering::Relaxed);
        
        match self.pool.acquire().await {
            Ok(conn) => {
                let stats = self.pool.stats();
                tracing::debug!("Database connection acquired successfully (total: {}, idle: {})", 
                    stats.total_connections, stats.idle_connections);
                Ok(conn)
            },
            Err(e) => {
                self.acquire_errors.fetch_add(1, Ordering::Relaxed);
                let pool_error = PoolError::AcquisitionFailed(e.to_string());
                tracing::error!("Failed to acquire database connection: {}", pool_error);
                Err(pool_error)
            }
        }
    }

    /// Execute a query directly with the pool
    pub async fn execute(&self, sql: &str, params: &[crate::backends::DatabaseValue]) -> Result<u64, PoolError> {
        self.pool.execute(sql, params).await
            .map_err(|e| PoolError::AcquisitionFailed(e.to_string()))
    }

    /// Begin a database transaction with statistics tracking
    pub async fn begin_transaction(&self) -> Result<Box<dyn crate::backends::DatabaseTransaction>, PoolError> {
        self.acquire_count.fetch_add(1, Ordering::Relaxed);
        
        match self.pool.begin_transaction().await {
            Ok(tx) => {
                tracing::debug!("Database transaction started successfully");
                Ok(tx)
            },
            Err(e) => {
                self.acquire_errors.fetch_add(1, Ordering::Relaxed);
                let pool_error = PoolError::AcquisitionFailed(e.to_string());
                tracing::error!("Failed to begin database transaction: {}", pool_error);
                Err(pool_error)
            }
        }
    }

    /// Get pool statistics with extended metrics
    pub fn extended_stats(&self) -> ExtendedPoolStats {
        ExtendedPoolStats {
            pool_stats: self.pool.stats(),
            acquire_count: self.acquire_count.load(Ordering::Relaxed),
            acquire_errors: self.acquire_errors.load(Ordering::Relaxed),
            created_at: self.created_at,
        }
    }

    /// Get current pool statistics (legacy method for backward compatibility)
    pub fn stats(&self) -> DatabasePoolStats {
        self.pool.stats()
    }

    /// Check pool health with comprehensive error reporting
    pub async fn health_check(&self) -> Result<Duration, PoolError> {
        match self.pool.health_check().await {
            Ok(duration) => {
                tracing::debug!("Database health check passed in {:?}", duration);
                Ok(duration)
            },
            Err(e) => {
                let pool_error = PoolError::HealthCheckFailed { 
                    reason: e.to_string() 
                };
                tracing::error!("Database health check failed: {}", pool_error);
                Err(pool_error)
            }
        }
    }

    /// Check pool health and log detailed statistics
    pub async fn detailed_health_check(&self) -> Result<PoolHealthReport, PoolError> {
        let start = Instant::now();
        let initial_stats = self.extended_stats();
        
        // Perform the actual health check
        let check_duration = self.health_check().await?;
        
        // Get updated statistics 
        let final_stats = self.extended_stats();
        
        let report = PoolHealthReport {
            check_duration,
            total_check_time: start.elapsed(),
            pool_size: final_stats.pool_stats.total_connections,
            idle_connections: final_stats.pool_stats.idle_connections,
            active_connections: final_stats.pool_stats.active_connections,
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
    pub fn config(&self) -> &DatabasePoolConfig {
        &self.config
    }

    /// Close the connection pool
    pub async fn close(&self) -> Result<(), PoolError> {
        self.pool.close().await
            .map_err(|e| PoolError::ConfigurationError { message: e.to_string() })
    }
}

/// Database service provider for connection pool
pub struct DatabaseServiceProvider {
    database_url: String,
    config: DatabasePoolConfig,
    service_name: String,
    backend_registry: Arc<DatabaseBackendRegistry>,
}

impl DatabaseServiceProvider {
    pub fn new(database_url: String) -> Self {
        let mut registry = DatabaseBackendRegistry::new();
        registry.register(
            DatabaseBackendType::PostgreSQL,
            Arc::new(crate::backends::PostgresBackend::new())
        );
        
        Self {
            database_url,
            config: DatabasePoolConfig::default(),
            service_name: "database_pool".to_string(),
            backend_registry: Arc::new(registry),
        }
    }
    
    pub fn with_registry(mut self, registry: Arc<DatabaseBackendRegistry>) -> Self {
        self.backend_registry = registry;
        self
    }

    pub fn with_config(mut self, config: DatabasePoolConfig) -> Self {
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
        self.config.acquire_timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_idle_timeout(mut self, timeout_seconds: Option<u64>) -> Self {
        self.config.idle_timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_max_lifetime(mut self, lifetime_seconds: Option<u64>) -> Self {
        self.config.max_lifetime_seconds = lifetime_seconds;
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
    pub async fn create_pool(&self) -> Result<Arc<dyn DatabasePoolTrait>, ModelError> {
        self.backend_registry.create_pool(&self.database_url, self.config.clone())
            .await
            .map_err(|e| ModelError::Connection(e.to_string()))
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
    pub fn config(&self) -> &DatabasePoolConfig {
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
            self.config.max_connections, self.config.min_connections, self.config.acquire_timeout_seconds,
            self.config.idle_timeout_seconds, self.config.max_lifetime_seconds, self.config.test_before_acquire);
        Ok(())
    }
}

/// Helper function to create a database pool directly with default configuration
pub async fn create_database_pool(database_url: &str) -> Result<Arc<dyn DatabasePoolTrait>, ModelError> {
    create_database_pool_with_config(database_url, &DatabasePoolConfig::default()).await
}

/// Helper function to create a database pool with custom configuration
pub async fn create_database_pool_with_config(
    database_url: &str,
    config: &DatabasePoolConfig
) -> Result<Arc<dyn DatabasePoolTrait>, ModelError> {
    tracing::debug!("Creating database pool with config: max={}, min={}, timeout={}s, idle_timeout={:?}s, max_lifetime={:?}s, test_before_acquire={}", 
        config.max_connections, config.min_connections, config.acquire_timeout_seconds,
        config.idle_timeout_seconds, config.max_lifetime_seconds, config.test_before_acquire);
    
    let mut registry = DatabaseBackendRegistry::new();
    registry.register(
        DatabaseBackendType::PostgreSQL,
        Arc::new(crate::backends::PostgresBackend::new())
    );
    
    let pool = registry.create_pool(database_url, config.clone())
        .await
        .map_err(|e| {
            tracing::error!("Failed to create database pool: {}", e);
            ModelError::Connection(format!("Failed to create database pool: {}", e))
        })?;
    
    tracing::info!("✅ Database pool created successfully with {} max connections", config.max_connections);
    Ok(pool)
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
    pub fn get_all_stats(&self) -> std::collections::HashMap<String, DatabasePoolStats> {
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
pub async fn get_database_pool(_container: &Container) -> Result<Arc<dyn DatabasePoolTrait>, String> {
    // For now, return an error since the container doesn't have service registry yet
    // In future phases, this will integrate with the DI container to retrieve registered pools
    Err("Database pool not yet integrated with current Container implementation - use PoolRegistry for now".to_string())
}

/// Helper function to get custom named database pool from container (for future implementation)  
pub async fn get_named_database_pool(
    _container: &Container, 
    service_name: &str
) -> Result<Arc<dyn DatabasePoolTrait>, String> {
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
    pools: Vec<(String, String, DatabasePoolConfig)>
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
        let config = DatabasePoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.acquire_timeout_seconds, 30);
        assert_eq!(config.idle_timeout_seconds, Some(600));
        assert_eq!(config.max_lifetime_seconds, Some(1800));
        assert!(config.test_before_acquire);
    }

    #[test]
    fn test_database_service_provider_creation() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string());
        assert_eq!(provider.database_url(), "postgresql://test");
        assert_eq!(provider.config().max_connections, 10);
        assert_eq!(provider.config().min_connections, 1);
        assert_eq!(provider.config().acquire_timeout_seconds, 30);
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
        assert_eq!(provider.config().acquire_timeout_seconds, 60);
        assert_eq!(provider.config().idle_timeout_seconds, Some(300));
        assert_eq!(provider.config().max_lifetime_seconds, Some(900));
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
        assert_eq!(provider.config().acquire_timeout_seconds, 30);
        assert_eq!(provider.config().idle_timeout_seconds, Some(600));
        assert_eq!(provider.config().max_lifetime_seconds, Some(1800));
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
        assert_eq!(provider.config().acquire_timeout_seconds, 120);
        assert_eq!(provider.config().idle_timeout_seconds, None);
        assert_eq!(provider.config().max_lifetime_seconds, Some(3600));
        assert_eq!(provider.service_name(), "production_db");
        assert_eq!(provider.database_url(), "postgresql://test");
    }

    #[test]
    fn test_pool_config_creation() {
        let config = PoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.acquire_timeout_seconds, 30);
        assert_eq!(config.idle_timeout_seconds, Some(600));
        assert_eq!(config.max_lifetime_seconds, Some(1800));
        assert!(config.test_before_acquire);
    }

    #[test] 
    fn test_managed_pool_config_access() {
        let config = DatabasePoolConfig {
            max_connections: 5,
            min_connections: 2,
            acquire_timeout_seconds: 60,
            idle_timeout_seconds: None,
            max_lifetime_seconds: Some(3600),
            test_before_acquire: false,
        };
        
        // Verify the config values
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.acquire_timeout_seconds, 60);
        assert_eq!(config.idle_timeout_seconds, None);
        assert_eq!(config.max_lifetime_seconds, Some(3600));
        assert!(!config.test_before_acquire);
    }

    #[test]
    fn test_pool_config_builder() {
        let config = DatabasePoolConfig {
            max_connections: 20,
            min_connections: 2,
            acquire_timeout_seconds: 45,
            idle_timeout_seconds: Some(300),
            max_lifetime_seconds: Some(1200),
            test_before_acquire: false,
        };
        
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string())
            .with_config(config.clone());
        
        assert_eq!(provider.config().max_connections, 20);
        assert_eq!(provider.config().min_connections, 2);
        assert_eq!(provider.config().acquire_timeout_seconds, 45);
        assert_eq!(provider.config().idle_timeout_seconds, Some(300));
        assert_eq!(provider.config().max_lifetime_seconds, Some(1200));
        assert!(!provider.config().test_before_acquire);
    }

    #[test]
    fn test_pool_registry_creation() {
        let registry = PoolRegistry::new();
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