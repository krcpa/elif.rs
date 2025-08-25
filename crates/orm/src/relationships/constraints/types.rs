//! Constraint types and trait definitions for relationship queries

use crate::error::ModelResult;
use crate::query::QueryBuilder;
use async_trait::async_trait;

/// Constraint types for relationship queries
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    Where,
    Order,
    Limit,
    Offset,
    Having,
    GroupBy,
    Join,
    Raw,
}

/// Trait for applying constraints to relationship queries
#[async_trait]
pub trait RelationshipConstraint: Send + Sync + std::fmt::Debug {
    /// Apply constraint to the query builder
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()>;

    /// Get the type of constraint
    fn constraint_type(&self) -> ConstraintType;

    /// Get a description of the constraint for debugging
    fn description(&self) -> String;

    /// Validate the constraint before applying
    fn validate(&self) -> ModelResult<()> {
        Ok(()) // Default implementation - constraints can override
    }
}
