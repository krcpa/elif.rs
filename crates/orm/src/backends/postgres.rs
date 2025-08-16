//! PostgreSQL Backend Implementation
//! 
//! This module provides the PostgreSQL-specific implementation of the database
//! backend traits using sqlx as the underlying database driver.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use sqlx::{Pool, Postgres, Row as SqlxRow, postgres::PgPoolOptions, Column};
use serde_json::Value as JsonValue;
use crate::error::{OrmResult, OrmError};
use super::core::*;

/// PostgreSQL database backend implementation
#[derive(Debug)]
pub struct PostgresBackend;

impl PostgresBackend {
    /// Create a new PostgreSQL backend instance
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DatabaseBackend for PostgresBackend {
    async fn create_pool(&self, database_url: &str, config: DatabasePoolConfig) -> OrmResult<Arc<dyn DatabasePool>> {
        let mut options = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.acquire_timeout_seconds))
            .test_before_acquire(config.test_before_acquire);

        if let Some(idle_timeout) = config.idle_timeout_seconds {
            options = options.idle_timeout(std::time::Duration::from_secs(idle_timeout));
        }

        if let Some(max_lifetime) = config.max_lifetime_seconds {
            options = options.max_lifetime(std::time::Duration::from_secs(max_lifetime));
        }

        let sqlx_pool = options.connect(database_url)
            .await
            .map_err(|e| OrmError::Connection(format!("Failed to create PostgreSQL pool: {}", e)))?;

        Ok(Arc::new(PostgresPool::new(Arc::new(sqlx_pool))))
    }

    fn sql_dialect(&self) -> SqlDialect {
        SqlDialect::PostgreSQL
    }

    fn backend_type(&self) -> crate::backends::DatabaseBackendType {
        crate::backends::DatabaseBackendType::PostgreSQL
    }

    fn validate_database_url(&self, url: &str) -> OrmResult<()> {
        if !url.starts_with("postgresql://") && !url.starts_with("postgres://") {
            return Err(OrmError::Connection("Invalid PostgreSQL URL scheme".to_string()));
        }
        Ok(())
    }

    fn parse_database_url(&self, url: &str) -> OrmResult<DatabaseConnectionConfig> {
        // Basic URL parsing for PostgreSQL
        let parsed = url::Url::parse(url)
            .map_err(|e| OrmError::Connection(format!("Invalid database URL: {}", e)))?;

        let host = parsed.host_str()
            .ok_or_else(|| OrmError::Connection("Missing host in database URL".to_string()))?
            .to_string();

        let port = parsed.port().unwrap_or(5432);
        
        let database = parsed.path().trim_start_matches('/').to_string();
        if database.is_empty() {
            return Err(OrmError::Connection("Missing database name in URL".to_string()));
        }

        let username = if parsed.username().is_empty() {
            None
        } else {
            Some(parsed.username().to_string())
        };

        let password = parsed.password().map(|p| p.to_string());

        let mut additional_params = HashMap::new();
        for (key, value) in parsed.query_pairs() {
            additional_params.insert(key.to_string(), value.to_string());
        }

        let ssl_mode = additional_params.get("sslmode").cloned();

        Ok(DatabaseConnectionConfig {
            host,
            port,
            database,
            username,
            password,
            ssl_mode,
            additional_params,
        })
    }
}

/// PostgreSQL connection pool implementation
pub struct PostgresPool {
    pool: Arc<Pool<Postgres>>,
}

impl PostgresPool {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DatabasePool for PostgresPool {
    async fn acquire(&self) -> OrmResult<Box<dyn DatabaseConnection>> {
        let conn = self.pool.acquire()
            .await
            .map_err(|e| OrmError::Connection(format!("Failed to acquire connection: {}", e)))?;
        
        Ok(Box::new(PostgresConnection::new(conn)))
    }

    async fn begin_transaction(&self) -> OrmResult<Box<dyn DatabaseTransaction>> {
        // For now, we'll use a simpler approach that doesn't require explicit lifetimes
        Err(OrmError::Query("Transaction support not yet fully implemented in abstraction layer".to_string()))
    }

