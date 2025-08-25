//! Query Builder UPSERT operations

use super::builder::QueryBuilder;
use super::types::*;
use serde_json::Value;

/// Builder for UPSERT operations (INSERT ... ON CONFLICT UPDATE)
#[derive(Debug)]
pub struct UpsertBuilder<M = ()> {
    pub(crate) query_builder: QueryBuilder<M>,
    pub(crate) conflict_columns: Vec<String>,
    pub(crate) update_clauses: Vec<SetClause>,
}

impl<M> UpsertBuilder<M> {
    /// Add an update clause for conflict resolution
    pub fn update_set<T: Into<Value>>(mut self, column: &str, value: T) -> Self {
        self.update_clauses.push(SetClause {
            column: column.to_string(),
            value: Some(value.into()),
        });
        self
    }

    /// Add a NULL update clause for conflict resolution
    pub fn update_set_null(mut self, column: &str) -> Self {
        self.update_clauses.push(SetClause {
            column: column.to_string(),
            value: None,
        });
        self
    }

    /// Finish building the upsert query
    pub fn build(self) -> QueryBuilder<M> {
        // For now, we'll return the underlying query builder
        // In a full implementation, this would store the upsert information
        self.query_builder
    }

    /// Generate SQL for the upsert operation
    pub fn to_sql_with_params(&self) -> (String, Vec<String>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        let mut param_counter = 1;

        // Start with INSERT
        if let Some(table) = &self.query_builder.insert_table {
            sql.push_str(&format!("INSERT INTO {}", table));

            if !self.query_builder.set_clauses.is_empty() {
                sql.push_str(" (");
                let columns: Vec<String> = self
                    .query_builder
                    .set_clauses
                    .iter()
                    .map(|clause| clause.column.clone())
                    .collect();
                sql.push_str(&columns.join(", "));
                sql.push_str(") VALUES (");

                for (i, clause) in self.query_builder.set_clauses.iter().enumerate() {
                    if i > 0 {
                        sql.push_str(", ");
                    }
                    if let Some(ref value) = clause.value {
                        sql.push_str(&format!("${}", param_counter));
                        params.push(value.to_string());
                        param_counter += 1;
                    } else {
                        sql.push_str("NULL");
                    }
                }
                sql.push(')');
            }
        }

        // Add ON CONFLICT clause
        if !self.conflict_columns.is_empty() {
            sql.push_str(&format!(
                " ON CONFLICT ({}) DO UPDATE SET ",
                self.conflict_columns.join(", ")
            ));

            for (i, clause) in self.update_clauses.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str(&format!("{} = ", clause.column));
                if let Some(ref value) = clause.value {
                    sql.push_str(&format!("${}", param_counter));
                    params.push(value.to_string());
                    param_counter += 1;
                } else {
                    sql.push_str("NULL");
                }
            }
        }

        (sql, params)
    }
}
