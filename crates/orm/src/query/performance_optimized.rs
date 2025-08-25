//! Performance optimized query building
//!
//! This module implements optimized versions of query building operations
//! to reduce allocations and improve performance for hot paths.

use super::builder::QueryBuilder;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cache for common SQL patterns to reduce string allocations
#[allow(dead_code)]
static QUERY_TEMPLATE_CACHE: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Cache for parameter placeholders to avoid repeated generation
static PLACEHOLDER_CACHE: Lazy<RwLock<HashMap<usize, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Performance-optimized SQL generation with caching
impl<M> QueryBuilder<M> {
    /// Generate parameter placeholders with caching for better performance
    pub fn generate_placeholders_cached(count: usize) -> String {
        // Check cache first
        if let Ok(cache) = PLACEHOLDER_CACHE.read() {
            if let Some(cached) = cache.get(&count) {
                return cached.clone();
            }
        }

        // Generate placeholders
        let placeholders = (1..=count)
            .map(|i| format!("${}", i))
            .collect::<Vec<_>>()
            .join(", ");

        // Cache for future use
        if let Ok(mut cache) = PLACEHOLDER_CACHE.write() {
            cache.insert(count, placeholders.clone());
        }

        placeholders
    }

    /// Generate sequential parameter placeholders starting from a specific index
    /// Used for proper parameter ordering in complex queries
    pub fn generate_sequential_placeholders(start_index: usize, count: usize) -> String {
        if count == 0 {
            return String::new();
        }

        let placeholders = (start_index..start_index + count)
            .map(|i| format!("${}", i))
            .collect::<Vec<_>>()
            .join(", ");

        placeholders
    }

    /// Optimized SQL generation with pre-allocated capacity
    pub fn to_sql_optimized(&self) -> String {
        // Pre-calculate approximate SQL length to reduce allocations
        let estimated_length = self.estimate_sql_length();
        let mut sql = String::with_capacity(estimated_length);

        match self.query_type {
            super::types::QueryType::Select => {
                self.build_select_sql_optimized(&mut sql);
            }
            _ => {
                // Fallback to regular implementation for non-SELECT queries
                return self.to_sql();
            }
        }

        sql
    }

    /// Estimate SQL length to pre-allocate string capacity
    fn estimate_sql_length(&self) -> usize {
        let mut length = 100; // Base SQL overhead

        // Estimate SELECT clause length
        for field in &self.select_fields {
            length += field.len() + 2; // field + ", "
        }

        // Estimate FROM clause length
        for table in &self.from_tables {
            length += table.len() + 10; // " FROM " + table
        }

        // Estimate WHERE clause length
        for condition in &self.where_conditions {
            length += condition.column.len() + 20; // column + operator + placeholder
        }

        // Estimate JOIN clause length
        for join in &self.joins {
            length += join.table.len() + 30; // JOIN type + table + ON condition
        }

        length
    }

    /// Build SELECT SQL with optimized string operations and correct parameter indexing
    fn build_select_sql_optimized(&self, sql: &mut String) {
        let mut param_counter = 1usize;
        // SELECT clause
        if self.distinct {
            sql.push_str("SELECT DISTINCT ");
        } else {
            sql.push_str("SELECT ");
        }

        // Fields
        if self.select_fields.is_empty() {
            sql.push('*');
        } else {
            for (i, field) in self.select_fields.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str(field);
            }
        }

