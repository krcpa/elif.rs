//! CRUD Operations - Create, Read, Update, Delete operations for models
//!
//! Implements the core database operations with proper parameter binding,
//! timestamp management, soft delete support, and error handling.

use std::collections::HashMap;
use chrono::Utc;
use sqlx::{Pool, Postgres};
use serde_json::Value;

use crate::error::{ModelError, ModelResult};
use crate::model::core_trait::Model;

/// Trait providing CRUD operations for models
pub trait CrudOperations: Model {
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
    fn bind_json_value<'a>(mut query: sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>, value: &Value) -> ModelResult<sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>> {
        match value {
            Value::Null => Ok(query.bind(None::<String>)),
            Value::Bool(b) => Ok(query.bind(*b)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(query.bind(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(query.bind(f))
                } else {
                    Ok(query.bind(n.to_string()))
                }
            }
            Value::String(s) => Ok(query.bind(s.clone())),
            Value::Array(_) | Value::Object(_) => {
                // For complex JSON types, bind as JSONB
                Ok(query.bind(sqlx::types::Json(value.clone())))
            }
        }
    }
    // <<<ELIF:END agent-editable:model_crud_operations>>>
}

// Implement CrudOperations for all types that implement Model
impl<T: Model> CrudOperations for T {}