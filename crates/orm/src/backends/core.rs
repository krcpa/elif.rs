//! Core Database Backend Traits
//! 
//! This module defines the core traits and types for database backend abstraction.
//! These traits abstract away database-specific implementations and provide a unified
//! interface for the ORM to work with different database systems.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use crate::error::{OrmResult, OrmError};

/// Abstract database connection trait
#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    /// Execute a query and return affected rows count
    async fn execute(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<u64>;
    
    /// Execute a query and return the result rows
    async fn fetch_all(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Vec<Box<dyn DatabaseRow>>>;
    
    /// Execute a query and return the first result row
    async fn fetch_optional(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Option<Box<dyn DatabaseRow>>>;
    
    /// Begin a transaction
    async fn begin_transaction(&mut self) -> OrmResult<Box<dyn DatabaseTransaction>>;
    
    /// Close the connection
    async fn close(&mut self) -> OrmResult<()>;
}

/// Abstract database transaction trait
#[async_trait]
pub trait DatabaseTransaction: Send + Sync {
    /// Execute a query within the transaction
    async fn execute(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<u64>;
    
    /// Execute a query and return result rows within the transaction
    async fn fetch_all(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Vec<Box<dyn DatabaseRow>>>;
    
    /// Execute a query and return the first result row within the transaction
    async fn fetch_optional(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Option<Box<dyn DatabaseRow>>>;
    
    /// Commit the transaction
    async fn commit(self: Box<Self>) -> OrmResult<()>;
    
    /// Rollback the transaction
    async fn rollback(self: Box<Self>) -> OrmResult<()>;
}

/// Abstract database connection pool trait
#[async_trait]
pub trait DatabasePool: Send + Sync {
    /// Acquire a connection from the pool
    async fn acquire(&self) -> OrmResult<Box<dyn DatabaseConnection>>;
    
    /// Begin a transaction from the pool
    async fn begin_transaction(&self) -> OrmResult<Box<dyn DatabaseTransaction>>;
    
    /// Execute a query directly on the pool
    async fn execute(&self, sql: &str, params: &[DatabaseValue]) -> OrmResult<u64>;
    
    /// Execute a query and return result rows directly on the pool
    async fn fetch_all(&self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Vec<Box<dyn DatabaseRow>>>;
    
    /// Execute a query and return the first result row directly on the pool
    async fn fetch_optional(&self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Option<Box<dyn DatabaseRow>>>;
    
    /// Close the pool
    async fn close(&self) -> OrmResult<()>;
    
    /// Get pool statistics
    fn stats(&self) -> DatabasePoolStats;
    
    /// Perform a health check on the pool
    async fn health_check(&self) -> OrmResult<std::time::Duration>;
}

/// Database pool statistics
#[derive(Debug, Clone)]
pub struct DatabasePoolStats {
    pub total_connections: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
}

/// Abstract database row trait
pub trait DatabaseRow: Send + Sync {
    /// Get a column value by index
    fn get_by_index(&self, index: usize) -> OrmResult<DatabaseValue>;
    
    /// Get a column value by name
    fn get_by_name(&self, name: &str) -> OrmResult<DatabaseValue>;
    
    /// Get column count
    fn column_count(&self) -> usize;
    
    /// Get column names
    fn column_names(&self) -> Vec<String>;
    
    /// Convert row to JSON value
    fn to_json(&self) -> OrmResult<JsonValue>;
    
    /// Convert row to HashMap
    fn to_map(&self) -> OrmResult<HashMap<String, DatabaseValue>>;
}

/// Extension trait for DatabaseRow to support typed column access for models
pub trait DatabaseRowExt {
    /// Get a typed value from a column (for model deserialization)
    fn get<T>(&self, column: &str) -> Result<T, crate::error::ModelError>
    where
        T: for<'de> serde::Deserialize<'de>;
    
    /// Try to get an optional typed value from a column
    fn try_get<T>(&self, column: &str) -> Result<Option<T>, crate::error::ModelError>
    where
        T: for<'de> serde::Deserialize<'de>;
}

impl<R: DatabaseRow + ?Sized> DatabaseRowExt for R {
    fn get<T>(&self, column: &str) -> Result<T, crate::error::ModelError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let db_value = self.get_by_name(column)?;
        
        let json_value = db_value.to_json();
        serde_json::from_value(json_value)
            .map_err(|e| crate::error::ModelError::Serialization(format!("Failed to deserialize column '{}': {}", column, e)))
    }
    
    fn try_get<T>(&self, column: &str) -> Result<Option<T>, crate::error::ModelError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        match self.get_by_name(column) {
            Ok(db_value) => {
                if db_value.is_null() {
                    Ok(None)
                } else {
                    let json_value = db_value.to_json();
                    let parsed: T = serde_json::from_value(json_value)
                        .map_err(|e| crate::error::ModelError::Serialization(format!("Failed to deserialize column '{}': {}", column, e)))?;
                    Ok(Some(parsed))
                }
            },
            Err(crate::error::ModelError::ColumnNotFound(_)) => Ok(None),
            Err(e) => Err(e), // Preserve the original error type and information
        }
    }
}

/// Database value enumeration for type-safe parameter binding
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseValue {
    Null,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
    Bytes(Vec<u8>),
    Uuid(uuid::Uuid),
    DateTime(chrono::DateTime<chrono::Utc>),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    Json(JsonValue),
    Array(Vec<DatabaseValue>),
}

impl DatabaseValue {
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, DatabaseValue::Null)
    }
    
    /// Convert to JSON value
    pub fn to_json(&self) -> JsonValue {
        match self {
            DatabaseValue::Null => JsonValue::Null,
            DatabaseValue::Bool(b) => JsonValue::Bool(*b),
            DatabaseValue::Int32(i) => JsonValue::Number(serde_json::Number::from(*i)),
            DatabaseValue::Int64(i) => JsonValue::Number(serde_json::Number::from(*i)),
            DatabaseValue::Float32(f) => {
                JsonValue::Number(serde_json::Number::from_f64(*f as f64).unwrap_or_else(|| serde_json::Number::from(0)))
            },
            DatabaseValue::Float64(f) => {
                serde_json::Number::from_f64(*f)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null)
            },
            DatabaseValue::String(s) => JsonValue::String(s.clone()),
            DatabaseValue::Bytes(b) => JsonValue::Array(b.iter().map(|&x| JsonValue::Number(serde_json::Number::from(x))).collect()),
            DatabaseValue::Uuid(u) => JsonValue::String(u.to_string()),
            DatabaseValue::DateTime(dt) => JsonValue::String(dt.to_rfc3339()),
            DatabaseValue::Date(d) => JsonValue::String(d.to_string()),
            DatabaseValue::Time(t) => JsonValue::String(t.to_string()),
            DatabaseValue::Json(j) => j.clone(),
            DatabaseValue::Array(arr) => JsonValue::Array(arr.iter().map(|v| v.to_json()).collect()),
        }
    }

    /// Create DatabaseValue from JSON value
    pub fn from_json(json: JsonValue) -> Self {
        match json {
            JsonValue::Null => DatabaseValue::Null,
            JsonValue::Bool(b) => DatabaseValue::Bool(b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                        DatabaseValue::Int32(i as i32)
                    } else {
                        DatabaseValue::Int64(i)
                    }
                } else if let Some(f) = n.as_f64() {
                    DatabaseValue::Float64(f)
                } else {
                    DatabaseValue::Null
                }
            },
            JsonValue::String(s) => {
                // Try to parse as UUID first
                if let Ok(uuid) = uuid::Uuid::parse_str(&s) {
                    DatabaseValue::Uuid(uuid)
                } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                    DatabaseValue::DateTime(dt.with_timezone(&chrono::Utc))
                } else {
                    DatabaseValue::String(s)
                }
            },
            JsonValue::Array(arr) => {
                let db_values: Vec<DatabaseValue> = arr.into_iter().map(DatabaseValue::from_json).collect();
                DatabaseValue::Array(db_values)
            },
            JsonValue::Object(_) => DatabaseValue::Json(json),
        }
    }
}

