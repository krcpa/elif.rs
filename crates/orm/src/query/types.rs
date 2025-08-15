//! Query Builder Types - Core types and enums for query building

use std::fmt;
use serde_json::Value;

/// Query operator types
#[derive(Debug, Clone, PartialEq)]
pub enum QueryOperator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Like,
    NotLike,
    In,
    NotIn,
    IsNull,
    IsNotNull,
    Between,
}

impl fmt::Display for QueryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryOperator::Equal => write!(f, "="),
            QueryOperator::NotEqual => write!(f, "!="),
            QueryOperator::GreaterThan => write!(f, ">"),
            QueryOperator::GreaterThanOrEqual => write!(f, ">="),
            QueryOperator::LessThan => write!(f, "<"),
            QueryOperator::LessThanOrEqual => write!(f, "<="),
            QueryOperator::Like => write!(f, "LIKE"),
            QueryOperator::NotLike => write!(f, "NOT LIKE"),
            QueryOperator::In => write!(f, "IN"),
            QueryOperator::NotIn => write!(f, "NOT IN"),
            QueryOperator::IsNull => write!(f, "IS NULL"),
            QueryOperator::IsNotNull => write!(f, "IS NOT NULL"),
            QueryOperator::Between => write!(f, "BETWEEN"),
        }
    }
}

/// Where clause condition
#[derive(Debug, Clone)]
pub struct WhereCondition {
    pub column: String,
    pub operator: QueryOperator,
    pub value: Option<Value>,
    pub values: Vec<Value>, // For IN, NOT IN, BETWEEN
}

/// Join types
#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl fmt::Display for JoinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JoinType::Inner => write!(f, "INNER JOIN"),
            JoinType::Left => write!(f, "LEFT JOIN"),
            JoinType::Right => write!(f, "RIGHT JOIN"),
            JoinType::Full => write!(f, "FULL JOIN"),
        }
    }
}

/// Join clause
#[derive(Debug, Clone)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub on_conditions: Vec<(String, String)>, // (left_column, right_column)
}

/// Order by direction
#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl fmt::Display for OrderDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderDirection::Asc => write!(f, "ASC"),
            OrderDirection::Desc => write!(f, "DESC"),
        }
    }
}

/// Query types supported by the builder
#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
}

/// Set clause for UPDATE and INSERT operations
#[derive(Debug, Clone)]
pub struct SetClause {
    pub column: String,
    pub value: Option<Value>, // None for NULL values
}