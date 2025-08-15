//! Constraint implementations for relationship queries

use async_trait::async_trait;
use crate::error::{ModelError, ModelResult};
use crate::query::{QueryBuilder, OrderDirection, QueryOperator};
use super::types::{RelationshipConstraint, ConstraintType};

/// WHERE constraint implementation
#[derive(Debug, Clone)]
pub(crate) struct WhereConstraint {
    pub field: String,
    pub operator: QueryOperator,
    pub value: serde_json::Value,
}

#[async_trait]
impl RelationshipConstraint for WhereConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply the WHERE condition using the appropriate method based on operator
        *query = match self.operator {
            QueryOperator::Equal => query.clone().where_eq(&self.field, self.value.clone()),
            QueryOperator::NotEqual => query.clone().where_ne(&self.field, self.value.clone()),
            QueryOperator::GreaterThan => query.clone().where_gt(&self.field, self.value.clone()),
            QueryOperator::GreaterThanOrEqual => query.clone().where_gte(&self.field, self.value.clone()),
            QueryOperator::LessThan => query.clone().where_lt(&self.field, self.value.clone()),
            QueryOperator::LessThanOrEqual => query.clone().where_lte(&self.field, self.value.clone()),
            QueryOperator::Like => {
                if let Some(pattern) = self.value.as_str() {
                    query.clone().where_like(&self.field, pattern)
                } else {
                    return Err(ModelError::Validation("LIKE operator requires string value".to_string()));
                }
            },
            QueryOperator::NotLike => {
                if let Some(pattern) = self.value.as_str() {
                    query.clone().where_not_like(&self.field, pattern)
                } else {
                    return Err(ModelError::Validation("NOT LIKE operator requires string value".to_string()));
                }
            },
            _ => {
                return Err(ModelError::Validation(format!(
                    "Unsupported operator {:?} for WHERE constraint", self.operator
                )));
            }
        };
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Where
    }
    
    fn description(&self) -> String {
        format!("WHERE {} {:?} {}", self.field, self.operator, self.value)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("WHERE constraint field cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// ORDER BY constraint implementation  
#[derive(Debug, Clone)]
pub(crate) struct OrderConstraint {
    pub field: String,
    pub direction: OrderDirection,
}

#[async_trait]
impl RelationshipConstraint for OrderConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply the ORDER BY condition to the query builder
        *query = match self.direction {
            OrderDirection::Desc => query.clone().order_by_desc(&self.field),
            OrderDirection::Asc => query.clone().order_by(&self.field),
        };
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Order
    }
    
    fn description(&self) -> String {
        format!("ORDER BY {} {:?}", self.field, self.direction)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("ORDER BY constraint field cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// LIMIT constraint implementation
#[derive(Debug, Clone)]
pub(crate) struct LimitConstraint {
    pub count: i64,
}

#[async_trait] 
impl RelationshipConstraint for LimitConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply the LIMIT condition to the query builder
        *query = query.clone().limit(self.count);
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Limit
    }
    
    fn description(&self) -> String {
        format!("LIMIT {}", self.count)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.count < 0 {
            return Err(ModelError::Validation("LIMIT count must be non-negative".to_string()));
        }
        Ok(())
    }
}

/// OFFSET constraint implementation
#[derive(Debug, Clone)]
pub(crate) struct OffsetConstraint {
    pub count: i64,
}

#[async_trait]
impl RelationshipConstraint for OffsetConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        *query = query.clone().offset(self.count);
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Offset
    }
    
    fn description(&self) -> String {
        format!("OFFSET {}", self.count)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.count < 0 {
            return Err(ModelError::Validation("OFFSET count must be non-negative".to_string()));
        }
        Ok(())
    }
}

/// WHERE IN constraint implementation
#[derive(Debug, Clone)]
pub(crate) struct WhereInConstraint {
    pub field: String,
    pub values: Vec<serde_json::Value>,
}

#[async_trait]
impl RelationshipConstraint for WhereInConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Convert values to strings for the where_in method
        let string_values: Vec<String> = self.values
            .iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                _ => v.to_string(),
            })
            .collect();
        
        *query = query.clone().where_in(&self.field, string_values);
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Where
    }
    
    fn description(&self) -> String {
        format!("WHERE {} IN ({} values)", self.field, self.values.len())
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("WHERE IN constraint field cannot be empty".to_string()));
        }
        if self.values.is_empty() {
            return Err(ModelError::Validation("WHERE IN constraint must have at least one value".to_string()));
        }
        Ok(())
    }
}

/// GROUP BY constraint implementation
#[derive(Debug, Clone)]
pub(crate) struct GroupByConstraint {
    pub field: String,
}

#[async_trait]
impl RelationshipConstraint for GroupByConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        *query = query.clone().group_by(&self.field);
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::GroupBy
    }
    
    fn description(&self) -> String {
        format!("GROUP BY {}", self.field)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("GROUP BY constraint field cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// HAVING constraint implementation
#[derive(Debug, Clone)]
pub(crate) struct HavingConstraint {
    pub field: String,
    pub operator: QueryOperator,
    pub value: serde_json::Value,
}

#[async_trait]
impl RelationshipConstraint for HavingConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply HAVING constraint using the having method
        *query = query.clone().having(&self.field, self.operator, self.value.clone());
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Having
    }
    
    fn description(&self) -> String {
        format!("HAVING {} {:?} {}", self.field, self.operator, self.value)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("HAVING constraint field cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// Raw SQL constraint implementation for complex cases
#[derive(Debug, Clone)]
pub(crate) struct RawConstraint {
    pub sql: String,
    pub constraint_type: ConstraintType,
}

#[async_trait]
impl RelationshipConstraint for RawConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply raw constraint based on its type
        match self.constraint_type {
            ConstraintType::Where => {
                *query = query.clone().where_raw(&self.sql);
            },
            ConstraintType::Having => {
                *query = query.clone().having_raw(&self.sql);
            },
            ConstraintType::Raw => {
                // For generic raw constraints, we'd need a way to append raw SQL
                // This would require extending the QueryBuilder with a raw method
                return Err(ModelError::Validation("Raw constraints not yet supported".to_string()));
            },
            _ => {
                return Err(ModelError::Validation(format!(
                    "Raw constraints not supported for type {:?}", self.constraint_type
                )));
            }
        }
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        self.constraint_type.clone()
    }
    
    fn description(&self) -> String {
        format!("RAW {:?}: {}", self.constraint_type, self.sql)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.sql.trim().is_empty() {
            return Err(ModelError::Validation("Raw constraint SQL cannot be empty".to_string()));
        }
        Ok(())
    }
}