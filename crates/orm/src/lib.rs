//! # elif-orm: Database Layer for elif.rs
//!
//! Phase 2 implementation: Production-ready ORM with relationships,
//! query builder, migrations, and connection management.
//!
//! This crate provides the core database layer for elif.rs web framework,
//! including Model trait, QueryBuilder, error handling, and future support
//! for relationships, migrations, and connection management.

pub mod model;
pub mod query;
pub mod error;

// Re-export core traits and types
pub use model::*;
pub use query::*;
pub use error::*;

// Derive macro re-exports (when implemented in future)
// pub use elif_orm_derive::*;