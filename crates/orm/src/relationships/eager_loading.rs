//! Eager Loading System - Prevents N+1 query problems with efficient relationship loading

use std::collections::HashMap;
use sqlx::{Pool, Postgres, Row, Column};

use crate::error::{ModelError, ModelResult};
use crate::model::Model;
use crate::query::QueryBuilder;
use super::constraints::{RelationshipConstraintBuilder};

/// Represents a relationship to be eagerly loaded
#[derive(Debug)]
pub struct EagerLoadSpec {
    /// Relationship name (e.g., "posts" or "posts.comments")  
    pub relation: String,
    /// Optional constraints for the relationship query
    pub constraints: Option<RelationshipConstraintBuilder>,
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
                    let spec = self.specs.remove(index);
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