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
pub mod execution;
pub mod performance;
pub mod performance_optimized;
pub mod with;

// Re-export main types and builder (minimal exports to avoid conflicts)
pub use types::{OrderDirection, QueryOperator};
pub use builder::QueryBuilder;
pub use upsert::UpsertBuilder;
pub use with::{QueryBuilderWithMethods, QueryBuilderWithEagerLoading};
pub use performance_optimized::{acquire_query_builder, release_query_builder, QueryBuilderPool};

// Import SQL generation methods to make them available when using QueryBuilder
pub use crate::sql::generation;