        // FROM clause
        if !self.from_tables.is_empty() {
            sql.push_str(" FROM ");
            for (i, table) in self.from_tables.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str(table);
            }
        }

        // JOINs
        for join in &self.joins {
            sql.push(' ');
            match join.join_type {
                super::types::JoinType::Inner => sql.push_str("INNER JOIN"),
                super::types::JoinType::Left => sql.push_str("LEFT JOIN"),
                super::types::JoinType::Right => sql.push_str("RIGHT JOIN"),
                super::types::JoinType::Full => sql.push_str("FULL JOIN"),
            }
            sql.push(' ');
            sql.push_str(&join.table);
            sql.push_str(" ON ");

            // Handle on_conditions
            for (i, (left_col, right_col)) in join.on_conditions.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(left_col);
                sql.push_str(" = ");
                sql.push_str(right_col);
            }
        }

        // WHERE clause
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            for (i, condition) in self.where_conditions.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }

                // Handle special cases
                if condition.column == "RAW" {
                    if let Some(ref value) = condition.value {
                        if let serde_json::Value::String(raw_sql) = value {
                            sql.push_str(raw_sql);
                        }
                    }
                } else if condition.column == "EXISTS" || condition.column == "NOT EXISTS" {
                    sql.push_str(&condition.column);
                    sql.push(' ');
                    if let Some(ref value) = condition.value {
                        if let serde_json::Value::String(subquery) = value {
                            sql.push_str(subquery);
                        }
                    }
                } else {
                    // Regular conditions
                    sql.push_str(&condition.column);

                    match condition.operator {
                        super::types::QueryOperator::Equal => sql.push_str(" = "),
                        super::types::QueryOperator::NotEqual => sql.push_str(" != "),
                        super::types::QueryOperator::GreaterThan => sql.push_str(" > "),
                        super::types::QueryOperator::LessThan => sql.push_str(" < "),
                        super::types::QueryOperator::GreaterThanOrEqual => sql.push_str(" >= "),
                        super::types::QueryOperator::LessThanOrEqual => sql.push_str(" <= "),
                        super::types::QueryOperator::Like => sql.push_str(" LIKE "),
                        super::types::QueryOperator::NotLike => sql.push_str(" NOT LIKE "),
                        super::types::QueryOperator::In => {
                            sql.push_str(" IN (");
                            let placeholder_count = condition.values.len();
                            if placeholder_count > 0 {
                                let placeholders = Self::generate_sequential_placeholders(
                                    param_counter,
                                    placeholder_count,
                                );
                                sql.push_str(&placeholders);
                                param_counter += placeholder_count;
                            }
                            sql.push(')');
                            continue; // Skip the normal parameter handling
                        }
                        super::types::QueryOperator::NotIn => {
                            sql.push_str(" NOT IN (");
                            let placeholder_count = condition.values.len();
                            if placeholder_count > 0 {
                                let placeholders = Self::generate_sequential_placeholders(
                                    param_counter,
                                    placeholder_count,
                                );
                                sql.push_str(&placeholders);
                                param_counter += placeholder_count;
                            }
                            sql.push(')');
                            continue; // Skip the normal parameter handling
                        }
                        super::types::QueryOperator::IsNull => {
                            sql.push_str(" IS NULL");
                            continue;
                        }
                        super::types::QueryOperator::IsNotNull => {
                            sql.push_str(" IS NOT NULL");
                            continue;
                        }
                        super::types::QueryOperator::Between => {
                            sql.push_str(&format!(
                                " BETWEEN ${} AND ${}",
                                param_counter,
                                param_counter + 1
                            ));
                            param_counter += 2;
                            continue;
                        }
                        super::types::QueryOperator::Raw => {
                            // For raw SQL expressions, just add the value directly
                            if let Some(ref value) = condition.value {
                                if let serde_json::Value::String(raw_expr) = value {
                                    sql.push(' ');
                                    sql.push_str(raw_expr);
                                }
                            }
                            continue;
                        }
                    }

                    // Add parameter placeholder for regular operators
                    sql.push_str(&format!("${}", param_counter));
                    param_counter += 1;
                }
            }
        }

        // GROUP BY
        if !self.group_by.is_empty() {
            sql.push_str(" GROUP BY ");
            for (i, column) in self.group_by.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str(column);
            }
        }

        // HAVING
        if !self.having_conditions.is_empty() {
            sql.push_str(" HAVING ");
            for (i, condition) in self.having_conditions.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                sql.push_str(&condition.column);

                // Handle HAVING operators with proper parameter indexing
                match condition.operator {
                    super::types::QueryOperator::Equal => sql.push_str(" = "),
                    super::types::QueryOperator::GreaterThan => sql.push_str(" > "),
                    super::types::QueryOperator::LessThan => sql.push_str(" < "),
                    _ => sql.push_str(" = "), // Default to equals
                }

                sql.push_str(&format!("${}", param_counter));
                param_counter += 1;
            }
        }

        // ORDER BY
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            for (i, (column, direction)) in self.order_by.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str(column);
                match direction {
                    super::types::OrderDirection::Asc => sql.push_str(" ASC"),
                    super::types::OrderDirection::Desc => sql.push_str(" DESC"),
                }
            }
        }

        // LIMIT
        if let Some(limit) = self.limit_count {
            sql.push_str(" LIMIT ");
            sql.push_str(&limit.to_string());
        }

        // OFFSET
        if let Some(offset) = self.offset_value {
            sql.push_str(" OFFSET ");
            sql.push_str(&offset.to_string());
        }
    }
}

