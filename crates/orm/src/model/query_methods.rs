//! Query Methods - Collection and batch query operations for models
//!
//! Provides methods for querying multiple records, filtering, counting,
//! and other collection-based database operations.

use sqlx::{Pool, Postgres, Row};

use crate::error::{ModelError, ModelResult};
use crate::model::core_trait::Model;
use crate::query::QueryBuilder;

/// Trait providing query operations for model collections
pub trait QueryMethods: Model {
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

    /// Get all records for this model with proper SQL execution
    async fn all(pool: &Pool<Postgres>) -> ModelResult<Vec<Self>>
    where
        Self: Sized,
    {
        let sql = if Self::uses_soft_deletes() {
            format!("SELECT * FROM {} WHERE deleted_at IS NULL", Self::table_name())
        } else {
            format!("SELECT * FROM {}", Self::table_name())
        };
        
        let rows = sqlx::query(&sql)
            .fetch_all(pool)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to fetch all from {}: {}", Self::table_name(), e)))?;

        let mut models = Vec::new();
        for row in rows {
            models.push(Self::from_row(&row)?);
        }
        
        Ok(models)
    }

    /// Count all records for this model with proper SQL execution
    async fn count(pool: &Pool<Postgres>) -> ModelResult<i64>
    where
        Self: Sized,
    {
        let sql = if Self::uses_soft_deletes() {
            format!("SELECT COUNT(*) FROM {} WHERE deleted_at IS NULL", Self::table_name())
        } else {
            format!("SELECT COUNT(*) FROM {}", Self::table_name())
        };
        
        let row = sqlx::query(&sql)
            .fetch_one(pool)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to count {}: {}", Self::table_name(), e)))?;

        let count: i64 = row.try_get(0)
            .map_err(|e| ModelError::Database(format!("Failed to extract count: {}", e)))?;
        Ok(count)
    }

    /// Find models by a specific field value with proper parameter binding
    async fn where_field<V>(pool: &Pool<Postgres>, field: &str, value: V) -> ModelResult<Vec<Self>>
    where
        Self: Sized,
        V: Send + Sync + 'static,
        for<'q> V: sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres>,
    {
        let sql = if Self::uses_soft_deletes() {
            format!("SELECT * FROM {} WHERE {} = $1 AND deleted_at IS NULL", Self::table_name(), field)
        } else {
            format!("SELECT * FROM {} WHERE {} = $1", Self::table_name(), field)
        };
        
        let rows = sqlx::query(&sql)
            .bind(value)
            .fetch_all(pool)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to query {} by {}: {}", Self::table_name(), field, e)))?;

        let mut models = Vec::new();
        for row in rows {
            models.push(Self::from_row(&row)?);
        }
        
        Ok(models)
    }

    /// Find the first model by a specific field value
    async fn first_where<V>(pool: &Pool<Postgres>, field: &str, value: V) -> ModelResult<Option<Self>>
    where
        Self: Sized,
        V: Send + Sync + 'static,
        for<'q> V: sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres>,
    {
        let sql = if Self::uses_soft_deletes() {
            format!("SELECT * FROM {} WHERE {} = $1 AND deleted_at IS NULL LIMIT 1", Self::table_name(), field)
        } else {
            format!("SELECT * FROM {} WHERE {} = $1 LIMIT 1", Self::table_name(), field)
        };
        
        let row = sqlx::query(&sql)
            .bind(value)
            .fetch_optional(pool)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to find first {} by {}: {}", Self::table_name(), field, e)))?;

        match row {
            Some(row) => {
                let model = Self::from_row(&row)?;
                Ok(Some(model))
            }
            None => Ok(None),
        }
    }

    /// Get the first record from this model
    async fn first(pool: &Pool<Postgres>) -> ModelResult<Option<Self>>
    where
        Self: Sized,
    {
        let sql = if Self::uses_soft_deletes() {
            format!("SELECT * FROM {} WHERE deleted_at IS NULL LIMIT 1", Self::table_name())
        } else {
            format!("SELECT * FROM {} LIMIT 1", Self::table_name())
        };
        
        let row = sqlx::query(&sql)
            .fetch_optional(pool)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to get first {}: {}", Self::table_name(), e)))?;

        match row {
            Some(row) => {
                let model = Self::from_row(&row)?;
                Ok(Some(model))
            }
            None => Ok(None),
        }
    }

    /// Get the last record from this model (ordered by primary key)
    async fn last(pool: &Pool<Postgres>) -> ModelResult<Option<Self>>
    where
        Self: Sized,
    {
        let sql = if Self::uses_soft_deletes() {
            format!("SELECT * FROM {} WHERE deleted_at IS NULL ORDER BY {} DESC LIMIT 1", 
                    Self::table_name(), Self::primary_key_name())
        } else {
            format!("SELECT * FROM {} ORDER BY {} DESC LIMIT 1", 
                    Self::table_name(), Self::primary_key_name())
        };
        
        let row = sqlx::query(&sql)
            .fetch_optional(pool)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to get last {}: {}", Self::table_name(), e)))?;

        match row {
            Some(row) => {
                let model = Self::from_row(&row)?;
                Ok(Some(model))
            }
            None => Ok(None),
        }
    }
    // <<<ELIF:END agent-editable:model_query_methods>>>
}

// Implement QueryMethods for all types that implement Model
impl<T: Model> QueryMethods for T {}