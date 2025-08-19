//! # elif-orm: Database Layer for elif.rs
//!
//! Production-ready ORM with modular architecture featuring:
//! - Core database abstraction layer
//! - Query building and execution
//! - Model system with relationships
//! - Migration and schema management
//! - Factory system for testing
//! - Connection pool management
//!
//! ## New Modular Architecture
//!
//! The ORM is organized into 6 main domains:
//! - `backends/` - Database backend abstractions (PostgreSQL, etc.)
//! - `connection/` - Connection pool management and health monitoring
//! - `transactions/` - Transaction lifecycle and isolation management
//! - `query/` - Query building and execution
//! - `sql/` - SQL generation and security
//! - `models/` - Model traits and CRUD operations
//! - `relationships/` - Relationship system and loading
//! - `loading/` - Eager/lazy loading strategies
//! - `migrations/` - Schema migrations
//! - `factories/` - Model factories and test data

// New modular architecture
pub mod backends;
pub mod connection;
pub mod transactions;
pub mod query;
pub mod sql;
pub mod models;
pub mod relationships;
pub mod loading;
pub mod migrations;
pub mod factories;

// Legacy modules (maintained for backward compatibility)
pub mod model;
pub mod error;
pub mod database;
pub mod transaction;
pub mod migration;
pub mod migration_runner;
pub mod security;
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
pub use relationships::*;
pub use security::*;
pub use backends::*;
pub use factory::*;

// Derive macro re-exports (when implemented in future)
// pub use elif_orm_derive::*;