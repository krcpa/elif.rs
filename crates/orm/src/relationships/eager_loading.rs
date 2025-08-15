//! Eager Loading System - Prevents N+1 query problems with efficient relationship loading

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use async_trait::async_trait;
use sqlx::{Pool, Postgres, Row, Column};

use crate::error::{ModelError, ModelResult};
use crate::model::Model;
use crate::query::{QueryBuilder, OrderDirection, QueryOperator};

/// Represents a relationship to be eagerly loaded
#[derive(Debug)]
pub struct EagerLoadSpec {
    /// Relationship name (e.g., "posts" or "posts.comments")  
    pub relation: String,
    /// Optional constraints for the relationship query
    pub constraints: Option<RelationshipConstraintBuilder>,
}

/// Constraint types for relationship queries
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    Where,
    Order,
    Limit,
    Offset,
    Having,
    GroupBy,
    Join,
    Raw,
}

/// Trait for applying constraints to relationship queries
#[async_trait]
pub trait RelationshipConstraint: Send + Sync + std::fmt::Debug {
    /// Apply constraint to the query builder
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()>;
    
    /// Get the type of constraint
    fn constraint_type(&self) -> ConstraintType;
    
    /// Get a description of the constraint for debugging
    fn description(&self) -> String;
    
    /// Validate the constraint before applying
    fn validate(&self) -> ModelResult<()> {
        Ok(()) // Default implementation - constraints can override
    }
}

/// Builder for relationship constraints with type safety and validation
#[derive(Debug)]
pub struct RelationshipConstraintBuilder {
    constraints: Vec<Box<dyn RelationshipConstraint>>,
    /// Track constraint types to prevent conflicts
    applied_types: std::collections::HashSet<ConstraintType>,
}

