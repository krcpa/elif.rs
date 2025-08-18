//! Migration Definitions - Core types and structures for migrations
//!
//! Defines the fundamental types used throughout the migration system including
//! Migration, MigrationRecord, and MigrationConfig structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a database migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// Unique identifier for the migration (typically timestamp)
    pub id: String,
    /// Human-readable name for the migration
    pub name: String,
    /// SQL statements to apply the migration
    pub up_sql: String,
    /// SQL statements to rollback the migration
    pub down_sql: String,
    /// When the migration was created
    pub created_at: DateTime<Utc>,
}

/// Migration status in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Migration ID
    pub id: String,
    /// When the migration was applied
    pub applied_at: DateTime<Utc>,
    /// Batch number (for grouping migrations)
    pub batch: i32,
}

/// Configuration for the migration system
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Directory where migration files are stored
    pub migrations_dir: PathBuf,
    /// Table name for tracking migrations
    pub migrations_table: String,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            migrations_dir: PathBuf::from("migrations"),
            migrations_table: "elif_migrations".to_string(),
        }
    }
}

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

/// Result of rolling back migrations
#[derive(Debug)]
pub struct RollbackResult {
    /// Number of migrations that were rolled back
    pub rolled_back_count: usize,
    /// IDs of migrations that were rolled back
    pub rolled_back_migrations: Vec<String>,
    /// Total execution time in milliseconds
    pub execution_time_ms: u128,
}

/// Migration direction for execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationDirection {
    /// Apply the migration (run UP statements)
    Up,
    /// Rollback the migration (run DOWN statements)
    Down,
}

/// Migration status in the system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationStatus {
    /// Migration is pending (not yet applied)
    Pending,
    /// Migration has been applied
    Applied {
        /// When it was applied
        applied_at: DateTime<Utc>,
        /// Batch number
        batch: i32,
    },
}