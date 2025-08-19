use crate::{
    error::{OrmError, OrmResult},
    loading::batch_loader::BatchLoader,
};
use super::plan::{QueryPlan, QueryNode};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// Result of executing a query plan
#[derive(Debug)]
pub struct ExecutionResult {
    /// Query results grouped by node ID
    pub results: HashMap<String, Vec<JsonValue>>,
    /// Execution statistics
    pub stats: ExecutionStats,
    /// Errors that occurred during execution
    pub errors: Vec<OrmError>,
}

/// Statistics about query plan execution
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    /// Total execution time
    pub total_duration: Duration,
    /// Time spent on each phase
    pub phase_durations: Vec<Duration>,
    /// Number of queries executed
    pub query_count: usize,
    /// Total rows fetched
    pub rows_fetched: usize,
    /// Number of phases executed in parallel
    pub parallel_phases: usize,
    /// Average response time per query
    pub avg_query_time: Duration,
    /// Peak memory usage (if available)
    pub peak_memory_mb: Option<f64>,
}

impl ExecutionStats {
    pub fn new() -> Self {
        Self {
            total_duration: Duration::from_secs(0),
            phase_durations: Vec::new(),
            query_count: 0,
            rows_fetched: 0,
            parallel_phases: 0,
            avg_query_time: Duration::from_secs(0),
            peak_memory_mb: None,
        }
    }

    /// Calculate average query time
    pub fn calculate_averages(&mut self) {
        if self.query_count > 0 {
            self.avg_query_time = self.total_duration / self.query_count as u32;
        }
    }

    /// Add phase duration
    pub fn add_phase_duration(&mut self, duration: Duration) {
        self.phase_durations.push(duration);
        self.total_duration += duration;
    }
}

impl Default for ExecutionStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Executes optimized query plans with parallel processing
pub struct PlanExecutor {
    /// Batch loader for executing queries
    batch_loader: BatchLoader,
    /// Maximum number of parallel tasks
    max_parallel_tasks: usize,
    /// Timeout for individual queries
    query_timeout: Duration,
}

impl PlanExecutor {
    /// Create a new plan executor
    pub fn new(batch_loader: BatchLoader) -> Self {
        Self {
            batch_loader,
            max_parallel_tasks: 10, // Reasonable default
            query_timeout: Duration::from_secs(30),
        }
    }

    /// Create a plan executor with custom configuration
    pub fn with_config(
        batch_loader: BatchLoader,
        max_parallel_tasks: usize,
        query_timeout: Duration,
    ) -> Self {
        Self {
            batch_loader,
            max_parallel_tasks,
            query_timeout,
        }
    }

    /// Execute a query plan with optimized parallel processing
    pub async fn execute_plan(
        &self,
        plan: &QueryPlan,
        connection: &sqlx::PgPool,
    ) -> OrmResult<ExecutionResult> {
        let start_time = Instant::now();
        let mut results: HashMap<String, Vec<JsonValue>> = HashMap::new();
        let mut stats = ExecutionStats::new();
        let mut errors = Vec::new();

        // Execute each phase
        for (phase_index, phase) in plan.execution_phases.iter().enumerate() {
            let phase_start = Instant::now();
            
            if phase.len() == 1 {
                // Single node - execute directly
                let node_id = &phase[0];
                if let Some(node) = plan.nodes.get(node_id) {
                    match self.execute_node_query(node, connection).await {
                        Ok(node_results) => {
                            stats.query_count += 1;
                            stats.rows_fetched += node_results.len();
                            results.insert(node_id.clone(), node_results);
                        }
                        Err(e) => errors.push(e),
                    }
                }
            } else {
                // Multiple nodes - execute in parallel
                stats.parallel_phases += 1;
                let parallel_results = self.execute_phase_parallel(phase, plan, connection).await;
                
                for (node_id, result) in parallel_results {
                    match result {
                        Ok(node_results) => {
                            stats.query_count += 1;
                            stats.rows_fetched += node_results.len();
                            results.insert(node_id, node_results);
                        }
                        Err(e) => errors.push(e),
                    }
                }
            }
            
            let phase_duration = phase_start.elapsed();
            stats.add_phase_duration(phase_duration);
        }

        stats.total_duration = start_time.elapsed();
        stats.calculate_averages();

        Ok(ExecutionResult {
            results,
            stats,
            errors,
        })
    }

    /// Execute a phase with multiple nodes in parallel
    async fn execute_phase_parallel(
        &self,
        phase: &[String],
        plan: &QueryPlan,
        connection: &sqlx::PgPool,
    ) -> HashMap<String, OrmResult<Vec<JsonValue>>> {
        let handles: Vec<JoinHandle<(String, OrmResult<Vec<JsonValue>>)>> = Vec::new();
        let mut results = HashMap::new();

        // Limit parallel tasks to avoid overwhelming the database
        let chunks: Vec<_> = phase.chunks(self.max_parallel_tasks).collect();
        
        for chunk in chunks {
            let mut chunk_handles = Vec::new();
            
            for node_id in chunk {
                if let Some(node) = plan.nodes.get(node_id) {
                    let node_clone = node.clone();
                    let node_id_clone = node_id.clone();
                    let connection_clone = connection.clone();
                    
                    let handle = tokio::spawn(async move {
                        let result = Self::execute_node_query_static(&node_clone, &connection_clone).await;
                        (node_id_clone, result)
                    });
                    
                    chunk_handles.push(handle);
                }
            }
            
            // Wait for chunk to complete
            for handle in chunk_handles {
                match handle.await {
                    Ok((node_id, result)) => {
                        results.insert(node_id, result);
                    }
                    Err(e) => {
                        eprintln!("Task join error: {}", e);
                    }
                }
            }
        }

        results
    }

