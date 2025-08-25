//! Modular migration system
//!
//! This module organizes the migration system into focused components:
//! - `definitions` - Core types and structures
//! - `manager` - File system operations and migration creation
//! - `runner` - Migration execution against databases
//! - `rollback` - Rolling back applied migrations

pub mod definitions;
pub mod manager;
pub mod rollback;
pub mod runner;
pub mod schema_builder;

// Re-export commonly used types
pub use definitions::{
    Migration, MigrationConfig, MigrationDirection, MigrationRecord, MigrationRunResult,
    MigrationStatus, RollbackResult,
};
pub use manager::MigrationManager;
pub use rollback::MigrationRollback;
pub use runner::MigrationRunner;
pub use schema_builder::{SchemaBuilder, TableBuilder};
