//! Migration Rollback - Handles rolling back applied migrations
//!
//! Provides functionality to rollback migrations by batch or individually,
//! executing DOWN statements to reverse schema changes.

use sqlx::Row;

use crate::error::{OrmError, OrmResult};
use super::definitions::{Migration, MigrationRecord, RollbackResult};
use super::runner::MigrationRunner;

/// Extension trait for MigrationRunner to add rollback functionality
pub trait MigrationRollback {
    /// Rollback the last batch of migrations
    async fn rollback_last_batch(&self) -> OrmResult<RollbackResult>;
    
    /// Rollback all migrations in a specific batch
    async fn rollback_batch(&self, batch: i32) -> OrmResult<RollbackResult>;
    
    /// Rollback a specific migration by ID
    async fn rollback_migration(&self, migration_id: &str) -> OrmResult<()>;
    
    /// Rollback all applied migrations
    async fn rollback_all(&self) -> OrmResult<RollbackResult>;
    
    /// Get migrations in a specific batch
    async fn get_migrations_in_batch(&self, batch: i32) -> OrmResult<Vec<MigrationRecord>>;
}

impl MigrationRollback for MigrationRunner {
    async fn rollback_last_batch(&self) -> OrmResult<RollbackResult> {
        let start_time = std::time::Instant::now();
        
        // Get the latest batch number
        let latest_batch = self.get_latest_batch_number().await?;
        
        if latest_batch == 0 {
            return Ok(RollbackResult {
                rolled_back_count: 0,
                rolled_back_migrations: Vec::new(),
                execution_time_ms: start_time.elapsed().as_millis(),
            });
        }
        
        self.rollback_batch(latest_batch).await
    }
    
    async fn rollback_batch(&self, batch: i32) -> OrmResult<RollbackResult> {
        let start_time = std::time::Instant::now();
        
        // Get all migrations in this batch
        let batch_migrations = self.get_migrations_in_batch(batch).await?;
        
        if batch_migrations.is_empty() {
            return Ok(RollbackResult {
                rolled_back_count: 0,
                rolled_back_migrations: Vec::new(),
                execution_time_ms: start_time.elapsed().as_millis(),
            });
        }
        
        // Load all migration files to get DOWN SQL
        let all_migrations = self.manager().load_migrations().await?;
        let migration_map: std::collections::HashMap<String, Migration> = all_migrations
            .into_iter()
            .map(|m| (m.id.clone(), m))
            .collect();
        
        let mut rolled_back_migrations = Vec::new();
        
        // Rollback migrations in reverse order
        for record in batch_migrations.iter().rev() {
            if let Some(migration) = migration_map.get(&record.id) {
                println!("Rolling back migration: {} - {}", migration.id, migration.name);
                
                // Begin transaction
                let mut transaction = self.pool().begin().await
                    .map_err(|e| OrmError::Migration(format!("Failed to start rollback transaction: {}", e)))?;
                
                // Execute DOWN SQL
                if !migration.down_sql.trim().is_empty() {
                    for statement in self.manager().split_sql_statements(&migration.down_sql) {
                        if !statement.trim().is_empty() {
                            sqlx::query(&statement).execute(&mut *transaction).await
                                .map_err(|e| OrmError::Migration(format!(
                                    "Failed to rollback migration {}: {}", migration.id, e
                                )))?;
                        }
                    }
                }
                
                // Remove migration record
                let (remove_sql, params) = self.remove_migration_sql(&migration.id);
                let mut query = sqlx::query(&remove_sql);
                for param in params {
                    query = query.bind(param);
                }
                query.execute(&mut *transaction).await
                    .map_err(|e| OrmError::Migration(format!("Failed to remove migration record: {}", e)))?;
                
                // Commit transaction
                transaction.commit().await
                    .map_err(|e| OrmError::Migration(format!("Failed to commit rollback: {}", e)))?;
                
                rolled_back_migrations.push(record.id.clone());
            } else {
                return Err(OrmError::Migration(format!(
                    "Migration file not found for applied migration: {}", record.id
                )));
            }
        }
        
        Ok(RollbackResult {
            rolled_back_count: rolled_back_migrations.len(),
            rolled_back_migrations,
            execution_time_ms: start_time.elapsed().as_millis(),
        })
    }
    
