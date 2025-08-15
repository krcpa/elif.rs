//! Query Builder SQL generation

use serde_json::Value;
use super::builder::QueryBuilder;
use super::types::*;

impl<M> QueryBuilder<M> {
    /// Generate SQL from query with parameter placeholders and return parameters
    pub fn to_sql_with_params(&self) -> (String, Vec<String>) {
        match self.query_type {
            QueryType::Select => self.build_select_sql(),
            QueryType::Insert => self.build_insert_sql(),
            QueryType::Update => self.build_update_sql(),
            QueryType::Delete => self.build_delete_sql(),
        }
    }

    /// Build SELECT SQL with parameters
    fn build_select_sql(&self) -> (String, Vec<String>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        let mut param_counter = 1;

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
        if !self.from_tables.is_empty() {
            sql.push_str(" FROM ");
            sql.push_str(&self.from_tables.join(", "));
        }

        // JOIN clauses
        for join in &self.joins {
            sql.push(' ');
            sql.push_str(&join.join_type.to_string());
            sql.push(' ');
            sql.push_str(&join.table);
            sql.push_str(" ON ");
            for (i, (left, right)) in join.on_conditions.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(&format!("{} = {}", left, right));
            }
        }

        self.build_where_clause(&mut sql, &mut params, &mut param_counter);
        self.build_order_limit_clause(&mut sql);

