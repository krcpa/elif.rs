//! Transaction Management
//!
//! This module provides comprehensive transaction management including
//! lifecycle management, isolation levels, and automatic cleanup.

pub mod isolation;
pub mod lifecycle;
pub mod savepoints;

// Re-export specific types to avoid conflicts
pub use lifecycle::{Transaction, TransactionConfig};
// Note: IsolationLevel is defined in lifecycle.rs, not isolation.rs
pub use lifecycle::IsolationLevel;

/// Transaction builder for configuring transaction options
pub struct TransactionBuilder {
    config: TransactionConfig,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            config: TransactionConfig::default(),
        }
    }

    pub fn isolation_level(mut self, level: IsolationLevel) -> Self {
        self.config.isolation_level = Some(level);
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.config.read_only = read_only;
        self
    }

    pub fn auto_retry(mut self, auto_retry: bool) -> Self {
        self.config.auto_retry = auto_retry;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    pub async fn begin(
        self,
        pool: &crate::database::ManagedPool,
    ) -> Result<Transaction, crate::error::ModelError> {
        Transaction::begin(pool, self.config).await
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
