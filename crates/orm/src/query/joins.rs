//! Query Builder JOIN operations

use super::builder::QueryBuilder;
use super::types::*;

impl<M> QueryBuilder<M> {
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
}
