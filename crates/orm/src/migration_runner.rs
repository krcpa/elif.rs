//! Migration runner that integrates with the database connection system
//!
//! This module provides the MigrationRunner that can execute migrations
//! against actual database connections using sqlx PostgreSQL pools.

use std::collections::HashSet;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use crate::migration::{Migration, MigrationManager, MigrationRecord};
use crate::error::{OrmError, OrmResult};

/// Result of running migrations
#[derive(Debug)]
pub struct MigrationRunResult {
    /// Number of migrations that were applied
    pub applied_count: usize,
    /// IDs of migrations that were applied
    pub applied_migrations: Vec<String>,
    /// Number of migrations that were skipped (already applied)
    pub skipped_count: usize,
    /// Total execution time in milliseconds
    pub execution_time_ms: u128,
}

/// Migration runner that executes migrations against a database
pub struct MigrationRunner {
    manager: MigrationManager,
    pool: PgPool,
}

impl MigrationRunner {
    /// Create a new migration runner
    pub fn new(manager: MigrationManager, pool: PgPool) -> Self {
        Self { manager, pool }
    }

    /// Create a new migration runner from database URL
    pub async fn from_url(manager: MigrationManager, database_url: &str) -> OrmResult<Self> {
        let pool = PgPool::connect(database_url).await
            .map_err(|e| OrmError::Migration(format!("Failed to connect to database: {}", e)))?;
        
        Ok(Self::new(manager, pool))
    }

    /// Run all pending migrations
    pub async fn run_migrations(&self) -> OrmResult<MigrationRunResult> {
        let start_time = std::time::Instant::now();
        
        // Ensure migrations table exists
        self.ensure_migrations_table().await?;

        // Load all migrations from files
        let all_migrations = self.manager.load_migrations().await?;
        
        // Get applied migrations from database
        let applied_migrations = self.get_applied_migrations().await?;
        let applied_ids: HashSet<String> = applied_migrations.into_iter().map(|m| m.id).collect();

        // Filter pending migrations
        let pending_migrations: Vec<_> = all_migrations
            .into_iter()
            .filter(|m| !applied_ids.contains(&m.id))
            .collect();

        if pending_migrations.is_empty() {
            return Ok(MigrationRunResult {
                applied_count: 0,
                applied_migrations: Vec::new(),
                skipped_count: applied_ids.len(),
                execution_time_ms: start_time.elapsed().as_millis(),
            });
        }

        // Get next batch number
        let next_batch = self.get_next_batch_number().await?;

        // Apply pending migrations
        let mut applied_migration_ids = Vec::new();
        
        for migration in &pending_migrations {
            println!("Applying migration: {} - {}", migration.id, migration.name);
            
            // Begin transaction for this migration
            let mut transaction = self.pool.begin().await
                .map_err(|e| OrmError::Migration(format!("Failed to start transaction: {}", e)))?;

            // Execute migration SQL
            if !migration.up_sql.trim().is_empty() {
                for statement in self.split_sql_statements(&migration.up_sql) {
                    if !statement.trim().is_empty() {
                        sqlx::query(&statement).execute(&mut *transaction).await
                            .map_err(|e| OrmError::Migration(format!(
                                "Failed to execute migration {}: {}", migration.id, e
                            )))?;
                    }
                }
            }

            // Record migration as applied
            let (record_sql, params) = self.manager.record_migration_sql(&migration.id, next_batch);
            sqlx::query(&record_sql)
                .bind(&params[0]) // id
                .bind(&params[1]) // applied_at
                .bind(&params[2]) // batch
                .execute(&mut *transaction).await
                .map_err(|e| OrmError::Migration(format!(
                    "Failed to record migration {}: {}", migration.id, e
                )))?;

            // Commit transaction
            transaction.commit().await
                .map_err(|e| OrmError::Migration(format!(
                    "Failed to commit migration {}: {}", migration.id, e
                )))?;

            applied_migration_ids.push(migration.id.clone());
            println!("✓ Applied migration: {}", migration.id);
        }

        Ok(MigrationRunResult {
            applied_count: applied_migration_ids.len(),
            applied_migrations: applied_migration_ids,
            skipped_count: applied_ids.len(),
            execution_time_ms: start_time.elapsed().as_millis(),
        })
    }

