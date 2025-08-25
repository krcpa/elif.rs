//! Query Builder Module - Type-safe, fluent query builder for complex database operations

pub mod builder;
pub mod dml;
pub mod execution;
pub mod joins;
pub mod ordering;
pub mod pagination;
pub mod performance;
pub mod performance_optimized;
pub mod select;
pub mod types;
pub mod upsert;
pub mod where_clause;
pub mod with;

// Re-export main types and builder (minimal exports to avoid conflicts)
pub use builder::QueryBuilder;
pub use performance_optimized::{acquire_query_builder, release_query_builder, QueryBuilderPool};
pub use types::{OrderDirection, QueryOperator};
pub use upsert::UpsertBuilder;
pub use with::{QueryBuilderWithEagerLoading, QueryBuilderWithMethods};

// Import SQL generation methods to make them available when using QueryBuilder
pub use crate::sql::generation;