impl From<bool> for DatabaseValue {
    fn from(value: bool) -> Self {
        DatabaseValue::Bool(value)
    }
}

impl From<i32> for DatabaseValue {
    fn from(value: i32) -> Self {
        DatabaseValue::Int32(value)
    }
}

impl From<i64> for DatabaseValue {
    fn from(value: i64) -> Self {
        DatabaseValue::Int64(value)
    }
}

impl From<f32> for DatabaseValue {
    fn from(value: f32) -> Self {
        DatabaseValue::Float32(value)
    }
}

impl From<f64> for DatabaseValue {
    fn from(value: f64) -> Self {
        DatabaseValue::Float64(value)
    }
}

impl From<String> for DatabaseValue {
    fn from(value: String) -> Self {
        DatabaseValue::String(value)
    }
}

impl From<&str> for DatabaseValue {
    fn from(value: &str) -> Self {
        DatabaseValue::String(value.to_string())
    }
}

impl From<Vec<u8>> for DatabaseValue {
    fn from(value: Vec<u8>) -> Self {
        DatabaseValue::Bytes(value)
    }
}

impl From<uuid::Uuid> for DatabaseValue {
    fn from(value: uuid::Uuid) -> Self {
        DatabaseValue::Uuid(value)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for DatabaseValue {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        DatabaseValue::DateTime(value)
    }
}

impl From<chrono::NaiveDate> for DatabaseValue {
    fn from(value: chrono::NaiveDate) -> Self {
        DatabaseValue::Date(value)
    }
}

impl From<chrono::NaiveTime> for DatabaseValue {
    fn from(value: chrono::NaiveTime) -> Self {
        DatabaseValue::Time(value)
    }
}

impl From<JsonValue> for DatabaseValue {
    fn from(value: JsonValue) -> Self {
        DatabaseValue::Json(value)
    }
}

impl<T> From<Option<T>> for DatabaseValue
where
    T: Into<DatabaseValue>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => v.into(),
            None => DatabaseValue::Null,
        }
    }
}

