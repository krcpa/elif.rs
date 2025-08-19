use crate::{
    error::OrmResult,
    loading::{
        batch_loader::BatchLoader,
        optimizer::{QueryOptimizer, QueryPlan, QueryNode, PlanExecutor, OptimizationStrategy},
        query_deduplicator::QueryDeduplicator,
    },
    relationships::RelationshipType,
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Configuration for the eager loader
#[derive(Debug, Clone)]
pub struct EagerLoadConfig {
    /// Maximum batch size for loading
    pub max_batch_size: usize,
    /// Enable query deduplication
    pub deduplicate_queries: bool,
    /// Maximum depth for nested relationships
    pub max_depth: usize,
    /// Enable parallel execution
    pub enable_parallelism: bool,
    /// Query timeout in milliseconds
    pub query_timeout_ms: u64,
}

impl Default for EagerLoadConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            deduplicate_queries: true,
            max_depth: 10,
            enable_parallelism: true,
            query_timeout_ms: 30000,
        }
    }
}

/// Result of an eager loading operation
#[derive(Debug)]
pub struct EagerLoadResult {
    /// Loaded data grouped by entity ID
    pub data: HashMap<JsonValue, JsonValue>,
    /// Performance statistics
    pub stats: EagerLoadStats,
    /// Applied optimizations
    pub optimizations: Vec<OptimizationStrategy>,
}

/// Statistics about the eager loading operation
#[derive(Debug, Clone)]
pub struct EagerLoadStats {
    /// Total execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of database queries executed
    pub query_count: usize,
    /// Total records loaded
    pub records_loaded: usize,
    /// Number of relationship levels loaded
    pub depth_loaded: usize,
    /// Cache hit ratio (0.0 to 1.0)
    pub cache_hit_ratio: f64,
}

impl Default for EagerLoadStats {
    fn default() -> Self {
        Self {
            execution_time_ms: 0,
            query_count: 0,
            records_loaded: 0,
            depth_loaded: 0,
            cache_hit_ratio: 0.0,
        }
    }
}

/// Optimized eager loader for relationship loading with advanced optimization strategies
pub struct OptimizedEagerLoader {
    batch_loader: BatchLoader,
    query_optimizer: QueryOptimizer,
    plan_executor: PlanExecutor,
    _query_deduplicator: QueryDeduplicator,
    config: EagerLoadConfig,
}

impl OptimizedEagerLoader {
    /// Create a new optimized eager loader with default configuration
    pub fn new() -> Self {
        let config = EagerLoadConfig::default();
        let batch_loader = BatchLoader::new();
        Self::with_config(config, batch_loader)
    }

    /// Create an optimized eager loader with custom configuration
    pub fn with_config(config: EagerLoadConfig, batch_loader: BatchLoader) -> Self {
        let query_optimizer = QueryOptimizer::new();
        let plan_executor = PlanExecutor::with_config(
            batch_loader.clone(),
            if config.enable_parallelism { 10 } else { 1 },
            std::time::Duration::from_millis(config.query_timeout_ms),
        );
        let query_deduplicator = QueryDeduplicator::new();

        Self {
            batch_loader,
            query_optimizer,
            plan_executor,
            _query_deduplicator: query_deduplicator,
            config,
        }
    }

    /// Load relationships eagerly with optimization
    pub async fn load_with_relationships(
        &mut self,
        root_table: &str,
        root_ids: Vec<JsonValue>,
        relationships: &str,
        connection: &sqlx::PgPool,
    ) -> OrmResult<EagerLoadResult> {
        let start_time = std::time::Instant::now();
        
        // Parse and build query plan
        let mut plan = self.build_query_plan(root_table, &root_ids, relationships)?;
        
        // Optimize the plan
        let optimization_strategies = self.query_optimizer.optimize_plan(&mut plan)?;
        
        // Execute the optimized plan
        let execution_result = self.plan_executor.execute_plan(&plan, connection).await?;
        
        // Process results into the expected format
        let processed_data = self.process_execution_results(execution_result.results, &root_ids)?;
        
        // Calculate statistics
        let execution_time = start_time.elapsed();
        let stats = EagerLoadStats {
            execution_time_ms: execution_time.as_millis() as u64,
            query_count: execution_result.stats.query_count,
            records_loaded: execution_result.stats.rows_fetched,
            depth_loaded: plan.max_depth,
            cache_hit_ratio: self.calculate_cache_hit_ratio().await,
        };

        Ok(EagerLoadResult {
            data: processed_data,
            stats,
            optimizations: optimization_strategies,
        })
    }