    async fn execute(&self, sql: &str, params: &[DatabaseValue]) -> OrmResult<u64> {
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let result = query.execute(&*self.pool)
            .await
            .map_err(|e| OrmError::Query(format!("Query execution failed: {}", e)))?;
        
        Ok(result.rows_affected())
    }

    async fn fetch_all(&self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Vec<Box<dyn DatabaseRow>>> {
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let rows = query.fetch_all(&*self.pool)
            .await
            .map_err(|e| OrmError::Query(format!("Query fetch failed: {}", e)))?;
        
        Ok(rows.into_iter().map(|row| Box::new(PostgresRow::new(row)) as Box<dyn DatabaseRow>).collect())
    }

    async fn fetch_optional(&self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Option<Box<dyn DatabaseRow>>> {
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let row = query.fetch_optional(&*self.pool)
            .await
            .map_err(|e| OrmError::Query(format!("Query fetch failed: {}", e)))?;
        
        Ok(row.map(|r| Box::new(PostgresRow::new(r)) as Box<dyn DatabaseRow>))
    }

    async fn close(&self) -> OrmResult<()> {
        self.pool.close().await;
        Ok(())
    }

    fn stats(&self) -> DatabasePoolStats {
        let total = self.pool.size() as u32;
        let idle = self.pool.num_idle() as u32;
        let active = total.saturating_sub(idle);

        DatabasePoolStats {
            total_connections: total,
            idle_connections: idle,
            active_connections: active,
        }
    }

    async fn health_check(&self) -> OrmResult<std::time::Duration> {
        let start = std::time::Instant::now();
        
        sqlx::query("SELECT 1")
            .execute(&*self.pool)
            .await
            .map_err(|e| OrmError::Connection(format!("Health check failed: {}", e)))?;
        
        Ok(start.elapsed())
    }
}

/// PostgreSQL connection implementation
pub struct PostgresConnection {
    conn: sqlx::pool::PoolConnection<Postgres>,
}

impl PostgresConnection {
    pub fn new(conn: sqlx::pool::PoolConnection<Postgres>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl DatabaseConnection for PostgresConnection {
    async fn execute(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<u64> {
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let result = query.execute(&mut *self.conn)
            .await
            .map_err(|e| OrmError::Query(format!("Query execution failed: {}", e)))?;
        
        Ok(result.rows_affected())
    }

    async fn fetch_all(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Vec<Box<dyn DatabaseRow>>> {
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let rows = query.fetch_all(&mut *self.conn)
            .await
            .map_err(|e| OrmError::Query(format!("Query fetch failed: {}", e)))?;
        
        Ok(rows.into_iter().map(|row| Box::new(PostgresRow::new(row)) as Box<dyn DatabaseRow>).collect())
    }

    async fn fetch_optional(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Option<Box<dyn DatabaseRow>>> {
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let row = query.fetch_optional(&mut *self.conn)
            .await
            .map_err(|e| OrmError::Query(format!("Query fetch failed: {}", e)))?;
        
        Ok(row.map(|r| Box::new(PostgresRow::new(r)) as Box<dyn DatabaseRow>))
    }

    async fn begin_transaction(&mut self) -> OrmResult<Box<dyn DatabaseTransaction>> {
        // For now, we'll use a simpler approach that doesn't require explicit lifetimes
        Err(OrmError::Query("Transaction support not yet fully implemented in abstraction layer".to_string()))
    }

    async fn close(&mut self) -> OrmResult<()> {
        // Connection will be returned to pool automatically when dropped
        Ok(())
    }
}

/// PostgreSQL transaction implementation
pub struct PostgresTransaction<'c> {
    tx: Option<sqlx::Transaction<'c, Postgres>>,
}

impl<'c> PostgresTransaction<'c> {
    pub fn new(tx: sqlx::Transaction<'c, Postgres>) -> Self {
        Self { tx: Some(tx) }
    }
}

#[async_trait]
impl<'c> DatabaseTransaction for PostgresTransaction<'c> {
    async fn execute(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<u64> {
        let tx = self.tx.as_mut().ok_or_else(|| OrmError::Query("Transaction already completed".to_string()))?;
        
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let result = query.execute(&mut **tx)
            .await
            .map_err(|e| OrmError::Query(format!("Query execution failed: {}", e)))?;
        
        Ok(result.rows_affected())
    }

    async fn fetch_all(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Vec<Box<dyn DatabaseRow>>> {
        let tx = self.tx.as_mut().ok_or_else(|| OrmError::Query("Transaction already completed".to_string()))?;
        
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let rows = query.fetch_all(&mut **tx)
            .await
            .map_err(|e| OrmError::Query(format!("Query fetch failed: {}", e)))?;
        
        Ok(rows.into_iter().map(|row| Box::new(PostgresRow::new(row)) as Box<dyn DatabaseRow>).collect())
    }

    async fn fetch_optional(&mut self, sql: &str, params: &[DatabaseValue]) -> OrmResult<Option<Box<dyn DatabaseRow>>> {
        let tx = self.tx.as_mut().ok_or_else(|| OrmError::Query("Transaction already completed".to_string()))?;
        
        let mut query = sqlx::query(sql);
        
        for param in params {
            query = bind_database_value(query, param)?;
        }
        
        let row = query.fetch_optional(&mut **tx)
            .await
            .map_err(|e| OrmError::Query(format!("Query fetch failed: {}", e)))?;
        
        Ok(row.map(|r| Box::new(PostgresRow::new(r)) as Box<dyn DatabaseRow>))
    }

    async fn commit(mut self: Box<Self>) -> OrmResult<()> {
        let tx = self.tx.take().ok_or_else(|| OrmError::Query("Transaction already completed".to_string()))?;
        
        tx.commit()
            .await
            .map_err(|e| OrmError::Query(format!("Transaction commit failed: {}", e)))?;
        
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> OrmResult<()> {
        let tx = self.tx.take().ok_or_else(|| OrmError::Query("Transaction already completed".to_string()))?;
        
        tx.rollback()
            .await
            .map_err(|e| OrmError::Query(format!("Transaction rollback failed: {}", e)))?;
        
        Ok(())
    }
}

/// PostgreSQL row implementation
pub struct PostgresRow {
    row: sqlx::postgres::PgRow,
}

impl PostgresRow {
    pub fn new(row: sqlx::postgres::PgRow) -> Self {
        Self { row }
    }
}

impl DatabaseRow for PostgresRow {
    fn get_by_index(&self, index: usize) -> OrmResult<DatabaseValue> {
        postgres_value_to_database_value(&self.row, index)
    }

    fn get_by_name(&self, name: &str) -> OrmResult<DatabaseValue> {
        // Find the column index by name
        let columns = self.row.columns();
        let index = columns.iter().position(|col| col.name() == name)
            .ok_or_else(|| OrmError::Query(format!("Column '{}' not found", name)))?;
        
        postgres_value_to_database_value(&self.row, index)
    }

    fn column_count(&self) -> usize {
        self.row.len()
    }

    fn column_names(&self) -> Vec<String> {
        self.row.columns().iter().map(|col| col.name().to_string()).collect()
    }

    fn to_json(&self) -> OrmResult<JsonValue> {
        let mut map = serde_json::Map::new();
        
        for (i, column) in self.row.columns().iter().enumerate() {
            let value = self.get_by_index(i)?;
            map.insert(column.name().to_string(), value.to_json());
        }
        
        Ok(JsonValue::Object(map))
    }

    fn to_map(&self) -> OrmResult<HashMap<String, DatabaseValue>> {
        let mut map = HashMap::new();
        
        for (i, column) in self.row.columns().iter().enumerate() {
            let value = self.get_by_index(i)?;
            map.insert(column.name().to_string(), value);
        }
        
        Ok(map)
    }
}

/// Bind a DatabaseValue to a sqlx query
fn bind_database_value<'a>(
    query: sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>,
    value: &DatabaseValue
) -> OrmResult<sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>> {
    match value {
        DatabaseValue::Null => Ok(query.bind(Option::<String>::None)),
        DatabaseValue::Bool(b) => Ok(query.bind(*b)),
        DatabaseValue::Int32(i) => Ok(query.bind(*i)),
        DatabaseValue::Int64(i) => Ok(query.bind(*i)),
        DatabaseValue::Float32(f) => Ok(query.bind(*f)),
        DatabaseValue::Float64(f) => Ok(query.bind(*f)),
        DatabaseValue::String(s) => Ok(query.bind(s.clone())),
        DatabaseValue::Bytes(b) => Ok(query.bind(b.clone())),
        DatabaseValue::Uuid(u) => Ok(query.bind(*u)),
        DatabaseValue::DateTime(dt) => Ok(query.bind(*dt)),
        DatabaseValue::Date(d) => Ok(query.bind(*d)),
        DatabaseValue::Time(t) => Ok(query.bind(*t)),
        DatabaseValue::Json(j) => Ok(query.bind(j.clone())),
        DatabaseValue::Array(_) => Err(OrmError::Query("Array binding not yet implemented for PostgreSQL".to_string())),
    }
}

/// Convert a PostgreSQL column value to DatabaseValue
fn postgres_value_to_database_value(row: &sqlx::postgres::PgRow, index: usize) -> OrmResult<DatabaseValue> {
    use sqlx::{Row, Column, TypeInfo};
    
    let column = &row.columns()[index];
    let type_name = column.type_info().name();
    
    // Handle null values
    if let Ok(Some(value)) = row.try_get::<Option<String>, _>(index) {
        return Ok(DatabaseValue::String(value));
    }
    
    match type_name {
        "BOOL" => {
            let value: bool = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get bool value: {}", e)))?;
            Ok(DatabaseValue::Bool(value))
        },
        "INT2" => {
            let value: i16 = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get int16 value: {}", e)))?;
            Ok(DatabaseValue::Int32(value as i32))
        },
        "INT4" => {
            let value: i32 = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get int32 value: {}", e)))?;
            Ok(DatabaseValue::Int32(value))
        },
        "INT8" => {
            let value: i64 = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get int64 value: {}", e)))?;
            Ok(DatabaseValue::Int64(value))
        },
        "FLOAT4" => {
            let value: f32 = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get float32 value: {}", e)))?;
            Ok(DatabaseValue::Float32(value))
        },
        "FLOAT8" => {
            let value: f64 = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get float64 value: {}", e)))?;
            Ok(DatabaseValue::Float64(value))
        },
        "TEXT" | "VARCHAR" => {
            let value: String = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get string value: {}", e)))?;
            Ok(DatabaseValue::String(value))
        },
        "BYTEA" => {
            let value: Vec<u8> = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get bytes value: {}", e)))?;
            Ok(DatabaseValue::Bytes(value))
        },
        "UUID" => {
            let value: uuid::Uuid = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get UUID value: {}", e)))?;
            Ok(DatabaseValue::Uuid(value))
        },
        "TIMESTAMPTZ" | "TIMESTAMP" => {
            let value: chrono::DateTime<chrono::Utc> = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get datetime value: {}", e)))?;
            Ok(DatabaseValue::DateTime(value))
        },
        "DATE" => {
            let value: chrono::NaiveDate = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get date value: {}", e)))?;
            Ok(DatabaseValue::Date(value))
        },
        "TIME" => {
            let value: chrono::NaiveTime = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get time value: {}", e)))?;
            Ok(DatabaseValue::Time(value))
        },
        "JSON" | "JSONB" => {
            let value: JsonValue = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get JSON value: {}", e)))?;
            Ok(DatabaseValue::Json(value))
        },
        _ => {
            // Fallback: try to get as string
            let value: String = row.try_get(index)
                .map_err(|e| OrmError::Query(format!("Failed to get value as string for unknown type '{}': {}", type_name, e)))?;
            Ok(DatabaseValue::String(value))
        }
    }
}

impl Default for PostgresBackend {
    fn default() -> Self {
        Self::new()
    }
}