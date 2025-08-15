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
        let sql = format!(
            "SELECT * FROM {} WHERE {} = $1",
            Self::table_name(),
            Self::primary_key_name()
        );

        let row = sqlx::query(&sql)
            .bind(id.to_string())
            .fetch_optional(pool)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to find {}: {}", Self::table_name(), e)))?;

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
        Self::find(pool, id.clone())
            .await?
            .ok_or_else(|| ModelError::NotFound(format!("{}({})", Self::table_name(), id)))
    }

    /// Create a new model instance in the database with field-based insertion
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

        // Get field-value pairs from the model
        let fields = model.to_fields();
        
        if fields.is_empty() {
            // Fallback to DEFAULT VALUES if no fields
            let insert_sql = format!("INSERT INTO {} DEFAULT VALUES RETURNING *", Self::table_name());
            let row = sqlx::query(&insert_sql)
                .fetch_one(pool)
                .await
                .map_err(|e| ModelError::Database(format!("Failed to create {}: {}", Self::table_name(), e)))?;
            
            return Self::from_row(&row);
        }

        // Build dynamic INSERT query with actual field values
        let field_names: Vec<String> = fields.keys().cloned().collect();
        let field_placeholders: Vec<String> = (1..=field_names.len()).map(|i| format!("${}", i)).collect();
        
        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
            Self::table_name(),
            field_names.join(", "),
            field_placeholders.join(", ")
        );
        
        let mut query = sqlx::query(&insert_sql);
        
        // Bind values in the same order as field_names
        for field_name in &field_names {
            if let Some(value) = fields.get(field_name) {
                query = Self::bind_json_value(query, value)?;
            }
        }
        
        let row = query
            .fetch_one(pool)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to create {}: {}", Self::table_name(), e)))?;

        Self::from_row(&row)
    }

    /// Update this model instance in the database with field-based updates
    async fn update(&mut self, pool: &Pool<Postgres>) -> ModelResult<()> {
        if let Some(pk) = self.primary_key() {
            // Set updated_at timestamp if enabled
            if Self::uses_timestamps() {
                self.set_updated_at(Utc::now());
            }

            // Get field-value pairs from the model
            let fields = self.to_fields();
            
            if fields.is_empty() {
                // Fallback to just updating timestamp
                let update_sql = format!(
                    "UPDATE {} SET updated_at = NOW() WHERE {} = $1",
                    Self::table_name(),
                    Self::primary_key_name()
                );
                
                sqlx::query(&update_sql)
                    .bind(pk.to_string())
                    .execute(pool)
                    .await
                    .map_err(|e| ModelError::Database(format!("Failed to update {}: {}", Self::table_name(), e)))?;
                
                return Ok(());
            }

            // Build dynamic UPDATE query with actual field values
            // Filter out primary key from updates
            let pk_name = Self::primary_key_name();
            let update_fields: Vec<String> = fields.keys()
                .filter(|&field| field != pk_name)
                .enumerate()
                .map(|(i, field)| format!("{} = ${}", field, i + 1))
                .collect();
                
            if update_fields.is_empty() {
                // No fields to update
                return Ok(());
            }
            
            let update_sql = format!(
                "UPDATE {} SET {} WHERE {} = ${}",
                Self::table_name(),
                update_fields.join(", "),
                pk_name,
                update_fields.len() + 1
            );
            
            let mut query = sqlx::query(&update_sql);
            
            // Bind update values (excluding primary key)
            for field_name in fields.keys() {
                if field_name != pk_name {
                    if let Some(value) = fields.get(field_name) {
                        query = Self::bind_json_value(query, value)?;
                    }
                }
            }
            
            // Bind primary key for WHERE clause
            query = query.bind(pk.to_string());
            
            query.execute(pool)
                .await
                .map_err(|e| ModelError::Database(format!("Failed to update {}: {}", Self::table_name(), e)))?;

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
                    .await
                    .map_err(|e| ModelError::Database(format!("Failed to soft delete {}: {}", Self::table_name(), e)))?;
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
                    .await
                    .map_err(|e| ModelError::Database(format!("Failed to delete {}: {}", Self::table_name(), e)))?;
            }
            
            Ok(())
        } else {
            Err(ModelError::MissingPrimaryKey)
        }
    }

    /// Helper method to bind JSON values to SQL queries
    fn bind_json_value<'a>(mut query: sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>, value: &serde_json::Value) -> ModelResult<sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>> {
        match value {
            serde_json::Value::Null => Ok(query.bind(None::<String>)),
            serde_json::Value::Bool(b) => Ok(query.bind(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(query.bind(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(query.bind(f))
                } else {
                    Ok(query.bind(n.to_string()))
                }
            }
            serde_json::Value::String(s) => Ok(query.bind(s.clone())),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                // For complex JSON types, bind as JSONB
                Ok(query.bind(sqlx::types::Json(value.clone())))
            }
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