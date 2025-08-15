//! Query Builder performance and optimization methods

use serde_json::Value;
use super::builder::QueryBuilder;

impl<M> QueryBuilder<M> {
    /// Get parameter bindings (for prepared statements)
    /// Enhanced to support subqueries and complex conditions
    pub fn bindings(&self) -> Vec<Value> {
        let mut bindings = Vec::new();
        
        for condition in &self.where_conditions {
            // Skip RAW, EXISTS, NOT EXISTS conditions from parameter binding
            if matches!(condition.column.as_str(), "RAW" | "EXISTS" | "NOT EXISTS") {
                continue;
            }
            
            if let Some(value) = &condition.value {
                // Skip subquery values (they're already formatted)
                if let Value::String(val_str) = value {
                    if !val_str.starts_with('(') || !val_str.ends_with(')') {
                        bindings.push(value.clone());
                    }
                } else {
                    bindings.push(value.clone());
                }
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
    
    /// Clone this query builder for use in subqueries
    pub fn clone_for_subquery(&self) -> Self {
        self.clone()
    }
    
    /// Optimize query by analyzing conditions
    pub fn optimize(self) -> Self {
        // TODO: Implement query optimization strategies
        // - Remove redundant conditions
        // - Optimize join order
        // - Suggest index usage
        self
    }
    
    /// Get query complexity score for performance monitoring
    pub fn complexity_score(&self) -> u32 {
        let mut score = 0;
        
        score += self.where_conditions.len() as u32;
        score += self.joins.len() as u32 * 2; // Joins are more expensive
        score += self.group_by.len() as u32;
        score += self.having_conditions.len() as u32;
        
        if self.distinct {
            score += 1;
        }
        
        score
    }
}