//! Core Model Trait - Base definition for database entities
//!
//! Defines the fundamental Model trait with type requirements, table metadata,
//! primary key handling, timestamp configuration, and serialization contract.

use std::collections::HashMap;
use std::fmt::Debug;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::error::ModelResult;
use crate::backends::DatabaseRow;

/// Core trait for database models with standard ORM operations
pub trait Model: Send + Sync + Debug + Serialize + for<'de> Deserialize<'de> {
    /// The type used for this model's primary key
    type PrimaryKey: Clone + Send + Sync + Debug + std::fmt::Display + Default;

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

    /// Create a model instance from a database row
    /// This will be automatically implemented by the derive macro
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self>
    where
        Self: Sized;

    /// Create a model instance from a database row (abstracted version)
    /// This will replace from_row in the future
    fn from_database_row(row: &dyn DatabaseRow) -> ModelResult<Self>
    where
        Self: Sized,
    {
        // Default implementation that can be overridden
        // For now, this requires concrete implementation by each model
        Err(crate::error::ModelError::Serialization("from_database_row not implemented for this model - still using legacy from_row".to_string()))
    }

    /// Convert model to field-value pairs for database operations
    /// This will be automatically implemented by the derive macro
    fn to_fields(&self) -> HashMap<String, serde_json::Value>;
}