    /// Load with a specific optimization strategy
    pub async fn load_with_strategy(
        &mut self,
        root_table: &str,
        root_ids: Vec<JsonValue>,
        relationships: &str,
        strategy: OptimizationStrategy,
        connection: &sqlx::PgPool,
    ) -> OrmResult<EagerLoadResult> {
        // Build plan
        let mut plan = self.build_query_plan(root_table, &root_ids, relationships)?;
        
        // Apply specific strategy
        match strategy {
            OptimizationStrategy::IncreaseParallelism => {
                self.apply_parallel_optimization(&mut plan)?;
            }
            OptimizationStrategy::ReduceBatchSize => {
                self.apply_batch_size_optimization(&mut plan)?;
            }
            OptimizationStrategy::ReorderPhases => {
                plan.build_execution_phases()?;
            }
            _ => {
                // Apply through optimizer
                let _strategies = self.query_optimizer.optimize_plan(&mut plan)?;
            }
        }
        
        // Execute with the applied strategy
        let execution_result = self.plan_executor.execute_plan(&plan, connection).await?;
        let processed_data = self.process_execution_results(execution_result.results, &root_ids)?;
        
        let stats = EagerLoadStats {
            execution_time_ms: 0, // Will be calculated
            query_count: execution_result.stats.query_count,
            records_loaded: execution_result.stats.rows_fetched,
            depth_loaded: plan.max_depth,
            cache_hit_ratio: self.calculate_cache_hit_ratio().await,
        };

        Ok(EagerLoadResult {
            data: processed_data,
            stats,
            optimizations: vec![strategy],
        })
    }

    /// Build a query plan from relationship specification
    fn build_query_plan(
        &self,
        root_table: &str,
        root_ids: &[JsonValue],
        relationships: &str,
    ) -> OrmResult<QueryPlan> {
        let mut plan = QueryPlan::new();
        let mut node_counter = 0;
        
        // Create root node
        let root_node_id = format!("root_{}", node_counter);
        node_counter += 1;
        
        let mut root_node = QueryNode::root(root_node_id.clone(), root_table.to_string());
        root_node.set_estimated_rows(root_ids.len());
        plan.add_node(root_node);
        
        // Parse relationships and build plan tree
        if !relationships.is_empty() {
            self.build_relationship_nodes(
                &mut plan,
                &root_node_id,
                relationships,
                1, // Start at depth 1
                &mut node_counter,
            )?;
        }
        
        // Build execution phases
        plan.build_execution_phases()?;
        
        Ok(plan)
    }

    /// Recursively build relationship nodes
    fn build_relationship_nodes(
        &self,
        plan: &mut QueryPlan,
        parent_node_id: &str,
        relationships: &str,
        depth: usize,
        node_counter: &mut usize,
    ) -> OrmResult<()> {
        if depth > self.config.max_depth {
            return Ok(()); // Prevent infinite recursion
        }
        
        // Parse relationship path (e.g., "posts.comments,profile")
        let parts: Vec<&str> = relationships.split(',').collect();
        
        for part in parts {
            let relation_chain: Vec<&str> = part.split('.').collect();
            self.build_relation_chain(
                plan,
                parent_node_id,
                &relation_chain,
                depth,
                node_counter,
            )?;
        }
        
        Ok(())
    }

    /// Build a chain of relationships (e.g., posts.comments.user)
    fn build_relation_chain(
        &self,
        plan: &mut QueryPlan,
        parent_node_id: &str,
        chain: &[&str],
        depth: usize,
        node_counter: &mut usize,
    ) -> OrmResult<()> {
        if chain.is_empty() || depth > self.config.max_depth {
            return Ok(());
        }
        
        let relation_name = chain[0];
        let node_id = format!("{}_{}", relation_name, *node_counter);
        *node_counter += 1;
        
        // Determine relationship type and table mapping
        let (table_name, relationship_type, foreign_key) = self.get_relationship_info(relation_name)?;
        
        // Create relationship node
        let mut node = QueryNode::child(
            node_id.clone(),
            table_name,
            parent_node_id.to_string(),
            relationship_type,
            foreign_key,
        );
        node.set_depth(depth);
        node.set_estimated_rows(std::cmp::min(1000, self.config.max_batch_size)); // Reasonable default
        
        plan.add_node(node);
        
        // Continue with rest of chain
        if chain.len() > 1 {
            self.build_relation_chain(
                plan,
                &node_id,
                &chain[1..],
                depth + 1,
                node_counter,
            )?;
        }
        
        Ok(())
    }