        (sql, params)
    }

    /// Build INSERT SQL with parameters
    fn build_insert_sql(&self) -> (String, Vec<String>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        let mut param_counter = 1;

        if let Some(table) = &self.insert_table {
            sql.push_str(&format!("INSERT INTO {}", table));
            
            if !self.set_clauses.is_empty() {
                sql.push_str(" (");
                let columns: Vec<String> = self.set_clauses.iter()
                    .map(|clause| clause.column.clone())
                    .collect();
                sql.push_str(&columns.join(", "));
                sql.push_str(") VALUES (");
                
                for (i, clause) in self.set_clauses.iter().enumerate() {
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

        (sql, params)
    }

    /// Build UPDATE SQL with parameters
    fn build_update_sql(&self) -> (String, Vec<String>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        let mut param_counter = 1;

        if let Some(table) = &self.update_table {
            sql.push_str(&format!("UPDATE {}", table));
            
            if !self.set_clauses.is_empty() {
                sql.push_str(" SET ");
                for (i, clause) in self.set_clauses.iter().enumerate() {
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

            self.build_where_clause(&mut sql, &mut params, &mut param_counter);
        }

        (sql, params)
    }

    /// Build DELETE SQL with parameters
    fn build_delete_sql(&self) -> (String, Vec<String>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        let mut param_counter = 1;

        if let Some(table) = &self.delete_table {
            sql.push_str(&format!("DELETE FROM {}", table));
            self.build_where_clause(&mut sql, &mut params, &mut param_counter);
        }

        (sql, params)
    }

    /// Helper method to build WHERE clauses
    fn build_where_clause(&self, sql: &mut String, params: &mut Vec<String>, param_counter: &mut i32) {
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            for (i, condition) in self.where_conditions.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                
                sql.push_str(&condition.column);
                sql.push(' ');

                match condition.operator {
                    QueryOperator::In | QueryOperator::NotIn => {
                        sql.push_str(&condition.operator.to_string());
                        sql.push_str(" (");
                        for (j, value) in condition.values.iter().enumerate() {
                            if j > 0 {
                                sql.push_str(", ");
                            }
                            sql.push_str(&format!("${}", param_counter));
                            params.push(value.to_string());
                            *param_counter += 1;
                        }
                        sql.push(')');
                    }
                    QueryOperator::Between => {
                        sql.push_str(&condition.operator.to_string());
                        sql.push_str(&format!(" ${} AND ${}", param_counter, *param_counter + 1));
                        if condition.values.len() >= 2 {
                            params.push(condition.values[0].to_string());
                            params.push(condition.values[1].to_string());
                        }
                        *param_counter += 2;
                    }
                    QueryOperator::IsNull | QueryOperator::IsNotNull => {
                        sql.push_str(&condition.operator.to_string());
                    }
                    _ => {
                        sql.push_str(&condition.operator.to_string());
                        if let Some(ref value) = condition.value {
                            sql.push_str(&format!(" ${}", param_counter));
                            params.push(value.to_string());
                            *param_counter += 1;
                        }
                    }
                }
            }
        }
    }

    /// Helper method to build ORDER BY and LIMIT clauses
    fn build_order_limit_clause(&self, sql: &mut String) {
        // ORDER BY clause
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            for (i, (column, direction)) in self.order_by.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str(&format!("{} {}", column, direction));
            }
        }

        // LIMIT clause
        if let Some(limit) = self.limit_count {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // OFFSET clause
        if let Some(offset) = self.offset_value {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
    }

    /// Convert the query to SQL string (for backwards compatibility)
    pub fn to_sql(&self) -> String {
        match self.query_type {
            QueryType::Select => self.build_select_sql_simple(),
            _ => self.to_sql_with_params().0,
        }
    }

    /// Build SELECT SQL without parameters (for testing and simple queries)
    fn build_select_sql_simple(&self) -> String {
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
        if !self.from_tables.is_empty() {
            sql.push_str(" FROM ");
            sql.push_str(&self.from_tables.join(", "));
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
                .map(|(column, direction)| format!("{} {}", column, direction))
                .collect();
            sql.push_str(&order_clauses.join(", "));
        }

        // LIMIT clause
        if let Some(limit) = self.limit_count {
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
                // Handle special raw conditions
                if condition.column == "RAW" {
                    if let Some(Value::String(raw_sql)) = &condition.value {
                        return raw_sql.clone();
                    }
                }
                
                // Handle EXISTS and NOT EXISTS
                if condition.column == "EXISTS" || condition.column == "NOT EXISTS" {
                    if let Some(Value::String(subquery)) = &condition.value {
                        return format!("{} {}", condition.column, subquery);
                    }
                }
                
                match &condition.operator {
                    QueryOperator::IsNull | QueryOperator::IsNotNull => {
                        format!("{} {}", condition.column, condition.operator)
                    }
                    QueryOperator::In | QueryOperator::NotIn => {
                        // Handle subqueries (stored in value field) vs regular IN lists (stored in values field)
                        if let Some(Value::String(subquery)) = &condition.value {
                            if subquery.starts_with('(') && subquery.ends_with(')') {
                                // This is a subquery
                                format!("{} {} {}", condition.column, condition.operator, subquery)
                            } else {
                                // Single value IN (unusual case)
                                format!("{} {} ({})", condition.column, condition.operator, self.format_value(&condition.value.as_ref().unwrap()))
                            }
                        } else {
                            // Regular IN with multiple values
                            let values: Vec<String> = condition
                                .values
                                .iter()
                                .map(|v| self.format_value(v))
                                .collect();
                            format!("{} {} ({})", condition.column, condition.operator, values.join(", "))
                        }
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
                            // Handle subquery values
                            if let Value::String(val_str) = value {
                                if val_str.starts_with('(') && val_str.ends_with(')') {
                                    // This looks like a subquery
                                    format!("{} {} {}", condition.column, condition.operator, val_str)
                                } else {
                                    format!("{} {} {}", condition.column, condition.operator, self.format_value(value))
                                }
                            } else {
                                format!("{} {} {}", condition.column, condition.operator, self.format_value(value))
                            }
                        } else {
                            format!("{} = NULL", condition.column) // Fallback
                        }
                    }
                }
            })
            .collect()
    }

    /// Format a value for SQL
    pub(crate) fn format_value(&self, value: &Value) -> String {
        match value {
            Value::String(s) => format!("'{}'", s.replace('\'', "''")), // Escape single quotes
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "NULL".to_string(),
            _ => "NULL".to_string(), // Arrays and objects not yet supported
        }
    }
}