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
pub mod factories;
pub mod loading;
pub mod migrations;
pub mod models;
pub mod query;
pub mod relationships;
pub mod sql;
pub mod transactions;

// Event system and observers
pub mod event_error;
pub mod events;
pub mod observers;

// Legacy modules (maintained for backward compatibility)
pub mod database;
pub mod error;
pub mod factory;
pub mod migration;
pub mod migration_runner;
pub mod model;
pub mod security;
pub mod transaction;

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

// Prelude-style re-exports for core functionality
// Only export what actually exists in the modules

// Error handling
pub use error::{ModelError, ModelResult, OrmError, OrmResult};

// Database core
pub use database::{DatabasePool, DatabaseServiceProvider, PoolError, PoolHealthReport};

// Model system
pub use model::{CrudOperations, Model, PrimaryKey};

// Query system
pub use query::QueryBuilder;

// Transaction management
pub use transaction::{IsolationLevel, Transaction};

// Migration system
pub use migration::{
    Migration, MigrationManager, MigrationRunResult as MigrationResult, MigrationStatus,
    RollbackResult,
};

// Relationships (minimal exports to avoid conflicts)
pub use relationships::{
    relationship_traits::Relationship, RelationshipCache, RelationshipConstraint,
    RelationshipMetadata, RelationshipRegistry, RelationshipType,
};

// Database backends
pub use backends::{
    DatabaseBackendRegistry, DatabaseBackendType, DatabasePoolConfig, PostgresBackend,
};

// Event system and observers
pub use event_error::EventError;
pub use events::{ModelEvent, ModelObserver};
pub use model::lifecycle::ModelLifecycle;
pub use observers::{GlobalObserverRegistry, ObserverManager, ObserverRegistry};

// Derive macro re-exports (when implemented in future)
// pub use elif_orm_derive::*;
