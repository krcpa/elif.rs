//! Query Builder ORDER BY, GROUP BY, HAVING operations

use serde_json::Value;
use crate::query::builder::QueryBuilder;
use crate::query::types::*;

impl<M> QueryBuilder<M> {
    /// Add ORDER BY clause (ascending)
    pub fn order_by(mut self, column: &str) -> Self {
        self.order_by.push((column.to_string(), OrderDirection::Asc));
        self
    }

    /// Add ORDER BY clause (descending)
    pub fn order_by_desc(mut self, column: &str) -> Self {
        self.order_by.push((column.to_string(), OrderDirection::Desc));
        self
    }

    /// Add GROUP BY clause
    pub fn group_by(mut self, column: &str) -> Self {
        self.group_by.push(column.to_string());
        self
    }

    /// Add HAVING clause with equals
    pub fn having_eq<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.having_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::Equal,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }
    
    /// Add HAVING clause with custom operator and value  
    pub fn having<T: Into<Value>>(mut self, column: &str, operator: QueryOperator, value: T) -> Self {
        self.having_conditions.push(WhereCondition {
            column: column.to_string(),
            operator,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }
    
    /// Add raw HAVING clause
    pub fn having_raw(mut self, raw_condition: &str) -> Self {
        self.having_conditions.push(WhereCondition {
            column: "".to_string(), // Empty column for raw conditions
            operator: QueryOperator::Raw,
            value: Some(Value::String(raw_condition.to_string())),
            values: Vec::new(),
        });
        self
    }
}