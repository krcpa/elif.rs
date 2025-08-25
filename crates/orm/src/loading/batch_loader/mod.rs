use crate::{
    error::{OrmError, OrmResult},
    model::Model,
    query::QueryBuilder,
};
use serde_json::Value as JsonValue;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod config;
pub mod row_conversion;

pub use config::{BatchConfig, CacheStats};

/// Result of a batch load operation
#[derive(Debug)]
pub struct BatchLoadResult {
    /// Loaded records grouped by model type and ID
    pub records: HashMap<String, HashMap<Value, JsonValue>>,
    /// Number of queries executed
    pub query_count: usize,
    /// Total records loaded
    pub record_count: usize,
}

/// Batch loader for efficient relationship loading
#[derive(Clone)]
pub struct BatchLoader {
    config: BatchConfig,
    query_cache: Arc<RwLock<HashMap<String, Vec<JsonValue>>>>,
}

impl Default for BatchLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchLoader {
    /// Create a new batch loader with default configuration
    pub fn new() -> Self {
        Self::with_config(BatchConfig::default())
    }

    /// Create a new batch loader with custom configuration
    pub fn with_config(config: BatchConfig) -> Self {
        Self {
            config,
            query_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load multiple records in batches
    pub async fn load_batch<M: Model>(
        &self,
        ids: Vec<Value>,
        table: &str,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_results = Vec::new();
        let chunks: Vec<_> = ids.chunks(self.config.max_batch_size).collect();

        for chunk in chunks {
            let results = self.execute_batch_query(chunk, table, connection).await?;
            all_results.extend(results);
        }

        Ok(all_results)
    }

    /// Execute a single batch query
    async fn execute_batch_query(
        &self,
        ids: &[Value],
        table: &str,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        // Build batch query using ANY() for efficient IN clause
        let id_values: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect();

        let query = QueryBuilder::<()>::new()
            .from(table)
            .where_raw(&format!("id = ANY(ARRAY[{}])", id_values.join(", ")));

        let (sql, _params) = query.to_sql_with_params();
        let mut db_query = sqlx::query(&sql);

        // Bind all ID values
        for id in ids {
            db_query = match id {
                Value::Null => db_query.bind(None::<i32>),
                Value::Bool(b) => db_query.bind(b),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        db_query.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        db_query.bind(f)
                    } else {
                        return Err(OrmError::Query("Invalid number type".into()));
                    }
                }
                Value::String(s) => db_query.bind(s.as_str()),
                _ => return Err(OrmError::Query("Unsupported ID type".into())),
            };
        }

        let rows = db_query
            .fetch_all(connection)
            .await
            .map_err(|e| OrmError::Database(format!("Batch query failed: {}", e)))?;

        // Convert rows to JSON values
        let mut results = Vec::new();
        for row in rows {
            let json_row = self
                .row_to_json(&row)
                .map_err(|e| OrmError::Database(format!("Failed to convert row to JSON: {}", e)))?;
            results.push(json_row);
        }

        Ok(results)
    }