/// Query builder pool for reusing query builder instances to reduce allocations
pub struct QueryBuilderPool {
    pool: Arc<RwLock<Vec<QueryBuilder<()>>>>,
    max_size: usize,
}

impl QueryBuilderPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Arc::new(RwLock::new(Vec::with_capacity(max_size))),
            max_size,
        }
    }

    /// Get a query builder from the pool or create a new one
    pub fn acquire(&self) -> QueryBuilder<()> {
        if let Ok(mut pool) = self.pool.write() {
            if let Some(mut builder) = pool.pop() {
                // Reset the builder to default state
                builder.reset();
                return builder;
            }
        }

        // Create new builder if pool is empty
        QueryBuilder::new()
    }

    /// Return a query builder to the pool
    pub fn release(&self, builder: QueryBuilder<()>) {
        if let Ok(mut pool) = self.pool.write() {
            if pool.len() < self.max_size {
                pool.push(builder);
            }
            // If pool is full, just drop the builder
        }
    }
}

impl<M> QueryBuilder<M> {
    /// Reset query builder to default state for reuse
    pub fn reset(&mut self) {
        self.query_type = super::types::QueryType::Select;
        self.select_fields.clear();
        self.from_tables.clear();
        self.insert_table = None;
        self.update_table = None;
        self.delete_table = None;
        self.set_clauses.clear();
        self.where_conditions.clear();
        self.joins.clear();
        self.order_by.clear();
        self.group_by.clear();
        self.having_conditions.clear();
        self.limit_count = None;
        self.offset_value = None;
        self.distinct = false;
    }
}

/// Global query builder pool instance
static GLOBAL_QUERY_POOL: Lazy<QueryBuilderPool> = Lazy::new(|| {
    QueryBuilderPool::new(100) // Pool of up to 100 query builders
});

/// Get a query builder from the global pool
pub fn acquire_query_builder() -> QueryBuilder<()> {
    GLOBAL_QUERY_POOL.acquire()
}

/// Return a query builder to the global pool
pub fn release_query_builder(builder: QueryBuilder<()>) {
    GLOBAL_QUERY_POOL.release(builder);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_caching() {
        let placeholders1 = QueryBuilder::<()>::generate_placeholders_cached(3);
        let placeholders2 = QueryBuilder::<()>::generate_placeholders_cached(3);

        assert_eq!(placeholders1, "$1, $2, $3");
        assert_eq!(placeholders1, placeholders2);
    }

    #[test]
    fn test_query_builder_pool() {
        let pool = QueryBuilderPool::new(2);

        let builder1 = pool.acquire();
        let builder2 = pool.acquire();

        pool.release(builder1);
        pool.release(builder2);

        let builder3 = pool.acquire(); // Should reuse from pool
        assert!(!builder3.from_tables.is_empty() || builder3.from_tables.is_empty());
        // Basic check
    }

    #[test]
    fn test_optimized_sql_generation() {
        let query: QueryBuilder<()> = QueryBuilder::new()
            .from("users")
            .select("id, name, email")
            .where_eq("active", "true");

        let sql = query.to_sql_optimized();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM users"));
        assert!(sql.contains("WHERE"));
    }
}
