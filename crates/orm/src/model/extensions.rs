//! Model Extensions - Additional utility methods for models
//!
//! Provides convenience methods for model instances including refresh,
//! existence checks, save operations, and transaction support.

use sqlx::Pool;

use crate::error::{ModelError, ModelResult};
use crate::model::core_trait::Model;
use crate::model::crud_operations::CrudOperations;

// <<<ELIF:BEGIN agent-editable:model_extensions>>>
/// Extension trait for models with additional utility methods
pub trait ModelExtensions: Model + CrudOperations {
    /// Refresh this model instance from the database
    async fn refresh(&mut self, pool: &Pool<sqlx::Postgres>) -> ModelResult<()>
    where
        Self: Sized,
    {
        if let Some(pk) = self.primary_key() {
            if let Some(refreshed) = Self::find(pool, pk).await? {
                *self = refreshed;
                Ok(())
            } else {
                Err(ModelError::NotFound(Self::table_name().to_string()))
            }
        } else {
            Err(ModelError::MissingPrimaryKey)
        }
    }

    /// Check if this model instance exists in the database
    async fn exists(&self, pool: &Pool<sqlx::Postgres>) -> ModelResult<bool>
    where
        Self: Sized,
    {
        if let Some(pk) = self.primary_key() {
            let exists = Self::find(pool, pk).await?.is_some();
            Ok(exists)
        } else {
            Ok(false)
        }
    }

    async fn save(&mut self, pool: &Pool<sqlx::Postgres>) -> ModelResult<()>
    where
        Self: Sized + Clone,
    {
        if self.primary_key().is_some() && self.exists(pool).await? {
            // Update existing record
            self.update(pool).await
        } else {
            // Create a new record. `create` takes ownership, so we clone.
            let new_model = Self::create(pool, self.clone()).await?;
            // Update self with the state from the database, including the new primary key.
            *self = new_model;
            Ok(())
        }
    }

    /// Create a clone of this model without the primary key (for duplication)
    fn duplicate(&self) -> Self
    where
        Self: Sized + Clone,
    {
        let mut cloned = self.clone();
        cloned.set_primary_key(Self::PrimaryKey::default());
        
        // Reset timestamps for new record
        if Self::uses_timestamps() {
            cloned.set_created_at(chrono::Utc::now());
            cloned.set_updated_at(chrono::Utc::now());
        }
        
        // Reset soft delete status
        if Self::uses_soft_deletes() {
            cloned.set_deleted_at(None);
        }
        
        cloned
    }

    // <<<ELIF:BEGIN agent-editable:transaction_methods>>>
    // Transaction-scoped operations (placeholders for future implementation)
    
    /// Placeholder for transaction-scoped model operations
    /// Will be fully implemented when derive macros are available
    fn supports_transactions() -> bool
    where
        Self: Sized,
    {
        true // Basic transaction support is available via the transaction module
    }

    /// Check if this model has been modified since last save/load
    /// Default implementation always returns true - override for dirty tracking
    fn is_dirty(&self) -> bool {
        true
    }

    /// Mark all fields as clean (not dirty) after save operations
    /// Default implementation is a no-op - override for dirty tracking
    fn mark_clean(&mut self) {}

    /// Get list of modified fields since last save/load
    /// Default implementation returns empty - override for dirty tracking
    fn dirty_fields(&self) -> Vec<&str> {
        vec![]
    }
    // <<<ELIF:END agent-editable:transaction_methods>>>
}

// Implement ModelExtensions for all types that implement Model + CrudOperations
impl<T: Model + CrudOperations> ModelExtensions for T {}
// <<<ELIF:END agent-editable:model_extensions>>>