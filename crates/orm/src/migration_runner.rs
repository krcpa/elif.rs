//! Migration runner - backward compatibility module
//!
//! This module provides backward compatibility by re-exporting the modular migration runner.
//! The migration system has been reorganized into focused modules under migrations/

// Re-export MigrationRunner and related types from the modular system
pub use crate::migrations::runner::*;
pub use crate::migrations::definitions::MigrationRunResult;

// Re-export MigrationRollback trait for rollback functionality
pub use crate::migrations::rollback::MigrationRollback;