impl RelationshipConstraintBuilder {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            applied_types: std::collections::HashSet::new(),
        }
    }
    
    /// Apply all constraints to the query builder
    pub async fn apply_all(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        for constraint in &self.constraints {
            constraint.validate()?;
            constraint.apply(query).await?;
        }
        Ok(())
    }
    
    /// Get all constraints
    pub fn constraints(&self) -> &[Box<dyn RelationshipConstraint>] {
        &self.constraints
    }
    
    /// Check if a constraint type has been applied
    pub fn has_constraint_type(&self, constraint_type: &ConstraintType) -> bool {
        self.applied_types.contains(constraint_type)
    }
    
    /// Add a constraint and track its type
    fn add_constraint(&mut self, constraint: Box<dyn RelationshipConstraint>) {
        let constraint_type = constraint.constraint_type();
        self.applied_types.insert(constraint_type);
        self.constraints.push(constraint);
    }

    /// Add WHERE equals constraint
    pub fn where_eq<V>(mut self, field: &str, value: V) -> Self 
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::Equal,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add WHERE not equals constraint
    pub fn where_ne<V>(mut self, field: &str, value: V) -> Self 
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::NotEqual,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add WHERE greater than constraint
    pub fn where_gt<V>(mut self, field: &str, value: V) -> Self 
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::GreaterThan,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add WHERE greater than or equal constraint
    pub fn where_gte<V>(mut self, field: &str, value: V) -> Self 
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::GreaterThanOrEqual,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add WHERE less than constraint
    pub fn where_lt<V>(mut self, field: &str, value: V) -> Self 
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::LessThan,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add WHERE less than or equal constraint
    pub fn where_lte<V>(mut self, field: &str, value: V) -> Self 
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::LessThanOrEqual,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add WHERE LIKE constraint
    pub fn where_like(mut self, field: &str, pattern: &str) -> Self {
        let constraint = WhereConstraint {
            field: field.to_string(),
            operator: QueryOperator::Like,
            value: serde_json::Value::String(pattern.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add WHERE IN constraint
    pub fn where_in<V>(mut self, field: &str, values: Vec<V>) -> Self 
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = WhereInConstraint {
            field: field.to_string(),
            values: values.into_iter().map(|v| serde_json::Value::String(v.to_string())).collect(),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add raw WHERE constraint
    pub fn where_raw(mut self, condition: &str) -> Self {
        let constraint = RawConstraint {
            sql: condition.to_string(),
            constraint_type: ConstraintType::Where,
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add ORDER BY constraint
    pub fn order_by(mut self, field: &str) -> Self {
        let constraint = OrderConstraint {
            field: field.to_string(),
            direction: OrderDirection::Asc,
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add ORDER BY DESC constraint
    pub fn order_by_desc(mut self, field: &str) -> Self {
        let constraint = OrderConstraint {
            field: field.to_string(),
            direction: OrderDirection::Desc,
        };
        self.add_constraint(Box::new(constraint));
        self
    }

    /// Add LIMIT constraint
    pub fn limit(mut self, count: i64) -> Self {
        let constraint = LimitConstraint { count };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add OFFSET constraint
    pub fn offset(mut self, count: i64) -> Self {
        let constraint = OffsetConstraint { count };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add GROUP BY constraint
    pub fn group_by(mut self, field: &str) -> Self {
        let constraint = GroupByConstraint {
            field: field.to_string(),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add HAVING constraint
    pub fn having<V>(mut self, field: &str, operator: QueryOperator, value: V) -> Self 
    where
        V: Send + Sync + std::fmt::Display + Clone + 'static,
    {
        let constraint = HavingConstraint {
            field: field.to_string(),
            operator,
            value: serde_json::Value::String(value.to_string()),
        };
        self.add_constraint(Box::new(constraint));
        self
    }
    
    /// Add raw HAVING constraint
    pub fn having_raw(mut self, condition: &str) -> Self {
        let constraint = RawConstraint {
            sql: condition.to_string(),
            constraint_type: ConstraintType::Having,
        };
        self.add_constraint(Box::new(constraint));
        self
    }
}

/// WHERE constraint implementation
#[derive(Debug, Clone)]
struct WhereConstraint {
    field: String,
    operator: QueryOperator,
    value: serde_json::Value,
}

#[async_trait]
impl RelationshipConstraint for WhereConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply the WHERE condition using the appropriate method based on operator
        *query = match self.operator {
            QueryOperator::Equal => query.clone().where_eq(&self.field, self.value.clone()),
            QueryOperator::NotEqual => query.clone().where_ne(&self.field, self.value.clone()),
            QueryOperator::GreaterThan => query.clone().where_gt(&self.field, self.value.clone()),
            QueryOperator::GreaterThanOrEqual => query.clone().where_gte(&self.field, self.value.clone()),
            QueryOperator::LessThan => query.clone().where_lt(&self.field, self.value.clone()),
            QueryOperator::LessThanOrEqual => query.clone().where_lte(&self.field, self.value.clone()),
            QueryOperator::Like => {
                if let Some(pattern) = self.value.as_str() {
                    query.clone().where_like(&self.field, pattern)
                } else {
                    return Err(ModelError::Validation("LIKE operator requires string value".to_string()));
                }
            },
            QueryOperator::NotLike => {
                if let Some(pattern) = self.value.as_str() {
                    query.clone().where_not_like(&self.field, pattern)
                } else {
                    return Err(ModelError::Validation("NOT LIKE operator requires string value".to_string()));
                }
            },
            _ => {
                return Err(ModelError::Validation(format!(
                    "Unsupported operator {:?} for WHERE constraint", self.operator
                )));
            }
        };
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Where
    }
    
    fn description(&self) -> String {
        format!("WHERE {} {:?} {}", self.field, self.operator, self.value)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("WHERE constraint field cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// ORDER BY constraint implementation  
#[derive(Debug, Clone)]
struct OrderConstraint {
    field: String,
    direction: OrderDirection,
}

#[async_trait]
impl RelationshipConstraint for OrderConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply the ORDER BY condition to the query builder
        *query = match self.direction {
            OrderDirection::Desc => query.clone().order_by_desc(&self.field),
            OrderDirection::Asc => query.clone().order_by(&self.field),
        };
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Order
    }
    
    fn description(&self) -> String {
        format!("ORDER BY {} {:?}", self.field, self.direction)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("ORDER BY constraint field cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// LIMIT constraint implementation
#[derive(Debug, Clone)]
struct LimitConstraint {
    count: i64,
}

#[async_trait] 
impl RelationshipConstraint for LimitConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply the LIMIT condition to the query builder
        *query = query.clone().limit(self.count);
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Limit
    }
    
    fn description(&self) -> String {
        format!("LIMIT {}", self.count)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.count < 0 {
            return Err(ModelError::Validation("LIMIT count must be non-negative".to_string()));
        }
        Ok(())
    }
}

/// OFFSET constraint implementation
#[derive(Debug, Clone)]
struct OffsetConstraint {
    count: i64,
}

#[async_trait]
impl RelationshipConstraint for OffsetConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        *query = query.clone().offset(self.count);
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Offset
    }
    
    fn description(&self) -> String {
        format!("OFFSET {}", self.count)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.count < 0 {
            return Err(ModelError::Validation("OFFSET count must be non-negative".to_string()));
        }
        Ok(())
    }
}

/// WHERE IN constraint implementation
#[derive(Debug, Clone)]
struct WhereInConstraint {
    field: String,
    values: Vec<serde_json::Value>,
}

#[async_trait]
impl RelationshipConstraint for WhereInConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Convert values to strings for the where_in method
        let string_values: Vec<String> = self.values
            .iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                _ => v.to_string(),
            })
            .collect();
        
        *query = query.clone().where_in(&self.field, string_values);
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Where
    }
    
    fn description(&self) -> String {
        format!("WHERE {} IN ({} values)", self.field, self.values.len())
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("WHERE IN constraint field cannot be empty".to_string()));
        }
        if self.values.is_empty() {
            return Err(ModelError::Validation("WHERE IN constraint must have at least one value".to_string()));
        }
        Ok(())
    }
}

/// GROUP BY constraint implementation
#[derive(Debug, Clone)]
struct GroupByConstraint {
    field: String,
}

#[async_trait]
impl RelationshipConstraint for GroupByConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        *query = query.clone().group_by(&self.field);
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::GroupBy
    }
    
    fn description(&self) -> String {
        format!("GROUP BY {}", self.field)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("GROUP BY constraint field cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// HAVING constraint implementation
#[derive(Debug, Clone)]
struct HavingConstraint {
    field: String,
    operator: QueryOperator,
    value: serde_json::Value,
}

#[async_trait]
impl RelationshipConstraint for HavingConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply HAVING constraint using the having method
        *query = query.clone().having(&self.field, self.operator, self.value.clone());
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        ConstraintType::Having
    }
    
    fn description(&self) -> String {
        format!("HAVING {} {:?} {}", self.field, self.operator, self.value)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.field.trim().is_empty() {
            return Err(ModelError::Validation("HAVING constraint field cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// Raw SQL constraint implementation for complex cases
#[derive(Debug, Clone)]
struct RawConstraint {
    sql: String,
    constraint_type: ConstraintType,
}

#[async_trait]
impl RelationshipConstraint for RawConstraint {
    async fn apply(&self, query: &mut QueryBuilder) -> ModelResult<()> {
        // Apply raw constraint based on its type
        match self.constraint_type {
            ConstraintType::Where => {
                *query = query.clone().where_raw(&self.sql);
            },
            ConstraintType::Having => {
                *query = query.clone().having_raw(&self.sql);
            },
            ConstraintType::Raw => {
                // For generic raw constraints, we'd need a way to append raw SQL
                // This would require extending the QueryBuilder with a raw method
                return Err(ModelError::Validation("Raw constraints not yet supported".to_string()));
            },
            _ => {
                return Err(ModelError::Validation(format!(
                    "Raw constraints not supported for type {:?}", self.constraint_type
                )));
            }
        }
        Ok(())
    }
    
    fn constraint_type(&self) -> ConstraintType {
        self.constraint_type.clone()
    }
    
    fn description(&self) -> String {
        format!("RAW {:?}: {}", self.constraint_type, self.sql)
    }
    
    fn validate(&self) -> ModelResult<()> {
        if self.sql.trim().is_empty() {
            return Err(ModelError::Validation("Raw constraint SQL cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// Core eager loader that manages relationship loading
pub struct EagerLoader {
    /// Relationships to load
    specs: Vec<EagerLoadSpec>,
    /// Loaded relationship data organized by relationship name and parent key
    loaded_data: HashMap<String, HashMap<String, Vec<serde_json::Value>>>,
}

impl EagerLoader {
    /// Create a new eager loader
    pub fn new() -> Self {
        Self {
            specs: Vec::new(),
            loaded_data: HashMap::new(),
        }
    }

    /// Add a relationship to eagerly load
    pub fn with(mut self, relation: &str) -> Self {
        self.specs.push(EagerLoadSpec {
            relation: relation.to_string(),
            constraints: None,
        });
        self
    }

    /// Add a relationship with constraints
    pub fn with_constraint<F>(mut self, relation: &str, constraint_fn: F) -> Self
    where
        F: FnOnce(RelationshipConstraintBuilder) -> RelationshipConstraintBuilder + 'static,
    {
        // Build the constraint and store it
        let builder = RelationshipConstraintBuilder::new();
        let built_constraints = constraint_fn(builder);
        
        self.specs.push(EagerLoadSpec {
            relation: relation.to_string(),
            constraints: Some(built_constraints),
        });
        self
    }

    /// Load relationships for a collection of models
    pub async fn load_for_models<M>(&mut self, pool: &Pool<Postgres>, models: &[M]) -> ModelResult<()>
    where
        M: Model + Send + Sync,
    {
        if models.is_empty() {
            return Ok(());
        }

        // Load each relationship specification
        let relations: Vec<String> = self.specs.iter().map(|s| s.relation.clone()).collect();
        for relation in relations {
            self.load_relationship(pool, models, &relation).await?;
        }

        Ok(())
    }

    /// Load a specific relationship for the given models
    async fn load_relationship<M>(&mut self, pool: &Pool<Postgres>, models: &[M], relation: &str) -> ModelResult<()>
    where
        M: Model + Send + Sync,
    {
        // Parse nested relationships (e.g., "posts.comments.user")
        let parts: Vec<&str> = relation.split('.').collect();
        
        if parts.len() == 1 {
            // Simple relationship - find constraints for this relation
            let spec_index = self.specs
                .iter()
                .position(|spec| spec.relation == relation);
                
            if let Some(index) = spec_index {
                let has_constraints = self.specs[index].constraints.is_some();
                if has_constraints {
                    // We need to work around the borrow checker by taking ownership temporarily
                    let mut spec = self.specs.remove(index);
                    let constraints = spec.constraints.as_ref();
                    let result = self.load_simple_relationship(pool, models, relation, constraints).await;
                    self.specs.insert(index, spec);
                    result?;
                } else {
                    self.load_simple_relationship(pool, models, relation, None).await?;
                }
            } else {
                self.load_simple_relationship(pool, models, relation, None).await?;
            }
        } else {
            // Nested relationship - load step by step
            self.load_nested_relationship(pool, models, &parts).await?;
        }

        Ok(())
    }

    /// Load a simple (non-nested) relationship
    async fn load_simple_relationship<M>(&mut self, pool: &Pool<Postgres>, models: &[M], relation: &str, constraints: Option<&RelationshipConstraintBuilder>) -> ModelResult<()>
    where
        M: Model + Send + Sync,
    {
        // Collect parent keys
        let parent_keys: Vec<String> = models
            .iter()
            .filter_map(|m| m.primary_key().map(|pk| pk.to_string()))
            .collect();

        if parent_keys.is_empty() {
            return Ok(());
        }

        // Build the relationship query with constraints
        let query = self.build_relationship_query(relation, &parent_keys, constraints).await?;
        
        // Execute the query
        let rows = sqlx::query(&query).fetch_all(pool).await
            .map_err(|e| ModelError::Database(e.to_string()))?;

        // Group results by parent key
        let mut grouped_results: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        
        for row in rows {
            // Extract parent key and convert row to JSON
            // This assumes foreign key is always "parent_id" - needs to be dynamic
            let parent_key: String = row.try_get("parent_id")
                .map_err(|e| ModelError::Database(e.to_string()))?;
            
            let json_value = self.row_to_json(&row)?;
            
            grouped_results
                .entry(parent_key)
                .or_insert_with(Vec::new)
                .push(json_value);
        }

        // Store the loaded data
        self.loaded_data.insert(relation.to_string(), grouped_results);

        Ok(())
    }

    /// Load nested relationships (e.g., "posts.comments.user")
    async fn load_nested_relationship<M>(&mut self, pool: &Pool<Postgres>, models: &[M], parts: &[&str]) -> ModelResult<()>
    where
        M: Model + Send + Sync,
    {
        // Start with the root models
        let mut current_models: Vec<serde_json::Value> = Vec::new();
        
        // Load the first level relationship  
        self.load_simple_relationship(pool, models, parts[0], None).await?;
        
        // Get the loaded first level data
        if let Some(first_level_data) = self.loaded_data.get(parts[0]) {
            for values in first_level_data.values() {
                current_models.extend(values.iter().cloned());
            }
        }

        // Load subsequent levels
        for i in 1..parts.len() {
            let relation_path = parts[0..=i].join(".");
            let current_relation = parts[i];
            
            // Extract parent keys from current models
            let parent_keys: Vec<String> = current_models
                .iter()
                .filter_map(|v| v.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
                .collect();

            if parent_keys.is_empty() {
                continue;
            }

            // Build and execute query for this level  
            let query = self.build_relationship_query(current_relation, &parent_keys, None).await?;
            let rows = sqlx::query(&query).fetch_all(pool).await
                .map_err(|e| ModelError::Database(e.to_string()))?;

            // Group results and store
            let mut grouped_results: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
            let mut next_level_models = Vec::new();
            
            for row in rows {
                let parent_key: String = row.try_get("parent_id")
                    .map_err(|e| ModelError::Database(e.to_string()))?;
                
                let json_value = self.row_to_json(&row)?;
                next_level_models.push(json_value.clone());
                
                grouped_results
                    .entry(parent_key)
                    .or_insert_with(Vec::new)
                    .push(json_value);
            }

            self.loaded_data.insert(relation_path, grouped_results);
            current_models = next_level_models;
        }

        Ok(())
    }

    /// Build SQL query for a relationship with constraints
    async fn build_relationship_query(&self, relation: &str, parent_keys: &[String], constraints: Option<&RelationshipConstraintBuilder>) -> ModelResult<String> {
        // Build base query using QueryBuilder  
        let mut query = QueryBuilder::<()>::new();
        
        // Determine table name and foreign key from relation name
        // This is a basic implementation - needs proper metadata
        let table_name = match relation {
            "posts" => "posts",
            "comments" => "comments", 
            "user" => "users",
            "profile" => "profiles",
            _ => relation, // fallback
        };

        let foreign_key = match relation {
            "posts" => "user_id",
            "comments" => "post_id",
            "user" => "user_id", 
            "profile" => "user_id",
            _ => "parent_id", // fallback
        };
        
        // Build base query
        query = query
            .select("*")
            .from(table_name)
            .where_in(foreign_key, parent_keys.to_vec());
            
        // Apply constraints if present
        if let Some(constraint_builder) = constraints {
            constraint_builder.apply_all(&mut query).await?;
        }
        
        // Generate SQL
        Ok(query.to_sql())
    }

    /// Convert a database row to JSON value
    fn row_to_json(&self, row: &sqlx::postgres::PgRow) -> ModelResult<serde_json::Value> {
        let mut map = serde_json::Map::new();
        
        // This is a simplified conversion
        // In practice, we'd need to handle all column types properly
        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name();
            
            // Try to get the value as different types
            if let Ok(value) = row.try_get::<Option<String>, _>(i) {
                map.insert(column_name.to_string(), serde_json::Value::String(value.unwrap_or_default()));
            } else if let Ok(value) = row.try_get::<Option<i64>, _>(i) {
                if let Some(val) = value {
                    map.insert(column_name.to_string(), serde_json::Value::Number(serde_json::Number::from(val)));
                } else {
                    map.insert(column_name.to_string(), serde_json::Value::Null);
                }
            }
            // Add more type conversions as needed
        }

        Ok(serde_json::Value::Object(map))
    }

    /// Get loaded relationship data for a parent key
    pub fn get_loaded_data(&self, relation: &str, parent_key: &str) -> Option<&Vec<serde_json::Value>> {
        self.loaded_data
            .get(relation)?
            .get(parent_key)
    }

    /// Check if a relationship has been loaded
    pub fn is_loaded(&self, relation: &str) -> bool {
        self.loaded_data.contains_key(relation)
    }

    /// Get all loaded relationships
    pub fn loaded_relations(&self) -> Vec<&String> {
        self.loaded_data.keys().collect()
    }
}

impl Default for EagerLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for QueryBuilder to add eager loading support
pub trait QueryBuilderEagerLoading<M> {
    /// Add a relationship to eagerly load
    fn with(self, relation: &str) -> Self;
    
    /// Add a relationship with constraints
    fn with_where<F>(self, relation: &str, constraint: F) -> Self
    where
        F: FnOnce(RelationshipConstraintBuilder) -> RelationshipConstraintBuilder;
    
    /// Add conditional eager loading
    fn with_when(self, condition: bool, relation: &str) -> Self;
    
    /// Load relationship counts without loading the relationships
    fn with_count(self, relation: &str) -> Self;
    
    /// Load relationship counts with constraints
    fn with_count_where<F>(self, alias: &str, relation: &str, constraint: F) -> Self
    where
        F: FnOnce(RelationshipConstraintBuilder) -> RelationshipConstraintBuilder;
}

// Implementation will be added in the query builder integration