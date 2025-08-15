//! Query Builder pagination operations

use serde_json::Value;
use super::builder::QueryBuilder;
use super::types::*;

impl<M> QueryBuilder<M> {
    /// Add LIMIT clause
    pub fn limit(mut self, count: i64) -> Self {
        self.limit_count = Some(count);
        self
    }

    /// Add OFFSET clause
    pub fn offset(mut self, count: i64) -> Self {
        self.offset_value = Some(count);
        self
    }

    /// Add pagination (LIMIT + OFFSET)
    pub fn paginate(mut self, per_page: i64, page: i64) -> Self {
        self.limit_count = Some(per_page);
        self.offset_value = Some((page - 1) * per_page);
        self
    }
    
    /// Cursor-based pagination (for better performance on large datasets)
    pub fn paginate_cursor<T: Into<Value>>(mut self, cursor_column: &str, cursor_value: Option<T>, per_page: i64, direction: OrderDirection) -> Self {
        self.limit_count = Some(per_page);
        
        if let Some(cursor_val) = cursor_value {
            match direction {
                OrderDirection::Asc => {
                    self = self.where_gt(cursor_column, cursor_val);
                }
                OrderDirection::Desc => {
                    self = self.where_lt(cursor_column, cursor_val);
                }
            }
        }
        
        self.order_by.push((cursor_column.to_string(), direction));
        
        self
    }
}