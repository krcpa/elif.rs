//! Query Builder - Core builder implementation

use std::marker::PhantomData;

use super::types::*;

/// Query builder for constructing database queries
#[derive(Debug)]
pub struct QueryBuilder<M = ()> {
    pub(crate) query_type: QueryType,
    pub(crate) select_fields: Vec<String>,
    pub(crate) from_tables: Vec<String>,
    pub(crate) insert_table: Option<String>,
    pub(crate) update_table: Option<String>, 
    pub(crate) delete_table: Option<String>,
    pub(crate) set_clauses: Vec<SetClause>,
    pub(crate) where_conditions: Vec<WhereCondition>,
    pub(crate) joins: Vec<JoinClause>,
    pub(crate) order_by: Vec<(String, OrderDirection)>,
    pub(crate) group_by: Vec<String>,
    pub(crate) having_conditions: Vec<WhereCondition>,
    pub(crate) limit_count: Option<i64>,
    pub(crate) offset_value: Option<i64>,
    pub(crate) distinct: bool,
    _phantom: PhantomData<M>,
}

impl<M> Clone for QueryBuilder<M> {
    fn clone(&self) -> Self {
        Self {
            query_type: self.query_type.clone(),
            select_fields: self.select_fields.clone(),
            from_tables: self.from_tables.clone(),
            insert_table: self.insert_table.clone(),
            update_table: self.update_table.clone(),
            delete_table: self.delete_table.clone(),
            set_clauses: self.set_clauses.clone(),
            where_conditions: self.where_conditions.clone(),
            joins: self.joins.clone(),
            order_by: self.order_by.clone(),
            group_by: self.group_by.clone(),
            having_conditions: self.having_conditions.clone(),
            limit_count: self.limit_count,
            offset_value: self.offset_value,
            distinct: self.distinct,
            _phantom: PhantomData,
        }
    }
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
            query_type: QueryType::Select,
            select_fields: Vec::new(),
            from_tables: Vec::new(),
            insert_table: None,
            update_table: None,
            delete_table: None,
            set_clauses: Vec::new(),
            where_conditions: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having_conditions: Vec::new(),
            limit_count: None,
            offset_value: None,
            distinct: false,
            _phantom: PhantomData,
        }
    }
}