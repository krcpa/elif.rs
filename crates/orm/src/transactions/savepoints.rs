//! Transaction Savepoints
//!
//! Provides savepoint management for nested transactions and partial rollbacks.

use crate::backends::DatabaseTransaction;
use crate::error::{ModelError, ModelResult};

/// Savepoint manager for handling nested transactions
pub struct SavepointManager {
    savepoint_count: u32,
}

impl SavepointManager {
    pub fn new() -> Self {
        Self { savepoint_count: 0 }
    }

    /// Create a new savepoint
    pub async fn create_savepoint(
        &mut self,
        tx: &mut Box<dyn DatabaseTransaction>,
    ) -> ModelResult<String> {
        self.savepoint_count += 1;
        let savepoint_name = format!("sp_{}", self.savepoint_count);

        let sql = format!("SAVEPOINT {}", savepoint_name);
        tx.execute(&sql, &[])
            .await
            .map_err(|e| ModelError::Transaction(format!("Failed to create savepoint: {}", e)))?;

        Ok(savepoint_name)
    }

    /// Release a savepoint
    pub async fn release_savepoint(
        &mut self,
        tx: &mut Box<dyn DatabaseTransaction>,
        savepoint_name: &str,
    ) -> ModelResult<()> {
        let sql = format!("RELEASE SAVEPOINT {}", savepoint_name);
        tx.execute(&sql, &[])
            .await
            .map_err(|e| ModelError::Transaction(format!("Failed to release savepoint: {}", e)))?;

        Ok(())
    }

    /// Rollback to a savepoint
    pub async fn rollback_to_savepoint(
        &mut self,
        tx: &mut Box<dyn DatabaseTransaction>,
        savepoint_name: &str,
    ) -> ModelResult<()> {
        let sql = format!("ROLLBACK TO SAVEPOINT {}", savepoint_name);
        tx.execute(&sql, &[]).await.map_err(|e| {
            ModelError::Transaction(format!("Failed to rollback to savepoint: {}", e))
        })?;

        Ok(())
    }

    /// Get the current savepoint count
    pub fn savepoint_count(&self) -> u32 {
        self.savepoint_count
    }
}

impl Default for SavepointManager {
    fn default() -> Self {
        Self::new()
    }
}
