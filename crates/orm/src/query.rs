//! Query Builder - Type-safe, fluent query builder for complex database operations
//!
//! Provides a fluent interface for building queries with type safety,
//! compile-time validation, joins, subqueries, aggregations, and pagination.

use std::fmt;
use std::marker::PhantomData;
use serde_json::Value;
use sqlx::Row;

use crate::error::ModelResult;
use crate::model::Model;

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

/// Order by clause
#[derive(Debug, Clone)]
pub struct OrderByClause {
    pub column: String,
    pub direction: OrderDirection,
}

/// Query builder for constructing database queries
#[derive(Debug, Clone)]
pub struct QueryBuilder<M = ()> {
    select_fields: Vec<String>,
    from_table: Option<String>,
    where_conditions: Vec<WhereCondition>,
    joins: Vec<JoinClause>,
    order_by: Vec<OrderByClause>,
    group_by: Vec<String>,
    having_conditions: Vec<WhereCondition>,
    limit_value: Option<i64>,
    offset_value: Option<i64>,
    distinct: bool,
    _phantom: PhantomData<M>,
}

impl<M> Default for QueryBuilder<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> QueryBuilder<M> {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            select_fields: Vec::new(),
            from_table: None,
            where_conditions: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having_conditions: Vec::new(),
            limit_value: None,
            offset_value: None,
            distinct: false,
            _phantom: PhantomData,
        }
    }

    // <<<ELIF:BEGIN agent-editable:query_builder_select>>>
    /// Add SELECT fields to the query
    pub fn select(mut self, fields: &str) -> Self {
        if fields == "*" {
            self.select_fields.push("*".to_string());
        } else {
            self.select_fields.extend(
                fields
                    .split(',')
                    .map(|f| f.trim().to_string())
                    .collect::<Vec<String>>()
            );
        }
        self
    }

    /// Add SELECT DISTINCT to the query
    pub fn select_distinct(mut self, fields: &str) -> Self {
        self.distinct = true;
        self.select(fields)
    }

    /// Set the FROM table
    pub fn from(mut self, table: &str) -> Self {
        self.from_table = Some(table.to_string());
        self
    }
    // <<<ELIF:END agent-editable:query_builder_select>>>

    // <<<ELIF:BEGIN agent-editable:query_builder_where>>>
    /// Add WHERE condition with equality
    pub fn where_eq<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
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
    // <<<ELIF:END agent-editable:query_builder_where>>>

    // <<<ELIF:BEGIN agent-editable:query_builder_joins>>>
    /// Add INNER JOIN to the query
    pub fn join(mut self, table: &str, left_col: &str, right_col: &str) -> Self {
        self.joins.push(JoinClause {
            join_type: JoinType::Inner,
            table: table.to_string(),
            on_conditions: vec![(left_col.to_string(), right_col.to_string())],
        });
        self
    }

    /// Add LEFT JOIN to the query
    pub fn left_join(mut self, table: &str, left_col: &str, right_col: &str) -> Self {
        self.joins.push(JoinClause {
            join_type: JoinType::Left,
            table: table.to_string(),
            on_conditions: vec![(left_col.to_string(), right_col.to_string())],
        });
        self
    }

    /// Add RIGHT JOIN to the query
    pub fn right_join(mut self, table: &str, left_col: &str, right_col: &str) -> Self {
        self.joins.push(JoinClause {
            join_type: JoinType::Right,
            table: table.to_string(),
            on_conditions: vec![(left_col.to_string(), right_col.to_string())],
        });
        self
    }
    // <<<ELIF:END agent-editable:query_builder_joins>>>

    // <<<ELIF:BEGIN agent-editable:query_builder_order>>>
    /// Add ORDER BY clause (ascending)
    pub fn order_by(mut self, column: &str) -> Self {
        self.order_by.push(OrderByClause {
            column: column.to_string(),
            direction: OrderDirection::Asc,
        });
        self
    }

    /// Add ORDER BY clause (descending)
    pub fn order_by_desc(mut self, column: &str) -> Self {
        self.order_by.push(OrderByClause {
            column: column.to_string(),
            direction: OrderDirection::Desc,
        });
        self
    }

    /// Add GROUP BY clause
    pub fn group_by(mut self, column: &str) -> Self {
        self.group_by.push(column.to_string());
        self
    }

    /// Add HAVING clause (same as WHERE for now)
    pub fn having_eq<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.having_conditions.push(WhereCondition {
            column: column.to_string(),
            operator: QueryOperator::Equal,
            value: Some(value.into()),
            values: Vec::new(),
        });
        self
    }
    // <<<ELIF:END agent-editable:query_builder_order>>>

    // <<<ELIF:BEGIN agent-editable:query_builder_pagination>>>
    /// Add LIMIT clause
    pub fn limit(mut self, count: i64) -> Self {
        self.limit_value = Some(count);
        self
    }

    /// Add OFFSET clause
    pub fn offset(mut self, count: i64) -> Self {
        self.offset_value = Some(count);
        self
    }

    /// Add pagination (LIMIT + OFFSET)
    pub fn paginate(mut self, per_page: i64, page: i64) -> Self {
        self.limit_value = Some(per_page);
        self.offset_value = Some((page - 1) * per_page);
        self
    }
    // <<<ELIF:END agent-editable:query_builder_pagination>>>

    // <<<ELIF:BEGIN agent-editable:query_builder_sql_generation>>>
    /// Convert the query to SQL string
    pub fn to_sql(&self) -> String {
        let mut sql = String::new();

        // SELECT clause
        if self.distinct {
            sql.push_str("SELECT DISTINCT ");
        } else {
            sql.push_str("SELECT ");
        }

        if self.select_fields.is_empty() {
            sql.push('*');
        } else {
            sql.push_str(&self.select_fields.join(", "));
        }

        // FROM clause
        if let Some(table) = &self.from_table {
            sql.push_str(&format!(" FROM {}", table));
        }

        // JOIN clauses
        for join in &self.joins {
            sql.push_str(&format!(" {} {}", join.join_type, join.table));
            if !join.on_conditions.is_empty() {
                sql.push_str(" ON ");
                let conditions: Vec<String> = join
                    .on_conditions
                    .iter()
                    .map(|(left, right)| format!("{} = {}", left, right))
                    .collect();
                sql.push_str(&conditions.join(" AND "));
            }
        }

        // WHERE clause
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            let conditions = self.build_where_conditions(&self.where_conditions);
            sql.push_str(&conditions.join(" AND "));
        }

        // GROUP BY clause
        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        // HAVING clause
        if !self.having_conditions.is_empty() {
            sql.push_str(" HAVING ");
            let conditions = self.build_where_conditions(&self.having_conditions);
            sql.push_str(&conditions.join(" AND "));
        }

        // ORDER BY clause
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_clauses: Vec<String> = self
                .order_by
                .iter()
                .map(|clause| format!("{} {}", clause.column, clause.direction))
                .collect();
            sql.push_str(&order_clauses.join(", "));
        }

        // LIMIT clause
        if let Some(limit) = self.limit_value {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // OFFSET clause
        if let Some(offset) = self.offset_value {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        sql
    }

    /// Build WHERE condition strings
    fn build_where_conditions(&self, conditions: &[WhereCondition]) -> Vec<String> {
        conditions
            .iter()
            .map(|condition| {
                match &condition.operator {
                    QueryOperator::IsNull | QueryOperator::IsNotNull => {
                        format!("{} {}", condition.column, condition.operator)
                    }
                    QueryOperator::In | QueryOperator::NotIn => {
                        let values: Vec<String> = condition
                            .values
                            .iter()
                            .map(|v| self.format_value(v))
                            .collect();
                        format!("{} {} ({})", condition.column, condition.operator, values.join(", "))
                    }
                    QueryOperator::Between => {
                        if condition.values.len() == 2 {
                            format!(
                                "{} BETWEEN {} AND {}",
                                condition.column,
                                self.format_value(&condition.values[0]),
                                self.format_value(&condition.values[1])
                            )
                        } else {
                            format!("{} = NULL", condition.column) // Invalid BETWEEN
                        }
                    }
                    _ => {
                        if let Some(value) = &condition.value {
                            format!("{} {} {}", condition.column, condition.operator, self.format_value(value))
                        } else {
                            format!("{} = NULL", condition.column) // Fallback
                        }
                    }
                }
            })
            .collect()
    }

    /// Format a value for SQL
    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::String(s) => format!("'{}'", s.replace('\'', "''")), // Escape single quotes
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "NULL".to_string(),
            _ => "NULL".to_string(), // Arrays and objects not yet supported
        }
    }
    // <<<ELIF:END agent-editable:query_builder_sql_generation>>>

    /// Get parameter bindings (for prepared statements)
    /// TODO: Implement parameter binding for security
    pub fn bindings(&self) -> Vec<Value> {
        let mut bindings = Vec::new();
        
        for condition in &self.where_conditions {
            if let Some(value) = &condition.value {
                bindings.push(value.clone());
            }
            bindings.extend(condition.values.clone());
        }

        for condition in &self.having_conditions {
            if let Some(value) = &condition.value {
                bindings.push(value.clone());
            }
            bindings.extend(condition.values.clone());
        }

        bindings
    }
}

// Implement specialized methods for Model-typed query builders
impl<M: Model> QueryBuilder<M> {
    /// Execute query and return models
    pub async fn get(self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<Vec<M>> {
        let sql = self.to_sql();
        let rows = sqlx::query(&sql)
            .fetch_all(pool)
            .await?;

        let mut models = Vec::new();
        for row in rows {
            models.push(M::from_row(&row)?);
        }

        Ok(models)
    }

    /// Execute query and return first model
    pub async fn first(self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<Option<M>> {
        let query = self.limit(1);
        let mut results = query.get(pool).await?;
        Ok(results.pop())
    }

    /// Execute query and return first model or error
    pub async fn first_or_fail(self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<M> {
        self.first(pool)
            .await?
            .ok_or_else(|| crate::error::ModelError::NotFound(M::table_name().to_string()))
    }

    /// Count query results
    pub async fn count(mut self, pool: &sqlx::Pool<sqlx::Postgres>) -> ModelResult<i64> {
        self.select_fields = vec!["COUNT(*)".to_string()];
        let sql = self.to_sql();
        
        let row = sqlx::query(&sql)
            .fetch_one(pool)
            .await?;

        let count: i64 = row.try_get(0)?;
        Ok(count)
    }
}