//! Relationship constraint system for type-safe eager loading
//!
//! This module provides a comprehensive constraint system for applying
//! SQL constraints to eager loaded relationships, including WHERE clauses,
//! ORDER BY, LIMIT/OFFSET, GROUP BY, and HAVING constraints.

pub mod builder;
pub mod implementations;
pub mod types;

// Re-export public API
pub use builder::RelationshipConstraintBuilder;
pub use types::{ConstraintType, RelationshipConstraint};

// Internal implementations are not re-exported to keep the API clean
// They are used internally by the builder