    /// Execute a single node's query
    async fn execute_node_query(
        &self,
        node: &QueryNode,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        // Add timeout wrapper
        let query_future = self.execute_node_query_impl(node, connection);
        
        match tokio::time::timeout(self.query_timeout, query_future).await {
            Ok(result) => result,
            Err(_) => Err(OrmError::Query(format!(
                "Query timeout for node '{}' after {:?}",
                node.id, self.query_timeout
            ))),
        }
    }

    /// Actual implementation of node query execution using the batch loader
    async fn execute_node_query_impl(
        &self,
        node: &QueryNode,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        // Use the batch loader to execute real database queries
        // This replaces the previous mock implementation with actual database queries
        
        if node.is_root() {
            // Root node: Query all records from the table (with constraints if any)
            self.execute_root_query(node, connection).await
        } else {
            // Child node: Query based on parent relationship
            self.execute_relationship_query(node, connection).await
        }
    }

    /// Execute query for root node (no parent relationship)
    async fn execute_root_query(
        &self,
        node: &QueryNode,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        use crate::query::QueryBuilder;
        
        // Build base query for the table
        let mut query = QueryBuilder::<()>::new().from(&node.table);
        
        // Apply constraints if any
        for constraint in &node.constraints {
            query = query.where_raw(constraint);
        }
        
        // Apply reasonable limit to prevent excessive data loading
        let limit = std::cmp::min(node.estimated_rows, 1000);
        query = query.limit(limit as i64);
        
        // Execute the query
        let (sql, _params) = query.to_sql_with_params();
        let db_query = sqlx::query(&sql);
        
        let rows = db_query.fetch_all(connection).await
            .map_err(|e| OrmError::Database(e.to_string()))?;
        
        // Convert rows to JSON values
        let results: Result<Vec<JsonValue>, OrmError> = rows.into_iter()
            .map(|row| {
                crate::loading::batch_loader::row_conversion::convert_row_to_json(&row)
                    .map_err(|e| OrmError::Serialization(e.to_string()))
            })
            .collect();
        
        results
    }

    /// Execute query for child node (with parent relationship)  
    async fn execute_relationship_query(
        &self,
        node: &QueryNode,
        _connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        // For relationship queries, we need parent IDs to load the related records
        // In a real implementation, this would be called with parent IDs
        // For now, return empty results as this indicates the need for proper relationship loading
        
        // This method should be called with specific parent IDs via the batch loader
        // Example: self.batch_loader.load_batch::<Model>(parent_ids, &node.table, connection).await
        
        // Return empty for now - the actual loading should happen through the relationship system
        Ok(Vec::new())
    }

    /// Static version of execute_node_query for use in async tasks
    async fn execute_node_query_static(
        node: &QueryNode,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        // Use real database queries instead of mock data
        if node.is_root() {
            Self::execute_root_query_static(node, connection).await
        } else {
            Self::execute_relationship_query_static(node, connection).await
        }
    }

    /// Static version of root query execution
    async fn execute_root_query_static(
        node: &QueryNode,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        use crate::query::QueryBuilder;
        
        // Build base query for the table
        let mut query = QueryBuilder::<()>::new().from(&node.table);
        
        // Apply constraints if any
        for constraint in &node.constraints {
            query = query.where_raw(constraint);
        }
        
        // Apply reasonable limit to prevent excessive data loading
        let limit = std::cmp::min(node.estimated_rows, 1000);
        query = query.limit(limit as i64);
        
        // Execute the query
        let (sql, _params) = query.to_sql_with_params();
        let db_query = sqlx::query(&sql);
        
        let rows = db_query.fetch_all(connection).await
            .map_err(|e| OrmError::Database(e.to_string()))?;
        
        // Convert rows to JSON values
        let results: Result<Vec<JsonValue>, OrmError> = rows.into_iter()
            .map(|row| {
                crate::loading::batch_loader::row_conversion::convert_row_to_json(&row)
                    .map_err(|e| OrmError::Serialization(e.to_string()))
            })
            .collect();
        
        results
    }

    /// Static version of relationship query execution
    async fn execute_relationship_query_static(
        node: &QueryNode,
        _connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        // For relationship queries, we need parent IDs to load the related records
        // Return empty for now - the actual loading should happen through the relationship system
        Ok(Vec::new())
    }

    /// Get executor statistics
    pub fn get_stats(&self) -> ExecutorStats {
        ExecutorStats {
            max_parallel_tasks: self.max_parallel_tasks,
            query_timeout: self.query_timeout,
        }
    }

    /// Update executor configuration
    pub fn set_max_parallel_tasks(&mut self, max_tasks: usize) {
        self.max_parallel_tasks = max_tasks;
    }

    pub fn set_query_timeout(&mut self, timeout: Duration) {
        self.query_timeout = timeout;
    }
}

/// Statistics about the executor configuration
#[derive(Debug, Clone)]
pub struct ExecutorStats {
    pub max_parallel_tasks: usize,
    pub query_timeout: Duration,
}