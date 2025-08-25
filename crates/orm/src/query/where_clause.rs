//! Query Builder WHERE clause operations

use super::builder::QueryBuilder;
use super::types::*;
use serde_json::Value;

impl<M> QueryBuilder<M> {
    /// Add WHERE condition with equality
    pub fn where_eq<T>(mut self, column: &str, value: T) -> Self
    where
        T: Into<Value>,
    {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::Equal,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with not equal
    pub fn where_ne<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::NotEqual,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with greater than
    pub fn where_gt<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::GreaterThan,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with greater than or equal
    pub fn where_gte<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::GreaterThanOrEqual,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with less than
    pub fn where_lt<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::LessThan,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with less than or equal
    pub fn where_lte<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::LessThanOrEqual,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with LIKE
    pub fn where_like(mut self, column: &str, pattern: &str) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::Like,
            value: Some(Value::String(pattern.to_string())),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with NOT LIKE
    pub fn where_not_like(mut self, column: &str, pattern: &str) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::NotLike,
            value: Some(Value::String(pattern.to_string())),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with custom operator
    pub fn where_condition<T: Into<Value>>(
        mut self,
        column: &str,
        operator: &str,
        value: T,
    ) -> Self {
        let query_operator = match operator {
            "=" => QueryOperator::Equal,
            "!=" | "<>" => QueryOperator::NotEqual,
            ">" => QueryOperator::GreaterThan,
            ">=" => QueryOperator::GreaterThanOrEqual,
            "<" => QueryOperator::LessThan,
            "<=" => QueryOperator::LessThanOrEqual,
            "LIKE" => QueryOperator::Like,
            "NOT LIKE" => QueryOperator::NotLike,
            _ => QueryOperator::Equal, // Default fallback
        };

        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: query_operator,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with IN
    pub fn where_in<T: Into<Value>>(mut self, column: &str, values: Vec<T>) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::In,
            value: None,
            values: values.into_iter().map(|v| v.into()).collect(),
        });
        self
    }

    /// Add WHERE condition with NOT IN
    pub fn where_not_in<T: Into<Value>>(mut self, column: &str, values: Vec<T>) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::NotIn,
            value: None,
            values: values.into_iter().map(|v| v.into()).collect(),
        });
        self
    }

    /// Add WHERE condition with IS NULL
    pub fn where_null(mut self, column: &str) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::IsNull,
            value: None,
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with IS NOT NULL
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::IsNotNull,
            value: None,
            values: Vec::new(),
        });
        self
    }

    /// Add WHERE condition with BETWEEN
    pub fn where_between<T: Into<Value>>(mut self, column: &str, start: T, end: T) -> Self {
        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::Between,
            value: None,
            values: vec![start.into(), end.into()],
        });
        self
    }

    /// Add raw WHERE condition for complex cases
    pub fn where_raw(mut self, raw_condition: &str) -> Self {
        self.where_conditions.push(WhereCondition {
            column: "RAW".to_string(),
            operator: QueryOperator::Equal,
            value: Some(Value::String(raw_condition.to_string())),
            values: Vec::new(),
        });
        self
    }

    /// Add a subquery in the WHERE clause
    pub fn where_subquery<T>(
        mut self,
        column: &str,
        operator: QueryOperator,
        subquery: QueryBuilder<T>,
    ) -> Self {
        let subquery_sql = subquery.to_sql();
        let formatted_value = format!("({})", subquery_sql);

        self.where_conditions.push(WhereCondition {
            column: column.to_string(),
            operator,
            value: Some(Value::String(formatted_value)),
            values: Vec::new(),
        });
        self
    }

    /// Add EXISTS subquery condition
    pub fn where_exists<T>(mut self, subquery: QueryBuilder<T>) -> Self {
        self.where_conditions.push(WhereCondition {
            column: "EXISTS".to_string(),
            operator: QueryOperator::Equal,
            value: Some(Value::String(format!("({})", subquery.to_sql()))),
            values: Vec::new(),
        });
        self
    }

    /// Add NOT EXISTS subquery condition
    pub fn where_not_exists<T>(mut self, subquery: QueryBuilder<T>) -> Self {
        self.where_conditions.push(WhereCondition {
            column: "NOT EXISTS".to_string(),
            operator: QueryOperator::Equal,
            value: Some(Value::String(format!("({})", subquery.to_sql()))),
            values: Vec::new(),
        });
        self
    }
}
