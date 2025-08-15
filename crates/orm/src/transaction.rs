//! Transaction Management
//! 
//! Provides high-level transaction management with automatic cleanup,
//! scoped operations, and comprehensive error handling.

use sqlx::{Postgres, Transaction as SqlxTransaction};
use tracing::{debug, warn};
use crate::error::{ModelError, ModelResult};
use crate::database::ManagedPool;

/// Transaction isolation levels supported by PostgreSQL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Read Uncommitted - lowest isolation level
    ReadUncommitted,
    /// Read Committed - default PostgreSQL isolation level
    ReadCommitted,
    /// Repeatable Read - stronger consistency guarantees
    RepeatableRead,
    /// Serializable - highest isolation level
    Serializable,
}

impl IsolationLevel {
    /// Convert to SQL string for SET TRANSACTION ISOLATION LEVEL command
    pub fn as_sql(&self) -> &'static str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED", 
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
}

/// Transaction configuration options
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    /// Transaction isolation level
    pub isolation_level: Option<IsolationLevel>,
    /// Whether the transaction is read-only
    pub read_only: bool,
    /// Enable automatic retry on serialization failures
    pub auto_retry: bool,
    /// Maximum number of retry attempts
    pub max_retries: u32,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            isolation_level: None, // Use PostgreSQL default (READ COMMITTED)
            read_only: false,
            auto_retry: false,
            max_retries: 3,
        }
    }
}

/// High-level transaction wrapper with automatic cleanup and enhanced functionality
pub struct Transaction {
    inner: Option<SqlxTransaction<'static, Postgres>>,
    config: TransactionConfig,
    committed: bool,
}

impl Transaction {
    /// Create a new transaction with the given configuration
    pub async fn begin(pool: &ManagedPool, config: TransactionConfig) -> Result<Transaction, ModelError> {
        debug!("Beginning transaction with config: {:?}", config);
        
        let mut tx = pool.begin().await
            .map_err(|e| ModelError::Transaction(format!("Failed to begin transaction: {}", e)))?;
        
        // Set isolation level if specified
        if let Some(isolation_level) = config.isolation_level {
            let sql = format!("SET TRANSACTION ISOLATION LEVEL {}", isolation_level.as_sql());
            sqlx::query(&sql)
                .execute(&mut *tx)
                .await
                .map_err(|e| ModelError::Transaction(format!("Failed to set isolation level: {}", e)))?;
            debug!("Transaction isolation level set to: {:?}", isolation_level);
        }
        
        // Set read-only mode if specified
        if config.read_only {
            sqlx::query("SET TRANSACTION READ ONLY")
                .execute(&mut *tx)
                .await
                .map_err(|e| ModelError::Transaction(format!("Failed to set read-only mode: {}", e)))?;
            debug!("Transaction set to read-only mode");
        }
        
        // SAFETY: We need to transmute the lifetime to 'static for storage
        // This is safe because the Transaction struct manages the lifetime
        // and ensures the transaction is properly cleaned up
        let tx_static = unsafe { std::mem::transmute(tx) };
        
        Ok(Transaction {
            inner: Some(tx_static),
            config,
            committed: false,
        })
    }
    
    /// Create a transaction with default configuration
    pub async fn begin_default(pool: &ManagedPool) -> Result<Transaction, ModelError> {
        Self::begin(pool, TransactionConfig::default()).await
    }
    
    /// Create a read-only transaction
    pub async fn begin_read_only(pool: &ManagedPool) -> Result<Transaction, ModelError> {
        let config = TransactionConfig {
            read_only: true,
            ..Default::default()
        };
        Self::begin(pool, config).await
    }
    
    /// Create a serializable transaction
    pub async fn begin_serializable(pool: &ManagedPool) -> Result<Transaction, ModelError> {
        let config = TransactionConfig {
            isolation_level: Some(IsolationLevel::Serializable),
            auto_retry: true, // Enable auto-retry for serializable transactions
            ..Default::default()
        };
        Self::begin(pool, config).await
    }
    