/// SQL dialect enumeration for generating database-specific SQL
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlDialect {
    PostgreSQL,
    MySQL,
    SQLite,
}

impl SqlDialect {
    /// Get the parameter placeholder style for this dialect
    pub fn parameter_placeholder(&self, index: usize) -> String {
        match self {
            SqlDialect::PostgreSQL => format!("${}", index + 1),
            SqlDialect::MySQL | SqlDialect::SQLite => "?".to_string(),
        }
    }
    
    /// Get the quote character for identifiers in this dialect
    pub fn identifier_quote(&self) -> char {
        match self {
            SqlDialect::PostgreSQL => '"',
            SqlDialect::MySQL => '`',
            SqlDialect::SQLite => '"',
        }
    }
    
    /// Check if this dialect supports boolean types
    pub fn supports_boolean(&self) -> bool {
        match self {
            SqlDialect::PostgreSQL | SqlDialect::SQLite => true,
            SqlDialect::MySQL => false,
        }
    }
    
    /// Check if this dialect supports JSON types
    pub fn supports_json(&self) -> bool {
        match self {
            SqlDialect::PostgreSQL | SqlDialect::MySQL => true,
            SqlDialect::SQLite => false,
        }
    }
    
    /// Get the current timestamp function for this dialect
    pub fn current_timestamp(&self) -> &'static str {
        match self {
            SqlDialect::PostgreSQL => "NOW()",
            SqlDialect::MySQL => "CURRENT_TIMESTAMP",
            SqlDialect::SQLite => "datetime('now')",
        }
    }
    
    /// Get the auto-increment column definition for this dialect
    pub fn auto_increment(&self) -> &'static str {
        match self {
            SqlDialect::PostgreSQL => "SERIAL",
            SqlDialect::MySQL => "AUTO_INCREMENT",
            SqlDialect::SQLite => "AUTOINCREMENT",
        }
    }
}

