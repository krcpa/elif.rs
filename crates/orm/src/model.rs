//! Base Model System - Core trait and functionality for database entities
//! 
//! Implements the Model trait with standard CRUD operations, primary key handling,
//! timestamps, soft deletes, and model serialization/deserialization.

use std::collections::HashMap;
use std::fmt::Debug;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

use crate::error::{ModelError, ModelResult};
use crate::query::QueryBuilder;

/// Primary key types supported by the ORM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimaryKey {
    /// Auto-incrementing integer primary key
    Integer(i64),
    /// UUID primary key
    Uuid(Uuid),
    /// Composite primary key (multiple fields)
    Composite(HashMap<String, String>),
}

impl std::fmt::Display for PrimaryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimaryKey::Integer(id) => write!(f, "{}", id),
            PrimaryKey::Uuid(id) => write!(f, "{}", id),
            PrimaryKey::Composite(fields) => {
                let pairs: Vec<String> = fields.iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect();
                write!(f, "{}", pairs.join(","))
            }
        }
    }
}

impl PrimaryKey {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PrimaryKey::Integer(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            PrimaryKey::Uuid(id) => Some(*id),
            _ => None,
        }
    }
}

/// Trait for database models with standard ORM operations
pub trait Model: Send + Sync + Debug + Serialize + for<'de> Deserialize<'de> {
    /// The type used for this model's primary key
    type PrimaryKey: Clone + Send + Sync + Debug + std::fmt::Display;

    /// Table name for this model
    fn table_name() -> &'static str;

