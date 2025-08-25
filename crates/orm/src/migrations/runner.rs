//! Migration Runner - Executes migrations against the database
//!
//! Handles the actual execution of migrations, tracking applied migrations,
//! and managing migration batches.

use sqlx::{PgPool, Row};
use std::collections::HashSet;

use super::definitions::{Migration, MigrationRecord, MigrationRunResult};
use super::manager::MigrationManager;
use crate::error::{OrmError, OrmResult};

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
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| OrmError::Migration(format!("Failed to connect to database: {}", e)))?;

        Ok(Self::new(manager, pool))
    }

    /// Get the database pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get the migration manager
    pub fn manager(&self) -> &MigrationManager {
        &self.manager
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
            let mut transaction =
                self.pool.begin().await.map_err(|e| {
                    OrmError::Migration(format!("Failed to start transaction: {}", e))
                })?;

            // Execute migration SQL
            if !migration.up_sql.trim().is_empty() {
                for statement in self.manager.split_sql_statements(&migration.up_sql)? {
                    if !statement.trim().is_empty() {
                        sqlx::query(&statement)
                            .execute(&mut *transaction)
                            .await
                            .map_err(|e| {
                                OrmError::Migration(format!(
                                    "Failed to execute migration {}: {}",
                                    migration.id, e
                                ))
                            })?;
                    }
                }
            }

            // Record migration as applied
            let (record_sql, params) = self.record_migration_sql(&migration.id, next_batch);
            let mut query = sqlx::query(&record_sql);
            for param in params {
                query = query.bind(param);
            }
            query
                .execute(&mut *transaction)
                .await
                .map_err(|e| OrmError::Migration(format!("Failed to record migration: {}", e)))?;

            // Commit transaction
            transaction
                .commit()
                .await
                .map_err(|e| OrmError::Migration(format!("Failed to commit migration: {}", e)))?;

            applied_migration_ids.push(migration.id.clone());
        }

        Ok(MigrationRunResult {
            applied_count: applied_migration_ids.len(),
            applied_migrations: applied_migration_ids,
            skipped_count: applied_ids.len(),
            execution_time_ms: start_time.elapsed().as_millis(),
        })
    }

    /// Run a specific migration by ID
    pub async fn run_migration(&self, migration_id: &str) -> OrmResult<()> {
        // Load the specific migration
        let migrations = self.manager.load_migrations().await?;
        let migration = migrations
            .iter()
            .find(|m| m.id == migration_id)
            .ok_or_else(|| OrmError::Migration(format!("Migration {} not found", migration_id)))?;

        // Check if already applied
        if self.is_migration_applied(migration_id).await? {
            return Err(OrmError::Migration(format!(
                "Migration {} is already applied",
                migration_id
            )));
        }

        // Get next batch number
        let next_batch = self.get_next_batch_number().await?;

        // Apply the migration
        self.apply_migration(migration, next_batch).await?;

        Ok(())
    }

    /// Apply a single migration
    async fn apply_migration(&self, migration: &Migration, batch: i32) -> OrmResult<()> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|e| OrmError::Migration(format!("Failed to start transaction: {}", e)))?;

        // Execute migration SQL
        if !migration.up_sql.trim().is_empty() {
            for statement in self.manager.split_sql_statements(&migration.up_sql)? {
                if !statement.trim().is_empty() {
                    sqlx::query(&statement)
                        .execute(&mut *transaction)
                        .await
                        .map_err(|e| {
                            OrmError::Migration(format!(
                                "Failed to execute migration {}: {}",
                                migration.id, e
                            ))
                        })?;
                }
            }
        }

        // Record migration as applied
        let (record_sql, params) = self.record_migration_sql(&migration.id, batch);
        let mut query = sqlx::query(&record_sql);
        for param in params {
            query = query.bind(param);
        }
        query
            .execute(&mut *transaction)
            .await
            .map_err(|e| OrmError::Migration(format!("Failed to record migration: {}", e)))?;

        // Commit transaction
        transaction
            .commit()
            .await
            .map_err(|e| OrmError::Migration(format!("Failed to commit migration: {}", e)))?;

        Ok(())
    }

    /// Ensure migrations table exists
    async fn ensure_migrations_table(&self) -> OrmResult<()> {
        let sql = self.create_migrations_table_sql();
        sqlx::query(&sql).execute(&self.pool).await.map_err(|e| {
            OrmError::Migration(format!("Failed to create migrations table: {}", e))
        })?;
        Ok(())
    }

    /// Get applied migrations from database
    async fn get_applied_migrations(&self) -> OrmResult<Vec<MigrationRecord>> {
        let sql = self.get_applied_migrations_sql();
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await.map_err(|e| {
            OrmError::Migration(format!("Failed to query applied migrations: {}", e))
        })?;

        let mut records = Vec::new();
        for row in rows {
            let id: String = row
                .try_get("id")
                .map_err(|e| OrmError::Migration(format!("Failed to get migration id: {}", e)))?;
            let applied_at: chrono::DateTime<chrono::Utc> = row
                .try_get("applied_at")
                .map_err(|e| OrmError::Migration(format!("Failed to get applied_at: {}", e)))?;
            let batch: i32 = row
                .try_get("batch")
                .map_err(|e| OrmError::Migration(format!("Failed to get batch: {}", e)))?;

            records.push(MigrationRecord {
                id,
                applied_at,
                batch,
            });
        }

        Ok(records)
    }

    /// Check if a specific migration has been applied
    async fn is_migration_applied(&self, migration_id: &str) -> OrmResult<bool> {
        let (sql, params) = self.check_migration_sql(migration_id);
        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }

        let result = query
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| OrmError::Migration(format!("Failed to check migration status: {}", e)))?;

        Ok(result.is_some())
    }

    /// Get the next batch number
    async fn get_next_batch_number(&self) -> OrmResult<i32> {
        let sql = self.get_latest_batch_sql();
        let row = sqlx::query(&sql)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| OrmError::Migration(format!("Failed to get latest batch: {}", e)))?;

        let latest_batch: i32 = row.try_get(0).unwrap_or(0);
        Ok(latest_batch + 1)
    }

    /// SQL to create the migrations tracking table
    fn create_migrations_table_sql(&self) -> String {
        format!(
            "CREATE TABLE IF NOT EXISTS {} (\n    \
                id VARCHAR(255) PRIMARY KEY,\n    \
                applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,\n    \
                batch INTEGER NOT NULL\n\
            );",
            self.manager.config().migrations_table
        )
    }

    /// SQL to check if a migration has been applied
    fn check_migration_sql(&self, migration_id: &str) -> (String, Vec<String>) {
        (
            format!(
                "SELECT id FROM {} WHERE id = $1",
                self.manager.config().migrations_table
            ),
            vec![migration_id.to_string()],
        )
    }

    /// SQL to record a migration as applied
    fn record_migration_sql(&self, migration_id: &str, batch: i32) -> (String, Vec<String>) {
        (
            format!(
                "INSERT INTO {} (id, applied_at, batch) VALUES ($1, $2, $3)",
                self.manager.config().migrations_table
            ),
            vec![
                migration_id.to_string(),
                chrono::Utc::now().to_rfc3339(),
                batch.to_string(),
            ],
        )
    }

    /// SQL to get the latest batch number
    fn get_latest_batch_sql(&self) -> String {
        format!(
            "SELECT COALESCE(MAX(batch), 0) FROM {}",
            self.manager.config().migrations_table
        )
    }

    /// SQL to get applied migrations
    fn get_applied_migrations_sql(&self) -> String {
        format!(
            "SELECT id, applied_at, batch FROM {} ORDER BY batch DESC, applied_at DESC",
            self.manager.config().migrations_table
        )
    }

    /// Get migration status for all migrations (applied and pending)
    pub async fn get_migration_status(&self) -> OrmResult<Vec<(Migration, bool)>> {
        // Load all migrations from files
        let all_migrations = self.manager.load_migrations().await?;

        // Get applied migrations from database
        let applied_migrations = self.get_applied_migrations().await?;
        let applied_ids: HashSet<String> = applied_migrations.into_iter().map(|m| m.id).collect();

        // Map migrations to their status
        let mut status_list = Vec::new();
        for migration in all_migrations {
            let is_applied = applied_ids.contains(&migration.id);
            status_list.push((migration, is_applied));
        }

        Ok(status_list)
    }
}
