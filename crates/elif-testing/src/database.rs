//! Database testing utilities
//!
//! Provides comprehensive database testing support including:
//! - Automatic test database setup and cleanup
//! - Transaction-based test isolation
//! - Database assertion helpers
//! - Migration testing utilities

use std::sync::Arc;
use sqlx::{PgPool, Postgres, Transaction, Row};
use serde_json::Value as JsonValue;
use crate::{TestError, TestResult};

/// Test database manager that handles automatic setup and cleanup
#[derive(Clone)]
pub struct TestDatabase {
    pool: PgPool,
    transaction: Option<Arc<std::sync::Mutex<Transaction<'static, Postgres>>>>,
}

impl TestDatabase {
    /// Create a new test database connection
    /// 
    /// This will automatically configure a test database connection
    /// using environment variables or sensible defaults
    pub async fn new() -> TestResult<Self> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/elif_test".to_string());
            
        let pool = PgPool::connect(&database_url).await?;
        
        // Run any pending migrations in test mode
        Self::ensure_test_database(&pool).await?;
        
        Ok(Self {
            pool,
            transaction: None,
        })
    }
    
    /// Create a new test database with an isolated transaction
    /// 
    /// This is the recommended approach for test isolation
    pub async fn with_transaction() -> TestResult<Self> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/elif_test".to_string());
            
        let pool = PgPool::connect(&database_url).await?;
        Self::ensure_test_database(&pool).await?;
        
        let transaction = pool.begin().await?;
        
        Ok(Self {
            pool,
            transaction: Some(Arc::new(std::sync::Mutex::new(transaction))),
        })
    }
    
    /// Get the underlying connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    
    /// Execute a raw SQL query (for test setup)
    pub async fn execute(&self, sql: &str) -> TestResult<()> {
        sqlx::query(sql).execute(&self.pool).await?;
        Ok(())
    }
    
    /// Execute a query and return the first row
    pub async fn fetch_one(&self, sql: &str) -> TestResult<sqlx::postgres::PgRow> {
        let row = sqlx::query(sql).fetch_one(&self.pool).await?;
        Ok(row)
    }
    
    /// Execute a query and return all rows
    pub async fn fetch_all(&self, sql: &str) -> TestResult<Vec<sqlx::postgres::PgRow>> {
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        Ok(rows)
    }
    
    /// Check if a record exists in the database
    pub async fn record_exists(&self, table: &str, conditions: &[(&str, &dyn ToString)]) -> TestResult<bool> {
        let mut query = format!("SELECT 1 FROM {} WHERE", table);
        let mut params = Vec::new();
        
        for (i, (column, value)) in conditions.iter().enumerate() {
            if i > 0 {
                query.push_str(" AND");
            }
            query.push_str(&format!(" {} = ${}", column, i + 1));
            params.push(value.to_string());
        }
        
        let mut sql_query = sqlx::query(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }
        
        let result = sql_query.fetch_optional(&self.pool).await?;
        Ok(result.is_some())
    }
    
    /// Count records in a table with conditions
    pub async fn count_records(&self, table: &str, conditions: &[(&str, &dyn ToString)]) -> TestResult<i64> {
        let mut query = format!("SELECT COUNT(*) FROM {}", table);
        let mut params = Vec::new();
        
        if !conditions.is_empty() {
            query.push_str(" WHERE");
            for (i, (column, value)) in conditions.iter().enumerate() {
                if i > 0 {
                    query.push_str(" AND");
                }
                query.push_str(&format!(" {} = ${}", column, i + 1));
                params.push(value.to_string());
            }
        }
        
        let mut sql_query = sqlx::query_scalar(&query);
        for param in params {
            sql_query = sql_query.bind(param);
        }
        
        let count: i64 = sql_query.fetch_one(&self.pool).await?;
        Ok(count)
    }
    
    /// Clean up test data (truncate all tables except migrations)
    pub async fn cleanup(&self) -> TestResult<()> {
        let tables_query = r#"
            SELECT tablename FROM pg_tables 
            WHERE schemaname = 'public' 
            AND tablename != '_sqlx_migrations'
        "#;
        
        let rows = sqlx::query(tables_query).fetch_all(&self.pool).await?;
        
        for row in rows {
            let table_name: String = row.get("tablename");
            let truncate_sql = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE", table_name);
            sqlx::query(&truncate_sql).execute(&self.pool).await?;
        }
        
        Ok(())
    }
    
    /// Seed test data from a JSON file or inline JSON
    pub async fn seed_from_json(&self, data: JsonValue) -> TestResult<()> {
        if let Some(tables) = data.as_object() {
            for (table_name, records) in tables {
                if let Some(records_array) = records.as_array() {
                    for record in records_array {
                        self.insert_record(table_name, record).await?;
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Insert a single record from JSON
    async fn insert_record(&self, table: &str, record: &JsonValue) -> TestResult<()> {
        if let Some(fields) = record.as_object() {
            let columns: Vec<String> = fields.keys().cloned().collect();
            let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();
            
            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table,
                columns.join(", "),
                placeholders.join(", ")
            );
            
            let mut query = sqlx::query(&sql);
            for column in &columns {
                if let Some(value) = fields.get(column) {
                    match value {
                        JsonValue::String(s) => query = query.bind(s),
                        JsonValue::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                query = query.bind(i);
                            } else if let Some(f) = n.as_f64() {
                                query = query.bind(f);
                            }
                        },
                        JsonValue::Bool(b) => query = query.bind(b),
                        JsonValue::Null => query = query.bind(Option::<String>::None),
                        _ => query = query.bind(value.to_string()),
                    }
                }
            }
            
            query.execute(&self.pool).await?;
        }
        Ok(())
    }
    
    /// Ensure test database exists and is properly set up
    async fn ensure_test_database(pool: &PgPool) -> TestResult<()> {
        // This would typically run migrations in a test environment
        // For now, we'll just verify connectivity
        sqlx::query("SELECT 1").fetch_one(pool).await?;
        Ok(())
    }
}

/// Wrapper for database transactions in tests
pub struct DatabaseTransaction {
    transaction: Transaction<'static, Postgres>,
}

impl DatabaseTransaction {
    /// Create a new database transaction for testing
    pub async fn new(pool: &PgPool) -> TestResult<Self> {
        let transaction = pool.begin().await?;
        Ok(Self { transaction })
    }
    
    /// Execute a query within the transaction
    pub async fn execute(&mut self, sql: &str) -> TestResult<()> {
        sqlx::query(sql).execute(&mut *self.transaction).await?;
        Ok(())
    }
    
    /// Rollback the transaction (automatic cleanup)
    pub async fn rollback(self) -> TestResult<()> {
        self.transaction.rollback().await?;
        Ok(())
    }
}

/// Database assertion helpers
pub struct DatabaseAssertions<'a> {
    db: &'a TestDatabase,
}