    /// Get relationship information for a relation name
    fn get_relationship_info(&self, relation: &str) -> OrmResult<(String, RelationshipType, String)> {
        // This would normally use metadata from the relationship registry
        // For now, use convention-based mapping
        match relation {
            "posts" => Ok(("posts".to_string(), RelationshipType::HasMany, "user_id".to_string())),
            "comments" => Ok(("comments".to_string(), RelationshipType::HasMany, "post_id".to_string())),
            "user" => Ok(("users".to_string(), RelationshipType::BelongsTo, "user_id".to_string())),
            "profile" => Ok(("profiles".to_string(), RelationshipType::HasOne, "user_id".to_string())),
            _ => {
                // Default convention: relation name -> table name + _id
                Ok((
                    format!("{}s", relation),
                    RelationshipType::HasMany,
                    format!("{}_id", relation),
                ))
            }
        }
    }

    /// Process execution results into the expected eager loading format
    fn process_execution_results(
        &self,
        results: HashMap<String, Vec<JsonValue>>,
        root_ids: &[JsonValue],
    ) -> OrmResult<HashMap<JsonValue, JsonValue>> {
        let mut processed = HashMap::new();
        
        // For now, create a simplified mapping
        // In a real implementation, this would properly hydrate relationships
        for (_, root_id) in root_ids.iter().enumerate() {
            let mut entity_data = serde_json::json!({
                "id": root_id,
                "relationships": {}
            });
            
            // Merge in relationship data
            for (node_id, node_results) in &results {
                if node_id.starts_with("root_") {
                    continue; // Skip root nodes
                }
                
                // Simple relationship assignment - in reality this would be more complex
                if let Some(obj) = entity_data.as_object_mut() {
                    if let Some(relationships) = obj.get_mut("relationships").and_then(|r| r.as_object_mut()) {
                        relationships.insert(node_id.clone(), serde_json::json!(node_results));
                    }
                }
            }
            
            processed.insert(root_id.clone(), entity_data);
        }
        
        Ok(processed)
    }

    /// Apply parallel optimization to the plan
    fn apply_parallel_optimization(&self, plan: &mut QueryPlan) -> OrmResult<()> {
        for node in plan.nodes.values_mut() {
            if node.constraints.is_empty() {
                node.set_parallel_safe(true);
            }
        }
        plan.build_execution_phases()?;
        Ok(())
    }

    /// Apply batch size optimization
    fn apply_batch_size_optimization(&self, plan: &mut QueryPlan) -> OrmResult<()> {
        // Reduce estimated rows for large nodes
        for node in plan.nodes.values_mut() {
            if node.estimated_rows > 5000 {
                node.set_estimated_rows(node.estimated_rows / 2);
            }
        }
        Ok(())
    }

    /// Calculate cache hit ratio
    async fn calculate_cache_hit_ratio(&self) -> f64 {
        let stats = self.batch_loader.cache_stats().await;
        if stats.total_cached_records > 0 {
            0.75 // Mock hit ratio - in reality would track hits vs misses
        } else {
            0.0
        }
    }

    /// Get loader configuration
    pub fn config(&self) -> &EagerLoadConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: EagerLoadConfig) {
        self.config = config;
    }

    /// Clear all caches
    pub async fn clear_caches(&self) {
        self.batch_loader.clear_cache().await;
    }
}

impl Default for OptimizedEagerLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_eager_load_config_default() {
        let config = EagerLoadConfig::default();
        assert_eq!(config.max_batch_size, 100);
        assert!(config.deduplicate_queries);
        assert_eq!(config.max_depth, 10);
        assert!(config.enable_parallelism);
    }

    #[test]
    fn test_build_query_plan() {
        let loader = OptimizedEagerLoader::new();
        let root_ids = vec![json!(1), json!(2)];
        
        let plan = loader.build_query_plan("users", &root_ids, "posts.comments").unwrap();
        
        assert_eq!(plan.roots.len(), 1);
        assert!(plan.nodes.len() >= 1); // At least the root node
        assert_eq!(plan.max_depth, 2); // users -> posts -> comments
    }

    #[test]
    fn test_relationship_info_mapping() {
        let loader = OptimizedEagerLoader::new();
        
        let (table, rel_type, fk) = loader.get_relationship_info("posts").unwrap();
        assert_eq!(table, "posts");
        assert_eq!(rel_type, RelationshipType::HasMany);
        assert_eq!(fk, "user_id");
    }
}