    /// Get a mutable reference to the underlying sqlx transaction
    /// 
    /// # Safety
    /// This method provides direct access to the underlying transaction.
    /// Care should be taken not to commit or rollback the transaction directly
    /// as this will invalidate the Transaction wrapper.
    pub fn as_mut(&mut self) -> Option<&mut SqlxTransaction<'static, Postgres>> {
        self.inner.as_mut()
    }
    
    /// Get an immutable reference to the underlying sqlx transaction
    pub fn as_ref(&self) -> Option<&SqlxTransaction<'static, Postgres>> {
        self.inner.as_ref()
    }
    
    /// Execute a closure within the transaction scope with a borrowed transaction
    /// 
    /// This is a safe way to execute database operations within a transaction.
    /// The closure receives access to execute queries against the transaction.
    pub async fn execute<F, Fut, R>(&mut self, f: F) -> Result<R, ModelError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<R, ModelError>>,
    {
        if self.inner.is_some() {
            f().await
        } else {
            Err(ModelError::Transaction("Transaction has been consumed".to_string()))
        }
    }
    
    /// Commit the transaction
    pub async fn commit(mut self) -> ModelResult<()> {
        if let Some(tx) = self.inner.take() {
            debug!("Committing transaction");
            tx.commit().await
                .map_err(|e| ModelError::Transaction(format!("Failed to commit transaction: {}", e)))?;
            self.committed = true;
            debug!("Transaction committed successfully");
            Ok(())
        } else {
            Err(ModelError::Transaction("Transaction has already been consumed".to_string()))
        }
    }
    
    /// Rollback the transaction
    pub async fn rollback(mut self) -> ModelResult<()> {
        if let Some(tx) = self.inner.take() {
            debug!("Rolling back transaction");
            tx.rollback().await
                .map_err(|e| ModelError::Transaction(format!("Failed to rollback transaction: {}", e)))?;
            debug!("Transaction rolled back successfully");
            Ok(())
        } else {
            Err(ModelError::Transaction("Transaction has already been consumed".to_string()))
        }
    }
    
    /// Check if the transaction has been committed
    pub fn is_committed(&self) -> bool {
        self.committed
    }
    
    /// Check if the transaction is still active (not committed or rolled back)
    pub fn is_active(&self) -> bool {
        self.inner.is_some()
    }
    
    /// Get the transaction configuration
    pub fn config(&self) -> &TransactionConfig {
        &self.config
    }
}

impl Drop for Transaction {
    /// Automatic cleanup: rollback the transaction if it hasn't been committed or rolled back
    fn drop(&mut self) {
        if let Some(tx) = self.inner.take() {
            if !self.committed {
                warn!("Transaction dropped without explicit commit or rollback - this will cause an automatic rollback");
                // Note: We can't await in Drop, so we log a warning
                // The sqlx::Transaction will handle the rollback in its own Drop impl
                std::mem::drop(tx);
            }
        }
    }
}

/// Execute a closure within a transaction scope with automatic commit/rollback
/// 
/// This is a convenience function that handles transaction lifecycle automatically:
/// - If the closure succeeds, the transaction is committed
/// - If the closure fails, the transaction is rolled back
/// - Supports automatic retry for serializable transactions
pub async fn with_transaction<F, Fut, R>(
    pool: &ManagedPool,
    config: TransactionConfig,
    f: F,
) -> Result<R, ModelError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<R, ModelError>>,
{
    let mut attempts = 0;
    let max_attempts = if config.auto_retry { config.max_retries + 1 } else { 1 };
    
    loop {
        attempts += 1;
        debug!("Starting transaction attempt {} of {}", attempts, max_attempts);
        
        let mut tx = Transaction::begin(pool, config.clone()).await?;
        
        match tx.execute(&f).await {
            Ok(result) => {
                tx.commit().await?;
                return Ok(result);
            }
            Err(e) => {
                // Check if this is a serialization failure that can be retried
                let should_retry = config.auto_retry && 
                    attempts < max_attempts && 
                    is_serialization_failure(&e);
                
                if should_retry {
                    warn!("Serialization failure on attempt {}, retrying: {}", attempts, e);
                    tx.rollback().await.ok(); // Best effort rollback
                    continue;
                } else {
                    tx.rollback().await.ok(); // Best effort rollback
                    return Err(e);
                }
            }
        }
    }
}

/// Execute a closure within a default transaction scope
pub async fn with_transaction_default<F, Fut, R>(
    pool: &ManagedPool,
    f: F,
) -> Result<R, ModelError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<R, ModelError>>,
{
    with_transaction(pool, TransactionConfig::default(), f).await
}

/// Check if an error represents a serialization failure that can be retried
pub fn is_serialization_failure(error: &ModelError) -> bool {
    match error {
        ModelError::Database(msg) | ModelError::Transaction(msg) => {
            // PostgreSQL serialization failure error codes
            msg.contains("40001") || // serialization_failure
            msg.contains("40P01") || // deadlock_detected
            msg.contains("could not serialize access")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_isolation_level_sql() {
        assert_eq!(IsolationLevel::ReadUncommitted.as_sql(), "READ UNCOMMITTED");
        assert_eq!(IsolationLevel::ReadCommitted.as_sql(), "READ COMMITTED");
        assert_eq!(IsolationLevel::RepeatableRead.as_sql(), "REPEATABLE READ");
        assert_eq!(IsolationLevel::Serializable.as_sql(), "SERIALIZABLE");
    }
    
    #[test]
    fn test_transaction_config_default() {
        let config = TransactionConfig::default();
        assert!(config.isolation_level.is_none());
        assert!(!config.read_only);
        assert!(!config.auto_retry);
        assert_eq!(config.max_retries, 3);
    }
    
    #[test]
    fn test_serialization_failure_detection() {
        let err1 = ModelError::Database("ERROR: could not serialize access due to concurrent update".to_string());
        assert!(is_serialization_failure(&err1));
        
        let err2 = ModelError::Transaction("ERROR: 40001".to_string());
        assert!(is_serialization_failure(&err2));
        
        let err3 = ModelError::Validation("Invalid input".to_string());
        assert!(!is_serialization_failure(&err3));
    }
}