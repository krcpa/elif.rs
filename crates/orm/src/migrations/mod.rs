//! Migration System
//!
//! This module provides database migration management.
//! The legacy migration files are re-exported for backward compatibility.

// Re-export legacy migration system for backward compatibility
pub use crate::migration::*;
pub use crate::migration_runner::*;

// TODO: Implement modular migration system
// pub mod runner;
// pub mod definitions;
// pub mod rollback;
// pub mod validation;