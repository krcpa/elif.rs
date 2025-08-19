//! Database Abstraction - Database-agnostic model operations
//!
//! Provides database-abstracted versions of model operations that work
//! with the DatabasePool trait instead of concrete database types.

use std::sync::Arc;

use crate::error::{ModelError, ModelResult};
use crate::model::core_trait::Model;
use crate::backends::{DatabasePool, DatabaseValue};

/// Trait for database-abstracted model operations
/// This trait provides methods that use the database abstraction layer
/// instead of hardcoded PostgreSQL types
#[allow(async_fn_in_trait)]
pub trait ModelAbstracted: Model {
    /// Find a model by its primary key using database abstraction
    async fn find_abstracted(pool: &Arc<dyn DatabasePool>, id: Self::PrimaryKey) -> ModelResult<Option<Self>>
    where
        Self: Sized,
    {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = $1",
            Self::table_name(),
            Self::primary_key_name()
        );
        let params = vec![DatabaseValue::String(id.to_string())];

        let row = pool.fetch_optional(&sql, &params)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to find {}: {}", Self::table_name(), e)))?;

        match row {
            Some(row) => {
                let model = Self::from_database_row(row.as_ref())?;
                Ok(Some(model))
            }
            None => Ok(None),
        }
    }

    /// Find all models using database abstraction
    async fn all_abstracted(pool: &Arc<dyn DatabasePool>) -> ModelResult<Vec<Self>>
    where
        Self: Sized,
    {
        let sql = if Self::uses_soft_deletes() {
            format!("SELECT * FROM {} WHERE deleted_at IS NULL", Self::table_name())
        } else {
            format!("SELECT * FROM {}", Self::table_name())
        };
        let params = vec![];

        let rows = pool.fetch_all(&sql, &params)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to fetch {}: {}", Self::table_name(), e)))?;

        let mut models = Vec::new();
        for row in rows {
            let model = Self::from_database_row(row.as_ref())?;
            models.push(model);
        }

        Ok(models)
    }

    /// Count models using database abstraction
    async fn count_abstracted(pool: &Arc<dyn DatabasePool>) -> ModelResult<i64>
    where
        Self: Sized,
    {
        let sql = if Self::uses_soft_deletes() {
            format!("SELECT COUNT(*) FROM {} WHERE deleted_at IS NULL", Self::table_name())
        } else {
            format!("SELECT COUNT(*) FROM {}", Self::table_name())
        };
        let params = vec![];

        let row = pool.fetch_optional(&sql, &params)
            .await
            .map_err(|e| ModelError::Database(format!("Failed to count {}: {}", Self::table_name(), e)))?;

        match row {
            Some(row) => {
                let count_value = row.get_by_index(0)
                    .map_err(|e| ModelError::Database(format!("Failed to get count value: {}", e)))?;
                
                match count_value {
                    DatabaseValue::Int64(count) => Ok(count),
                    DatabaseValue::Int32(count) => Ok(count as i64),
                    _ => Err(ModelError::Database("Invalid count value type".to_string())),
                }
            }
            None => Ok(0),
        }
    }

    /// Create a new model using database abstraction
    async fn create_abstracted(pool: &Arc<dyn DatabasePool>, model: Self) -> ModelResult<Self>
    where
        Self: Sized,
    {
        let fields = model.to_fields();
        
        if fields.is_empty() {
            let insert_sql = format!("INSERT INTO {} DEFAULT VALUES RETURNING *", Self::table_name());
            let params = vec![];
            
            let row = pool.fetch_optional(&insert_sql, &params)
                .await
                .map_err(|e| ModelError::Database(format!("Failed to create {}: {}", Self::table_name(), e)))?;
            
            match row {
                Some(row) => Self::from_database_row(row.as_ref()),
                None => Err(ModelError::Database("Failed to get inserted row".to_string())),
            }
        } else {
            let field_names: Vec<String> = fields.keys().cloned().collect();
            let field_placeholders: Vec<String> = (1..=field_names.len()).map(|i| format!("${}", i)).collect();
            
            let insert_sql = format!(
                "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
                Self::table_name(),
                field_names.join(", "),
                field_placeholders.join(", ")
            );
            
            let params: Vec<DatabaseValue> = field_names.iter()
                .filter_map(|name| fields.get(name))
                .map(|value| DatabaseValue::from_json_value(value))
                .collect();
            
            let row = pool.fetch_optional(&insert_sql, &params)
                .await
                .map_err(|e| ModelError::Database(format!("Failed to create {}: {}", Self::table_name(), e)))?;
            
            match row {
                Some(row) => Self::from_database_row(row.as_ref()),
                None => Err(ModelError::Database("Failed to get inserted row".to_string())),
            }
        }
    }

    /// Update a model using database abstraction
    async fn update_abstracted(&self, pool: &Arc<dyn DatabasePool>) -> ModelResult<()>
    where
        Self: Sized,
    {
        if let Some(pk) = self.primary_key() {
            let fields = self.to_fields();
            let pk_name = Self::primary_key_name();
            
            let update_fields: Vec<String> = fields.keys()
                .filter(|&field| field != pk_name)
                .enumerate()
                .map(|(i, field)| format!("{} = ${}", field, i + 1))
                .collect();
                
            if update_fields.is_empty() {
                return Ok(());
            }
            
            let update_sql = format!(
                "UPDATE {} SET {} WHERE {} = ${}",
                Self::table_name(),
                update_fields.join(", "),
                pk_name,
                update_fields.len() + 1
            );
            
            let mut params: Vec<DatabaseValue> = fields.iter()
                .filter(|(field, _)| *field != pk_name)
                .map(|(_, value)| DatabaseValue::from_json_value(value))
                .collect();
            
            params.push(DatabaseValue::String(pk.to_string()));
            
            pool.execute(&update_sql, &params)
                .await
                .map_err(|e| ModelError::Database(format!("Failed to update {}: {}", Self::table_name(), e)))?;

            Ok(())
        } else {
            Err(ModelError::MissingPrimaryKey)
        }
    }

    /// Delete a model using database abstraction  
    async fn delete_abstracted(self, pool: &Arc<dyn DatabasePool>) -> ModelResult<()>
    where
        Self: Sized,
    {
        if let Some(pk) = self.primary_key() {
            let sql = if Self::uses_soft_deletes() {
                format!("UPDATE {} SET deleted_at = NOW() WHERE {} = $1", 
                        Self::table_name(), Self::primary_key_name())
            } else {
                format!("DELETE FROM {} WHERE {} = $1", 
                        Self::table_name(), Self::primary_key_name())
            };
            
            let params = vec![DatabaseValue::String(pk.to_string())];
            
            pool.execute(&sql, &params)
                .await
                .map_err(|e| ModelError::Database(format!("Failed to delete {}: {}", Self::table_name(), e)))?;
            
            Ok(())
        } else {
            Err(ModelError::MissingPrimaryKey)
        }
    }
}

// Implement ModelAbstracted for all types that implement Model
impl<T: Model> ModelAbstracted for T {}

impl DatabaseValue {
    /// Convert a JSON value to a DatabaseValue
    pub fn from_json_value(value: &serde_json::Value) -> DatabaseValue {
        match value {
            serde_json::Value::Null => DatabaseValue::Null,
            serde_json::Value::Bool(b) => DatabaseValue::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    // Check if it fits in i32 range
                    if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                        DatabaseValue::Int32(i as i32)
                    } else {
                        DatabaseValue::Int64(i)
                    }
                } else if let Some(f) = n.as_f64() {
                    DatabaseValue::Float64(f)
                } else {
                    DatabaseValue::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => DatabaseValue::String(s.clone()),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                DatabaseValue::Json(value.clone())
            }
        }
    }
}