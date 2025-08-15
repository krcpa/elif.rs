//! Query Builder Module - Type-safe, fluent query builder for complex database operations

pub mod types;
pub mod builder;
pub mod select;
pub mod dml;
pub mod where_clause;
pub mod joins;
pub mod ordering;
pub mod pagination;
pub mod upsert;
pub mod sql_generation;
pub mod execution;
pub mod performance;
pub mod with;

// Re-export main types and builder
pub use types::*;
pub use builder::QueryBuilder;
pub use upsert::UpsertBuilder;
pub use with::{QueryBuilderWithMethods, QueryBuilderWithEagerLoading};