    /// Primary key field name(s)
    fn primary_key_name() -> &'static str {
        "id"
    }

    /// Get the primary key value for this model instance
    fn primary_key(&self) -> Option<Self::PrimaryKey>;

    /// Set the primary key value for this model instance
    fn set_primary_key(&mut self, key: Self::PrimaryKey);

    /// Check if this model uses timestamps (created_at, updated_at)
    fn uses_timestamps() -> bool {
        false
    }

    /// Check if this model supports soft deletes
    fn uses_soft_deletes() -> bool {
        false
    }

    /// Get created_at timestamp if available
    fn created_at(&self) -> Option<DateTime<Utc>> {
        None
    }

    /// Set created_at timestamp
    fn set_created_at(&mut self, _timestamp: DateTime<Utc>) {}

    /// Get updated_at timestamp if available
    fn updated_at(&self) -> Option<DateTime<Utc>> {
        None
    }

    /// Set updated_at timestamp
    fn set_updated_at(&mut self, _timestamp: DateTime<Utc>) {}

    /// Get deleted_at timestamp if available (for soft deletes)
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        None
    }

    /// Set deleted_at timestamp (for soft deletes)
    fn set_deleted_at(&mut self, _timestamp: Option<DateTime<Utc>>) {}

    /// Check if this model instance is soft deleted
    fn is_soft_deleted(&self) -> bool {
        self.deleted_at().is_some()
    }

    // <<<ELIF:BEGIN agent-editable:model_crud_operations>>>
    /// Find a model by its primary key
    async fn find(pool: &Pool<Postgres>, id: Self::PrimaryKey) -> ModelResult<Option<Self>>
    where
        Self: Sized,
    {
        let query: QueryBuilder<Self> = QueryBuilder::new()
            .select("*")
            .from(Self::table_name())
            .where_eq(Self::primary_key_name(), id.to_string());

        let sql = query.to_sql();
        let row = sqlx::query(&sql)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(row) => {
                let model = Self::from_row(&row)?;
                Ok(Some(model))
            }
            None => Ok(None),
        }
    }

    /// Find a model by its primary key or return an error if not found
    async fn find_or_fail(pool: &Pool<Postgres>, id: Self::PrimaryKey) -> ModelResult<Self>
    where
        Self: Sized,
    {
        Self::find(pool, id)
            .await?
            .ok_or_else(|| ModelError::NotFound(Self::table_name().to_string()))
    }

    /// Create a new model instance in the database (placeholder implementation)
    async fn create(pool: &Pool<Postgres>, mut model: Self) -> ModelResult<Self>
    where
        Self: Sized,
    {
        // Set timestamps if enabled
        if Self::uses_timestamps() {
            let now = Utc::now();
            model.set_created_at(now);
            model.set_updated_at(now);
        }

        // TODO: Build INSERT query dynamically based on model fields
        // For now, this is a placeholder - real implementation will use derive macro
        // to generate field-specific SQL
        
        let insert_sql = format!("INSERT INTO {} DEFAULT VALUES RETURNING *", Self::table_name());
        let row = sqlx::query(&insert_sql)
            .fetch_one(pool)
            .await?;

        Self::from_row(&row)
    }

    /// Update this model instance in the database
    async fn update(&mut self, pool: &Pool<Postgres>) -> ModelResult<()> {
        if let Some(pk) = self.primary_key() {
            // Set updated_at timestamp if enabled
            if Self::uses_timestamps() {
                self.set_updated_at(Utc::now());
            }

            // TODO: Build UPDATE query dynamically based on changed fields
            // For now, this is a placeholder
            let update_sql = format!(
                "UPDATE {} SET updated_at = NOW() WHERE {} = $1",
                Self::table_name(),
                Self::primary_key_name()
            );
            
            sqlx::query(&update_sql)
                .bind(pk.to_string())
                .execute(pool)
                .await?;

            Ok(())
        } else {
            Err(ModelError::MissingPrimaryKey)
        }
    }

    /// Delete this model instance from the database
    async fn delete(self, pool: &Pool<Postgres>) -> ModelResult<()> {
        if let Some(pk) = self.primary_key() {
            if Self::uses_soft_deletes() {
                // Soft delete - just set deleted_at timestamp
                let soft_delete_sql = format!(
                    "UPDATE {} SET deleted_at = NOW() WHERE {} = $1",
                    Self::table_name(),
                    Self::primary_key_name()
                );
                
                sqlx::query(&soft_delete_sql)
                    .bind(pk.to_string())
                    .execute(pool)
                    .await?;
            } else {
                // Hard delete - remove from database
                let delete_sql = format!(
                    "DELETE FROM {} WHERE {} = $1",
                    Self::table_name(),
                    Self::primary_key_name()
                );
                
                sqlx::query(&delete_sql)
                    .bind(pk.to_string())
                    .execute(pool)
                    .await?;
            }
            
            Ok(())
        } else {
            Err(ModelError::MissingPrimaryKey)
        }
    }
    // <<<ELIF:END agent-editable:model_crud_operations>>>

    // <<<ELIF:BEGIN agent-editable:model_query_methods>>>
    /// Get a query builder for this model
    fn query() -> QueryBuilder<Self>
    where
        Self: Sized,
    {
        let builder = QueryBuilder::new()
            .from(Self::table_name());
        
        // Exclude soft-deleted records by default
        if Self::uses_soft_deletes() {
            builder.where_null("deleted_at")
        } else {
            builder
        }
    }

    /// Get all records for this model
    async fn all(pool: &Pool<Postgres>) -> ModelResult<Vec<Self>>
    where
        Self: Sized,
    {
        let query = Self::query().select("*");
        let sql = query.to_sql();
        
        let rows = sqlx::query(&sql)
            .fetch_all(pool)
            .await?;

        let mut models = Vec::new();
        for row in rows {
            models.push(Self::from_row(&row)?);
        }
        
        Ok(models)
    }

    /// Count all records for this model
    async fn count(pool: &Pool<Postgres>) -> ModelResult<i64>
    where
        Self: Sized,
    {
        let query = Self::query().select("COUNT(*)");
        let sql = query.to_sql();
        
        let row = sqlx::query(&sql)
            .fetch_one(pool)
            .await?;

        let count: i64 = row.try_get(0)?;
        Ok(count)
    }
    // <<<ELIF:END agent-editable:model_query_methods>>>

    /// Create a model instance from a database row
    /// This will be automatically implemented by the derive macro
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self>
    where
        Self: Sized;

    /// Convert model to field-value pairs for database operations
    /// This will be automatically implemented by the derive macro
    fn to_fields(&self) -> HashMap<String, serde_json::Value>;
}

// <<<ELIF:BEGIN agent-editable:model_extensions>>>
/// Extension trait for models with additional utility methods
pub trait ModelExtensions: Model {
    /// Refresh this model instance from the database
    async fn refresh(&mut self, pool: &Pool<Postgres>) -> ModelResult<()>
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
    async fn exists(&self, pool: &Pool<Postgres>) -> ModelResult<bool>
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

    /// Save this model instance (insert or update based on primary key)
    async fn save(&mut self, pool: &Pool<Postgres>) -> ModelResult<()>
    where
        Self: Sized,
    {
        if self.primary_key().is_some() && self.exists(pool).await? {
            // Update existing record
            self.update(pool).await
        } else {
            // For new records, this is a placeholder implementation
            // Real implementation will require derive macro support
            Err(ModelError::Validation("Cannot save new model without primary key support from derive macro".to_string()))
        }
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
    // <<<ELIF:END agent-editable:transaction_methods>>>
}

// Implement ModelExtensions for all types that implement Model
impl<T: Model> ModelExtensions for T {}
// <<<ELIF:END agent-editable:model_extensions>>>