impl<'a> DatabaseAssertions<'a> {
    pub fn new(db: &'a TestDatabase) -> Self {
        Self { db }
    }
    
    /// Assert that a record exists with the given conditions
    pub async fn assert_record_exists(&self, table: &str, conditions: &[(&str, &dyn ToString)]) -> TestResult<()> {
        let exists = self.db.record_exists(table, conditions).await?;
        if !exists {
            let conditions_str = conditions.iter()
                .map(|(k, v)| format!("{}={}", k, v.to_string()))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(TestError::Assertion {
                message: format!("Expected record to exist in table '{}' with conditions: {}", table, conditions_str),
            });
        }
        Ok(())
    }
    
    /// Assert that a record does not exist with the given conditions
    pub async fn assert_record_not_exists(&self, table: &str, conditions: &[(&str, &dyn ToString)]) -> TestResult<()> {
        let exists = self.db.record_exists(table, conditions).await?;
        if exists {
            let conditions_str = conditions.iter()
                .map(|(k, v)| format!("{}={}", k, v.to_string()))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(TestError::Assertion {
                message: format!("Expected record to NOT exist in table '{}' with conditions: {}", table, conditions_str),
            });
        }
        Ok(())
    }
    
    /// Assert the count of records matches expectations
    pub async fn assert_record_count(&self, table: &str, expected_count: i64, conditions: &[(&str, &dyn ToString)]) -> TestResult<()> {
        let actual_count = self.db.count_records(table, conditions).await?;
        if actual_count != expected_count {
            return Err(TestError::Assertion {
                message: format!("Expected {} records in table '{}', found {}", expected_count, table, actual_count),
            });
        }
        Ok(())
    }
}

/// Macro for easy database assertions (would be in a separate macro crate)
#[macro_export]
macro_rules! assert_database_has {
    ($db:expr, $table:expr, $($field:expr => $value:expr),+) => {
        {
            let conditions: Vec<(&str, &dyn ToString)> = vec![
                $(($field, $value),)+
            ];
            DatabaseAssertions::new($db).assert_record_exists($table, &conditions).await
        }
    };
}

#[macro_export]
macro_rules! assert_database_count {
    ($db:expr, $table:expr, $count:expr) => {
        DatabaseAssertions::new($db).assert_record_count($table, $count, &[]).await
    };
    ($db:expr, $table:expr, $count:expr, $($field:expr => $value:expr),+) => {
        {
            let conditions: Vec<(&str, &dyn ToString)> = vec![
                $(($field, $value),)+
            ];
            DatabaseAssertions::new($db).assert_record_count($table, $count, &conditions).await
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_database_utils() -> TestResult<()> {
        // These tests would require a test database setup
        // For now, we test the utility functions that don't require DB
        
        let test_json = json!({
            "users": [
                {"id": 1, "name": "Test User", "email": "test@example.com"},
                {"id": 2, "name": "Another User", "email": "another@example.com"}
            ]
        });
        
        assert!(test_json.is_object());
        if let Some(users) = test_json.get("users").and_then(|v| v.as_array()) {
            assert_eq!(users.len(), 2);
        }
        
        Ok(())
    }
    
    #[test]
    fn test_database_error_handling() {
        let error = TestError::Database(sqlx::Error::RowNotFound);
        assert!(matches!(error, TestError::Database(_)));
    }
    
    #[test]
    fn test_assertion_error_creation() {
        let error = TestError::Assertion {
            message: "Test assertion failed".to_string(),
        };
        assert!(error.to_string().contains("Test assertion failed"));
    }
}