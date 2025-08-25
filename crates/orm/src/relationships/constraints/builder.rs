//! Constraint builder for creating type-safe relationship constraints

use super::implementations::*;
use super::types::{ConstraintType, RelationshipConstraint};
use crate::error::ModelResult;
use crate::query::{OrderDirection, QueryBuilder, QueryOperator};
use std::collections::HashSet;

/// Builder for relationship constraints with type safety and validation
#[derive(Debug)]
pub struct RelationshipConstraintBuilder {
    constraints: Vec<Box<dyn RelationshipConstraint>>,
    /// Track constraint types to prevent conflicts
    applied_types: HashSet<ConstraintType>,
}

impl RelationshipConstraintBuilder {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            applied_types: HashSet::new(),
        }
    }

    /// Apply all constraints to the query builder
    pub async fn apply_all(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        for constraint in &self.constraints {
            constraint.validate()?;
            constraint.apply(query).await?;
        }
        Ok(())
    }

    /// Get all constraints
    pub fn constraints(&self) -> &[Box<dyn RelationshipConstraint>] {
        &self.constraints
    }

    /// Check if a constraint type has been applied
    pub fn has_constraint_type(&self, constraint_type: &ConstraintType) -> bool {
        self.applied_types.contains(constraint_type)
    }

    /// Add a constraint and track its type
    fn add_constraint(&mut self, constraint: Box<dyn RelationshipConstraint>) {
        let constraint_type = constraint.constraint_type();
        self.applied_types.insert(constraint_type);
        self.constraints.push(constraint);
    }

    /// Add WHERE equals constraint
    pub fn where_eq<V>(mut self, field: &str, value: V) -> Self
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::Equal,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add WHERE not equals constraint
    pub fn where_ne<V>(mut self, field: &str, value: V) -> Self
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::NotEqual,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add WHERE greater than constraint
    pub fn where_gt<V>(mut self, field: &str, value: V) -> Self
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::GreaterThan,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add WHERE greater than or equal constraint
    pub fn where_gte<V>(mut self, field: &str, value: V) -> Self
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::GreaterThanOrEqual,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add WHERE less than constraint
    pub fn where_lt<V>(mut self, field: &str, value: V) -> Self
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::LessThan,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add WHERE less than or equal constraint
    pub fn where_lte<V>(mut self, field: &str, value: V) -> Self
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::LessThanOrEqual,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add WHERE LIKE constraint
    pub fn where_like(mut self, field: &str, pattern: &str) -> Self {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::Like,
            value: serde_json::Value::String(pattern.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add WHERE IN constraint
    pub fn where_in<V>(mut self, field: &str, values: Vec<V>) -> Self
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereInConstraint {
            field: field.to_string(),
            values: values
                .into_iter()
                .map(|v| serde_json::Value::String(v.to_string()))
                .collect(),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add raw WHERE constraint
    pub fn where_raw(mut self, condition: &str) -> Self {
        let constraint = RawConstraint {
            sql: condition.to_string(),
            constraint_type: ConstraintType::Where,
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add ORDER BY constraint
    pub fn order_by(mut self, field: &str) -> Self {
        let constraint = OrderConstraint {
            field: field.to_string(),
            direction: OrderDirection::Asc,
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add ORDER BY DESC constraint
    pub fn order_by_desc(mut self, field: &str) -> Self {
        let constraint = OrderConstraint {
            field: field.to_string(),
            direction: OrderDirection::Desc,
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add LIMIT constraint
    pub fn limit(mut self, count: i64) -> Self {
        let constraint = LimitConstraint { count };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add OFFSET constraint
    pub fn offset(mut self, count: i64) -> Self {
        let constraint = OffsetConstraint { count };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add GROUP BY constraint
    pub fn group_by(mut self, field: &str) -> Self {
        let constraint = GroupByConstraint {
            field: field.to_string(),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add HAVING constraint
    pub fn having<V>(mut self, field: &str, operator: QueryOperator, value: V) -> Self
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = HavingConstraint {
            field: field.to_string(),
            operator,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add raw HAVING constraint
    pub fn having_raw(mut self, condition: &str) -> Self {
        let constraint = RawConstraint {
            sql: condition.to_string(),
            constraint_type: ConstraintType::Having,
        };
        self.add_constraint(Box::new(constraint));
        self
    }
}

impl Default for RelationshipConstraintBuilder {
    fn default() -> Self {
        Self::new()
    }
}
