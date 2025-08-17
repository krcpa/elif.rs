//! # elif-orm: Database Layer for elif.rs
//!
//! Phase 2 implementation: Production-ready ORM with relationships,
//! query builder, migrations, and connection management.
//!
//! This crate provides the core database layer for elif.rs web framework,
//! including Model trait, QueryBuilder, error handling, and future support
//! for relationships, migrations, and connection management.

pub mod model;
pub mod query; // Directory-based module
pub mod error;
pub mod database;
pub mod transaction;
pub mod migration;
pub mod migration_runner;
pub mod relationships;
pub mod security;
pub mod loading;
pub mod backends;
pub mod factory;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod transaction_tests;


#[cfg(test)]  
mod security_tests_minimal;

#[cfg(test)]
mod security_tests_comprehensive;

#[cfg(test)]
mod security_injection_tests;

// Re-export core traits and types
pub use model::*;
pub use query::*;
pub use error::*;
pub use database::*;
pub use transaction::*;
pub use migration::*;
pub use migration_runner::*;
pub use relationships::*;
pub use security::*;
pub use backends::*;
pub use factory::*;

// Derive macro re-exports (when implemented in future)
// pub use elif_orm_derive::*;