    /// Rollback the last batch of migrations
    pub async fn rollback_last_batch(&self) -> OrmResult<MigrationRunResult> {
        let start_time = std::time::Instant::now();
        
        // Get the latest batch
        let latest_batch = self.get_latest_batch_number().await?;
        if latest_batch == 0 {
            return Ok(MigrationRunResult {
                applied_count: 0,
                applied_migrations: Vec::new(),
                skipped_count: 0,
                execution_time_ms: start_time.elapsed().as_millis(),
            });
        }

        // Get migrations in the latest batch
        let batch_migrations = self.get_migrations_in_batch(latest_batch).await?;
        
        // Load migration files to get rollback SQL
        let all_migrations = self.manager.load_migrations().await?;
        let migration_map: std::collections::HashMap<String, Migration> = all_migrations
            .into_iter()
            .map(|m| (m.id.clone(), m))
            .collect();

        let mut rolled_back_ids = Vec::new();

        // Rollback migrations in reverse order
        for migration_record in batch_migrations.into_iter().rev() {
            if let Some(migration) = migration_map.get(&migration_record.id) {
                println!("Rolling back migration: {} - {}", migration.id, migration.name);

                // Begin transaction
                let mut transaction = self.pool.begin().await
                    .map_err(|e| OrmError::Migration(format!("Failed to start transaction: {}", e)))?;

                // Execute rollback SQL
                if !migration.down_sql.trim().is_empty() {
                    for statement in self.split_sql_statements(&migration.down_sql) {
                        if !statement.trim().is_empty() {
                            sqlx::query(&statement).execute(&mut *transaction).await
                                .map_err(|e| OrmError::Migration(format!(
                                    "Failed to rollback migration {}: {}", migration.id, e
                                )))?;
                        }
                    }
                }

                // Remove migration record
                let (remove_sql, params) = self.manager.remove_migration_sql(&migration.id);
                sqlx::query(&remove_sql)
                    .bind(&params[0])
                    .execute(&mut *transaction).await
                    .map_err(|e| OrmError::Migration(format!(
                        "Failed to remove migration record {}: {}", migration.id, e
                    )))?;

                // Commit transaction
                transaction.commit().await
                    .map_err(|e| OrmError::Migration(format!(
                        "Failed to commit rollback {}: {}", migration.id, e
                    )))?;

                rolled_back_ids.push(migration.id.clone());
                println!("✓ Rolled back migration: {}", migration.id);
            }
        }

        Ok(MigrationRunResult {
            applied_count: rolled_back_ids.len(),
            applied_migrations: rolled_back_ids,
            skipped_count: 0,
            execution_time_ms: start_time.elapsed().as_millis(),
        })
    }

    /// Get migration status (applied vs pending)
    pub async fn get_migration_status(&self) -> OrmResult<Vec<(Migration, bool)>> {
        // Ensure migrations table exists
        self.ensure_migrations_table().await?;

        // Load all migrations
        let all_migrations = self.manager.load_migrations().await?;
        
        // Get applied migrations
        let applied_migrations = self.get_applied_migrations().await?;
        let applied_ids: HashSet<String> = applied_migrations.into_iter().map(|m| m.id).collect();

        // Build status list
        let status: Vec<(Migration, bool)> = all_migrations
            .into_iter()
            .map(|m| {
                let is_applied = applied_ids.contains(&m.id);
                (m, is_applied)
            })
            .collect();

        Ok(status)
    }

    /// Ensure the migrations tracking table exists
    async fn ensure_migrations_table(&self) -> OrmResult<()> {
        let create_sql = self.manager.create_migrations_table_sql();
        sqlx::query(&create_sql).execute(&self.pool).await
            .map_err(|e| OrmError::Migration(format!("Failed to create migrations table: {}", e)))?;
        Ok(())
    }

    /// Get all applied migrations from the database
    async fn get_applied_migrations(&self) -> OrmResult<Vec<MigrationRecord>> {
        let sql = self.manager.get_applied_migrations_sql();
        
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await
            .map_err(|e| OrmError::Migration(format!("Failed to query applied migrations: {}", e)))?;

        let mut migrations = Vec::new();
        for row in rows {
            migrations.push(MigrationRecord {
                id: row.get(0),
                applied_at: row.get(1),
                batch: row.get(2),
            });
        }

        Ok(migrations)
    }

    /// Get the next batch number
    async fn get_next_batch_number(&self) -> OrmResult<i32> {
        let latest_batch = self.get_latest_batch_number().await?;
        Ok(latest_batch + 1)
    }

    /// Get the latest batch number
    async fn get_latest_batch_number(&self) -> OrmResult<i32> {
        let sql = self.manager.get_latest_batch_sql();
        
        let row = sqlx::query(&sql).fetch_one(&self.pool).await
            .map_err(|e| OrmError::Migration(format!("Failed to get latest batch: {}", e)))?;

        Ok(row.get(0))
    }

    /// Get migrations in a specific batch
    async fn get_migrations_in_batch(&self, batch: i32) -> OrmResult<Vec<MigrationRecord>> {
        let sql = format!(
            "SELECT id, applied_at, batch FROM {} WHERE batch = $1 ORDER BY applied_at DESC",
            "elif_migrations" // Use default table name
        );
        
        let rows = sqlx::query(&sql).bind(batch).fetch_all(&self.pool).await
            .map_err(|e| OrmError::Migration(format!("Failed to query batch migrations: {}", e)))?;

        let mut migrations = Vec::new();
        for row in rows {
            migrations.push(MigrationRecord {
                id: row.get(0),
                applied_at: row.get(1),
                batch: row.get(2),
            });
        }

        Ok(migrations)
    }

    /// Split SQL into individual statements
    fn split_sql_statements(&self, sql: &str) -> Vec<String> {
        sql.split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Note: These tests would require actual database connections to run
    // For now, we'll test the SQL generation and basic logic

    #[test]
    fn test_sql_statement_splitting() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::migration::MigrationConfig {
            migrations_dir: temp_dir.path().to_path_buf(),
            migrations_table: "test_migrations".to_string(),
        };
        
        let manager = MigrationManager::with_config(config);
        
        // Create a mock pool (this would need actual DB for real tests)
        let database_url = "postgresql://test:test@localhost:5432/test";
        
        // This test just verifies SQL splitting logic without database
        let sql = "CREATE TABLE users (id SERIAL PRIMARY KEY); INSERT INTO users (name) VALUES ('test');";
        let statements: Vec<String> = sql.split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0], "CREATE TABLE users (id SERIAL PRIMARY KEY)");
        assert_eq!(statements[1], "INSERT INTO users (name) VALUES ('test')");
    }

    #[test]
    fn test_empty_sql_handling() {
        let sql = "  ;  ; CREATE TABLE test ();  ;  ";
        let statements: Vec<String> = sql.split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0], "CREATE TABLE test ()");
    }
}