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

// Event system and observers
pub mod events;
pub mod event_error;
pub mod observers;

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


// Prelude-style re-exports for core functionality
// Only export what actually exists in the modules

// Error handling
pub use error::{ModelError, ModelResult, OrmError, OrmResult};

// Database core
pub use database::{DatabaseServiceProvider, DatabasePool, PoolError, PoolHealthReport};

// Model system  
pub use model::{Model, PrimaryKey, CrudOperations};

// Query system
pub use query::{QueryBuilder};

// Transaction management
pub use transaction::{Transaction, IsolationLevel};

// Migration system  
pub use migration::{Migration, MigrationManager, MigrationRunResult as MigrationResult, MigrationStatus, RollbackResult};

// Relationships (minimal exports to avoid conflicts)
pub use relationships::{
    relationship_traits::Relationship,
    RelationshipMetadata, RelationshipType, RelationshipConstraint,
    RelationshipCache, RelationshipRegistry,
};

// Database backends
pub use backends::{DatabaseBackendType, DatabasePoolConfig, PostgresBackend, DatabaseBackendRegistry};

// Event system and observers
pub use events::{ModelEvent, ModelObserver};
pub use event_error::EventError;
pub use observers::{ObserverRegistry, GlobalObserverRegistry, ObserverManager};
pub use model::lifecycle::ModelLifecycle;

// Derive macro re-exports (when implemented in future)
// pub use elif_orm_derive::*;