//! Connection Pool Management
//!
//! This module provides managed connection pools with statistics tracking,
//! health monitoring, and comprehensive error handling.

use super::health::PoolHealthReport;
use super::statistics::ExtendedPoolStats;
use crate::backends::{DatabasePool as DatabasePoolTrait, DatabasePoolConfig, DatabasePoolStats};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
impl From<PoolError> for crate::error::ModelError {
    fn from(err: PoolError) -> Self {
        match err {
            PoolError::AcquisitionFailed(err_msg) => crate::error::ModelError::Connection(format!(
                "Database connection failed: {}",
                err_msg
            )),
            PoolError::PoolClosed => {
                crate::error::ModelError::Connection("Database pool is closed".to_string())
            }
            PoolError::ConnectionTimeout { timeout } => crate::error::ModelError::Connection(
                format!("Database connection timeout after {}s", timeout),
            ),
            PoolError::PoolExhausted { max_connections } => {
                crate::error::ModelError::Connection(format!(
                    "Database pool exhausted: {} connections in use",
                    max_connections
                ))
            }
            PoolError::HealthCheckFailed { reason } => crate::error::ModelError::Connection(
                format!("Database health check failed: {}", reason),
            ),
            PoolError::ConfigurationError { message } => crate::error::ModelError::Connection(
                format!("Database configuration error: {}", message),
            ),
        }
    }
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
                tracing::debug!(
                    "Database connection acquired successfully (total: {}, idle: {})",
                    stats.total_connections,
                    stats.idle_connections
                );
                Ok(conn)
            }
            Err(e) => {
                self.acquire_errors.fetch_add(1, Ordering::Relaxed);
                let pool_error = PoolError::AcquisitionFailed(e.to_string());
                tracing::error!("Failed to acquire database connection: {}", pool_error);
                Err(pool_error)
            }
        }
    }

    /// Execute a query directly with the pool
    pub async fn execute(
        &self,
        sql: &str,
        params: &[crate::backends::DatabaseValue],
    ) -> Result<u64, PoolError> {
        self.pool
            .execute(sql, params)
            .await
            .map_err(|e| PoolError::AcquisitionFailed(e.to_string()))
    }

    /// Begin a database transaction with statistics tracking
    pub async fn begin_transaction(
        &self,
    ) -> Result<Box<dyn crate::backends::DatabaseTransaction>, PoolError> {
        self.acquire_count.fetch_add(1, Ordering::Relaxed);

        match self.pool.begin_transaction().await {
            Ok(tx) => {
                tracing::debug!("Database transaction started successfully");
                Ok(tx)
            }
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
            }
            Err(e) => {
                let pool_error = PoolError::HealthCheckFailed {
                    reason: e.to_string(),
                };
                tracing::error!("Database health check failed: {}", pool_error);
                Err(pool_error)
            }
        }
    }

    /// Check pool health and log detailed statistics
    pub async fn detailed_health_check(&self) -> Result<PoolHealthReport, PoolError> {
        let start = Instant::now();
        let _initial_stats = self.extended_stats();

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
        self.pool
            .close()
            .await
            .map_err(|e| PoolError::ConfigurationError {
                message: e.to_string(),
            })
    }
}

/// Legacy aliases for backward compatibility
pub type PoolConfig = DatabasePoolConfig;
pub type PoolStats = DatabasePoolStats;
pub type DatabasePool = ManagedPool;
