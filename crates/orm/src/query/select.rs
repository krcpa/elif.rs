//! Query Builder SELECT operations

use super::builder::QueryBuilder;

impl<M> QueryBuilder<M> {
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
        self.from_tables = vec![table.to_string()];
        self
    }

    /// Add aggregate functions to SELECT
    pub fn select_count(mut self, column: &str, alias: Option<&str>) -> Self {
        let select_expr = if let Some(alias) = alias {
            format!("COUNT({}) AS {}", column, alias)
        } else {
            format!("COUNT({})", column)
        };
        self.select_fields.push(select_expr);
        self
    }
    
    /// Add SUM aggregate
    pub fn select_sum(mut self, column: &str, alias: Option<&str>) -> Self {
        let select_expr = if let Some(alias) = alias {
            format!("SUM({}) AS {}", column, alias)
        } else {
            format!("SUM({})", column)
        };
        self.select_fields.push(select_expr);
        self
    }
    
    /// Add AVG aggregate
    pub fn select_avg(mut self, column: &str, alias: Option<&str>) -> Self {
        let select_expr = if let Some(alias) = alias {
            format!("AVG({}) AS {}", column, alias)
        } else {
            format!("AVG({})", column)
        };
        self.select_fields.push(select_expr);
        self
    }
    
    /// Add MIN aggregate
    pub fn select_min(mut self, column: &str, alias: Option<&str>) -> Self {
        let select_expr = if let Some(alias) = alias {
            format!("MIN({}) AS {}", column, alias)
        } else {
            format!("MIN({})", column)
        };
        self.select_fields.push(select_expr);
        self
    }
    
    /// Add MAX aggregate
    pub fn select_max(mut self, column: &str, alias: Option<&str>) -> Self {
        let select_expr = if let Some(alias) = alias {
            format!("MAX({}) AS {}", column, alias)
        } else {
            format!("MAX({})", column)
        };
        self.select_fields.push(select_expr);
        self
    }
    
    /// Add custom SELECT expression
    pub fn select_raw(mut self, expression: &str) -> Self {
        self.select_fields.push(expression.to_string());
        self
    }
}