    /// Load relationships in batches with deduplication
    pub async fn load_relationships(
        &self,
        parent_type: &str,
        parent_ids: Vec<Value>,
        relationship_name: &str,
        foreign_key: &str,
        related_table: &str,
        connection: &sqlx::PgPool,
    ) -> OrmResult<HashMap<Value, Vec<JsonValue>>> {
        if parent_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Check cache if deduplication is enabled
        let cache_key = format!("{}:{}:{:?}", parent_type, relationship_name, parent_ids);

        if self.config.deduplicate_queries {
            let cache = self.query_cache.read().await;
            if let Some(cached_results) = cache.get(&cache_key) {
                return self.group_by_parent_id(cached_results.clone(), foreign_key, &parent_ids);
            }
        }

        // Use ANY() for efficient IN clause
        let parent_id_values: Vec<String> = parent_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect();

        // Execute batch query for relationships
        let query = QueryBuilder::<()>::new()
            .from(related_table)
            .where_raw(&format!(
                "{} = ANY(ARRAY[{}])",
                foreign_key,
                parent_id_values.join(", ")
            ));

        let (sql, _params) = query.to_sql_with_params();
        let mut db_query = sqlx::query(&sql);

        // Bind parent IDs
        for parent_id in &parent_ids {
            db_query = match parent_id {
                Value::Null => db_query.bind(None::<i32>),
                Value::Bool(b) => db_query.bind(b),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        db_query.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        db_query.bind(f)
                    } else {
                        return Err(OrmError::Query("Invalid number type".into()));
                    }
                }
                Value::String(s) => db_query.bind(s.as_str()),
                _ => return Err(OrmError::Query("Unsupported ID type".into())),
            };
        }

        let rows = db_query
            .fetch_all(connection)
            .await
            .map_err(|e| OrmError::Database(format!("Relationship batch query failed: {}", e)))?;

        // Convert to JSON values
        let mut results = Vec::new();
        for row in rows {
            let json_row = self
                .row_to_json(&row)
                .map_err(|e| OrmError::Database(format!("Failed to convert row to JSON: {}", e)))?;
            results.push(json_row);
        }

        // Cache results if deduplication is enabled
        if self.config.deduplicate_queries {
            let mut cache = self.query_cache.write().await;
            cache.insert(cache_key, results.clone());
        }

        // Group results by parent ID
        self.group_by_parent_id(results, foreign_key, &parent_ids)
    }

    /// Load nested relationships with deep optimization
    pub async fn load_nested_relationships(
        &self,
        _root_table: &str,
        root_ids: Vec<Value>,
        relationship_path: &str,
        connection: &sqlx::PgPool,
    ) -> OrmResult<HashMap<Value, JsonValue>> {
        if root_ids.is_empty() || relationship_path.is_empty() {
            return Ok(HashMap::new());
        }

        // Parse relationship path (e.g., "posts.comments.user")
        let relations: Vec<&str> = relationship_path.split('.').collect();
        let mut current_ids = root_ids.clone();
        let mut results: HashMap<Value, JsonValue> = HashMap::new();

        // Process each level of nesting
        for (depth, relation) in relations.iter().enumerate() {
            if current_ids.is_empty() {
                break;
            }

            // Determine table and foreign key based on relationship type
            let (related_table, foreign_key) = self.get_relationship_mapping(relation)?;

            // Load current level relationships in optimized batches
            let level_results = self
                .load_relationships_optimized(
                    &format!("level_{}", depth),
                    current_ids,
                    relation,
                    &foreign_key,
                    &related_table,
                    connection,
                )
                .await?;

            // Update current IDs for next level
            current_ids = level_results
                .values()
                .flatten()
                .filter_map(|record| record.get("id").cloned())
                .collect();

            // Merge results with proper nesting
            self.merge_nested_results(&mut results, level_results, depth == 0);
        }

        Ok(results)
    }

    /// Load relationships with advanced optimization strategies
    async fn load_relationships_optimized(
        &self,
        parent_type: &str,
        parent_ids: Vec<Value>,
        relationship_name: &str,
        foreign_key: &str,
        related_table: &str,
        connection: &sqlx::PgPool,
    ) -> OrmResult<HashMap<Value, Vec<JsonValue>>> {
        // Use smaller batch sizes for nested queries to avoid memory issues
        let optimal_batch_size = std::cmp::min(self.config.max_batch_size, 50);
        let mut all_results: HashMap<Value, Vec<JsonValue>> = HashMap::new();

        // Process in optimized chunks
        for chunk in parent_ids.chunks(optimal_batch_size) {
            let chunk_results = self
                .load_relationships(
                    parent_type,
                    chunk.to_vec(),
                    relationship_name,
                    foreign_key,
                    related_table,
                    connection,
                )
                .await?;

            // Merge chunk results
            for (parent_id, relations) in chunk_results {
                all_results.entry(parent_id).or_default().extend(relations);
            }
        }

        Ok(all_results)
    }

    /// Get relationship mapping for a relation name
    fn get_relationship_mapping(&self, relation: &str) -> OrmResult<(String, String)> {
        // This would normally use relationship metadata
        // For now, we'll use convention-based mapping
        match relation {
            "posts" => Ok(("posts".to_string(), "user_id".to_string())),
            "comments" => Ok(("comments".to_string(), "post_id".to_string())),
            "user" => Ok(("users".to_string(), "user_id".to_string())),
            "profile" => Ok(("profiles".to_string(), "user_id".to_string())),
            _ => Ok((format!("{}s", relation), format!("{}_id", relation))),
        }
    }

    /// Merge nested results with proper hierarchical structure
    fn merge_nested_results(
        &self,
        target: &mut HashMap<Value, JsonValue>,
        source: HashMap<Value, Vec<JsonValue>>,
        is_root: bool,
    ) {
        for (parent_id, relations) in source {
            if is_root {
                // For root level, create the initial structure
                let parent_id_copy = parent_id.clone();
                target.insert(
                    parent_id,
                    serde_json::json!({
                        "id": parent_id_copy,
                        "relations": relations
                    }),
                );
            } else {
                // For nested levels, update existing structure
                if let Some(existing) = target.get_mut(&parent_id) {
                    if let Some(obj) = existing.as_object_mut() {
                        obj.insert("nested_relations".to_string(), serde_json::json!(relations));
                    }
                }
            }
        }
    }

    /// Group results by parent ID
    fn group_by_parent_id(
        &self,
        results: Vec<JsonValue>,
        foreign_key: &str,
        parent_ids: &[Value],
    ) -> OrmResult<HashMap<Value, Vec<JsonValue>>> {
        let mut grouped: HashMap<Value, Vec<JsonValue>> = HashMap::new();

        // Initialize with empty vecs for all parent IDs
        for parent_id in parent_ids {
            grouped.insert(parent_id.clone(), Vec::new());
        }

        // Group results by foreign key value
        for result in results {
            if let Some(fk_value) = result.get(foreign_key) {
                let parent_id = serde_json::from_value(fk_value.clone()).unwrap_or(Value::Null);

                grouped.entry(parent_id).or_default().push(result);
            }
        }

        Ok(grouped)
    }

    /// Clear the query cache
    pub async fn clear_cache(&self) {
        let mut cache = self.query_cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.query_cache.read().await;
        CacheStats {
            cached_queries: cache.len(),
            total_cached_records: cache.values().map(|v| v.len()).sum(),
        }
    }
}

#[cfg(test)]
mod tests;