    async fn rollback_migration(&self, migration_id: &str) -> OrmResult<()> {
        // Check if migration is applied
        let applied_migrations = self.get_applied_migrations_ordered().await?;
        let migration_record = applied_migrations.iter()
            .find(|m| m.id == migration_id)
            .ok_or_else(|| OrmError::Migration(format!("Migration {} is not applied", migration_id)))?;
        
        // Check if this is the most recent migration
        if let Some(most_recent) = applied_migrations.first() {
            if most_recent.id != migration_id {
                return Err(OrmError::Migration(
                    "Can only rollback the most recent migration. Use rollback_batch for batch operations.".to_string()
                ));
            }
        }
        
        // Load the migration file
        let migrations = self.manager().load_migrations().await?;
        let migration = migrations.iter()
            .find(|m| m.id == migration_id)
            .ok_or_else(|| OrmError::Migration(format!("Migration file {} not found", migration_id)))?;
        
        // Begin transaction
        let mut transaction = self.pool().begin().await
            .map_err(|e| OrmError::Migration(format!("Failed to start rollback transaction: {}", e)))?;
        
        // Execute DOWN SQL
        if !migration.down_sql.trim().is_empty() {
            for statement in self.manager().split_sql_statements(&migration.down_sql) {
                if !statement.trim().is_empty() {
                    sqlx::query(&statement).execute(&mut *transaction).await
                        .map_err(|e| OrmError::Migration(format!(
                            "Failed to rollback migration {}: {}", migration.id, e
                        )))?;
                }
            }
        }
        
        // Remove migration record
        let (remove_sql, params) = self.remove_migration_sql(&migration.id);
        let mut query = sqlx::query(&remove_sql);
        for param in params {
            query = query.bind(param);
        }
        query.execute(&mut *transaction).await
            .map_err(|e| OrmError::Migration(format!("Failed to remove migration record: {}", e)))?;
        
        // Commit transaction
        transaction.commit().await
            .map_err(|e| OrmError::Migration(format!("Failed to commit rollback: {}", e)))?;
        
        println!("Rolled back migration: {} - {}", migration.id, migration.name);
        
        Ok(())
    }
    
    async fn rollback_all(&self) -> OrmResult<RollbackResult> {
        let start_time = std::time::Instant::now();
        let mut total_rolled_back = Vec::new();
        
        loop {
            let result = self.rollback_last_batch().await?;
            if result.rolled_back_count == 0 {
                break;
            }
            total_rolled_back.extend(result.rolled_back_migrations);
        }
        
        Ok(RollbackResult {
            rolled_back_count: total_rolled_back.len(),
            rolled_back_migrations: total_rolled_back,
            execution_time_ms: start_time.elapsed().as_millis(),
        })
    }
    
    async fn get_migrations_in_batch(&self, batch: i32) -> OrmResult<Vec<MigrationRecord>> {
        let sql = format!(
            "SELECT id, applied_at, batch FROM {} WHERE batch = $1 ORDER BY applied_at DESC",
            self.manager().config().migrations_table
        );
        
        let rows = sqlx::query(&sql)
            .bind(batch)
            .fetch_all(self.pool())
            .await
            .map_err(|e| OrmError::Migration(format!("Failed to query batch migrations: {}", e)))?;
        
        let mut records = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")
                .map_err(|e| OrmError::Migration(format!("Failed to get migration id: {}", e)))?;
            let applied_at: chrono::DateTime<chrono::Utc> = row.try_get("applied_at")
                .map_err(|e| OrmError::Migration(format!("Failed to get applied_at: {}", e)))?;
            let batch: i32 = row.try_get("batch")
                .map_err(|e| OrmError::Migration(format!("Failed to get batch: {}", e)))?;
            
            records.push(MigrationRecord {
                id,
                applied_at,
                batch,
            });
        }
        
        Ok(records)
    }
}

// Extension methods for MigrationRunner
impl MigrationRunner {
    /// Get applied migrations ordered by batch and time (most recent first)
    async fn get_applied_migrations_ordered(&self) -> OrmResult<Vec<MigrationRecord>> {
        let sql = format!(
            "SELECT id, applied_at, batch FROM {} ORDER BY batch DESC, applied_at DESC",
            self.manager().config().migrations_table
        );
        
        let rows = sqlx::query(&sql).fetch_all(self.pool()).await
            .map_err(|e| OrmError::Migration(format!("Failed to query applied migrations: {}", e)))?;
        
        let mut records = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")
                .map_err(|e| OrmError::Migration(format!("Failed to get migration id: {}", e)))?;
            let applied_at: chrono::DateTime<chrono::Utc> = row.try_get("applied_at")
                .map_err(|e| OrmError::Migration(format!("Failed to get applied_at: {}", e)))?;
            let batch: i32 = row.try_get("batch")
                .map_err(|e| OrmError::Migration(format!("Failed to get batch: {}", e)))?;
            
            records.push(MigrationRecord {
                id,
                applied_at,
                batch,
            });
        }
        
        Ok(records)
    }
    
    /// Get the latest batch number
    async fn get_latest_batch_number(&self) -> OrmResult<i32> {
        let sql = format!(
            "SELECT COALESCE(MAX(batch), 0) FROM {}",
            self.manager().config().migrations_table
        );
        
        let row = sqlx::query(&sql).fetch_one(self.pool()).await
            .map_err(|e| OrmError::Migration(format!("Failed to get latest batch: {}", e)))?;
        
        let latest_batch: i32 = row.try_get(0).unwrap_or(0);
        Ok(latest_batch)
    }
    
    /// SQL to remove a migration record
    fn remove_migration_sql(&self, migration_id: &str) -> (String, Vec<String>) {
        (
            format!("DELETE FROM {} WHERE id = $1", self.manager().config().migrations_table),
            vec![migration_id.to_string()]
        )
    }
}