/// Database backend trait that provides database-specific implementations
#[async_trait]
pub trait DatabaseBackend: Send + Sync {
    /// Create a connection pool from a database URL
    async fn create_pool(&self, database_url: &str, config: DatabasePoolConfig) -> OrmResult<Arc<dyn DatabasePool>>;
    
    /// Get the SQL dialect used by this backend
    fn sql_dialect(&self) -> SqlDialect;
    
    /// Get the backend type
    fn backend_type(&self) -> crate::backends::DatabaseBackendType;
    
    /// Validate a database URL for this backend
    fn validate_database_url(&self, url: &str) -> OrmResult<()>;
    
    /// Parse connection parameters from a database URL
    fn parse_database_url(&self, url: &str) -> OrmResult<DatabaseConnectionConfig>;
}

/// Database pool configuration
#[derive(Debug, Clone)]
pub struct DatabasePoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: Option<u64>,
    pub max_lifetime_seconds: Option<u64>,
    pub test_before_acquire: bool,
}

impl Default for DatabasePoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_seconds: 30,
            idle_timeout_seconds: Some(600), // 10 minutes
            max_lifetime_seconds: Some(1800), // 30 minutes
            test_before_acquire: true,
        }
    }
}

/// Database connection configuration parsed from URL
#[derive(Debug, Clone)]
pub struct DatabaseConnectionConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub ssl_mode: Option<String>,
    pub additional_params: HashMap<String, String>,
}

/// Database backend registry for managing multiple backend implementations
pub struct DatabaseBackendRegistry {
    backends: HashMap<crate::backends::DatabaseBackendType, Arc<dyn DatabaseBackend>>,
}

impl DatabaseBackendRegistry {
    /// Create a new backend registry
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }
    
    /// Register a database backend
    pub fn register(&mut self, backend_type: crate::backends::DatabaseBackendType, backend: Arc<dyn DatabaseBackend>) {
        self.backends.insert(backend_type, backend);
    }
    
    /// Get a database backend by type
    pub fn get(&self, backend_type: &crate::backends::DatabaseBackendType) -> Option<Arc<dyn DatabaseBackend>> {
        self.backends.get(backend_type).cloned()
    }
    
    /// Create a connection pool using the appropriate backend for the given URL
    pub async fn create_pool(&self, database_url: &str, config: DatabasePoolConfig) -> OrmResult<Arc<dyn DatabasePool>> {
        let backend_type = self.detect_backend_from_url(database_url)?;
        let backend = self.get(&backend_type)
            .ok_or_else(|| OrmError::Connection(format!("No backend registered for {}", backend_type)))?;
        
        backend.create_pool(database_url, config).await
    }
    
    /// Detect database backend type from URL
    fn detect_backend_from_url(&self, url: &str) -> OrmResult<crate::backends::DatabaseBackendType> {
        if url.starts_with("postgresql://") || url.starts_with("postgres://") {
            Ok(crate::backends::DatabaseBackendType::PostgreSQL)
        } else if url.starts_with("mysql://") {
            Ok(crate::backends::DatabaseBackendType::MySQL)
        } else if url.starts_with("sqlite://") || url.starts_with("file:") {
            Ok(crate::backends::DatabaseBackendType::SQLite)
        } else {
            Err(OrmError::Connection(format!("Unable to detect database backend from URL: {}", url)))
        }
    }
    
    /// List all registered backend types
    pub fn registered_backends(&self) -> Vec<crate::backends::DatabaseBackendType> {
        self.backends.keys().cloned().collect()
    }
}

impl Default for DatabaseBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}