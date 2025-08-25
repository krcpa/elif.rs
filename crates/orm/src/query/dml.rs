//! Query Builder DML operations (INSERT, UPDATE, DELETE)

use super::builder::QueryBuilder;
use super::types::*;
use super::upsert::UpsertBuilder;
use serde_json::Value;

impl<M> QueryBuilder<M> {
    /// Start an INSERT query
    pub fn insert_into(mut self, table: &str) -> Self {
        self.query_type = QueryType::Insert;
        self.insert_table = Some(table.to_string());
        self
    }

    /// Start an UPDATE query
    pub fn update(mut self, table: &str) -> Self {
        self.query_type = QueryType::Update;
        self.update_table = Some(table.to_string());
        self
    }

    /// Start a DELETE query
    pub fn delete_from(mut self, table: &str) -> Self {
        self.query_type = QueryType::Delete;
        self.delete_table = Some(table.to_string());
        self
    }

    /// Set a column value (for INSERT/UPDATE)
    pub fn set<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.set_clauses.push(SetClause {
            column: column.to_string(),
            value: Some(value.into()),
        });
        self
    }

    /// Set a column to NULL (for INSERT/UPDATE)
    pub fn set_null(mut self, column: &str) -> Self {
        self.set_clauses.push(SetClause {
            column: column.to_string(),
            value: None,
        });
        self
    }

    /// Set multiple values at once
    pub fn set_values(mut self, values: Vec<(String, Value)>) -> Self {
        for (column, value) in values {
            self.set_clauses.push(SetClause {
                column,
                value: Some(value),
            });
        }
        self
    }

    /// Upsert operation (INSERT ... ON CONFLICT UPDATE)
    pub fn upsert(mut self, table: &str, conflict_columns: Vec<&str>) -> UpsertBuilder<M> {
        self.query_type = QueryType::Insert;
        self.insert_table = Some(table.to_string());

        UpsertBuilder {
            query_builder: self,
            conflict_columns: conflict_columns
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            update_clauses: Vec::new(),
        }
    }
}
