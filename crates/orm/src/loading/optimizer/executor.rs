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
        let mut handles: Vec<JoinHandle<(String, OrmResult<Vec<JsonValue>>)>> = Vec::new();
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

    /// Actual implementation of node query execution
    async fn execute_node_query_impl(
        &self,
        node: &QueryNode,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        // For now, simulate the query execution with better mock data
        // In a real implementation, this would:
        // 1. Use the node's metadata to build proper SQL queries
        // 2. Execute the queries using the batch loader
        // 3. Handle relationship joins and constraints
        
        // Enhanced mock implementation with more realistic data
        let mock_data = match node.table.as_str() {
            "users" => {
                vec![
                    serde_json::json!({
                        "id": 1,
                        "name": "John Doe",
                        "email": "john@example.com",
                        "created_at": "2024-01-01T00:00:00Z"
                    }),
                    serde_json::json!({
                        "id": 2,
                        "name": "Jane Smith", 
                        "email": "jane@example.com",
                        "created_at": "2024-01-02T00:00:00Z"
                    })
                ]
            }
            "posts" => {
                vec![
                    serde_json::json!({
                        "id": 1,
                        "user_id": 1,
                        "title": "First Post",
                        "content": "Hello World!",
                        "created_at": "2024-01-01T12:00:00Z"
                    }),
                    serde_json::json!({
                        "id": 2,
                        "user_id": 1,
                        "title": "Second Post",
                        "content": "More content here",
                        "created_at": "2024-01-02T12:00:00Z"
                    })
                ]
            }
            "comments" => {
                vec![
                    serde_json::json!({
                        "id": 1,
                        "post_id": 1,
                        "user_id": 2,
                        "content": "Great post!",
                        "created_at": "2024-01-01T13:00:00Z"
                    }),
                    serde_json::json!({
                        "id": 2,
                        "post_id": 1,
                        "user_id": 1,
                        "content": "Thanks!",
                        "created_at": "2024-01-01T14:00:00Z"
                    })
                ]
            }
            _ => {
                vec![
                    serde_json::json!({
                        "id": 1,
                        "table": node.table,
                        "depth": node.depth,
                        "estimated_rows": node.estimated_rows,
                        "parallel_safe": node.parallel_safe
                    })
                ]
            }
        };
        
        // Simulate realistic database response time based on estimated rows
        let delay_ms = std::cmp::min((node.estimated_rows / 100) as u64, 100);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        
        Ok(mock_data)
    }

    /// Static version of execute_node_query for use in async tasks
    async fn execute_node_query_static(
        node: &QueryNode,
        connection: &sqlx::PgPool,
    ) -> OrmResult<Vec<JsonValue>> {
        // Enhanced mock implementation matching the instance method
        let mock_data = match node.table.as_str() {
            "users" => {
                vec![
                    serde_json::json!({
                        "id": 1,
                        "name": "John Doe",
                        "email": "john@example.com",
                        "created_at": "2024-01-01T00:00:00Z"
                    }),
                    serde_json::json!({
                        "id": 2,
                        "name": "Jane Smith", 
                        "email": "jane@example.com",
                        "created_at": "2024-01-02T00:00:00Z"
                    })
                ]
            }
            "posts" => {
                vec![
                    serde_json::json!({
                        "id": 1,
                        "user_id": 1,
                        "title": "First Post",
                        "content": "Hello World!",
                        "created_at": "2024-01-01T12:00:00Z"
                    }),
                    serde_json::json!({
                        "id": 2,
                        "user_id": 1,
                        "title": "Second Post",
                        "content": "More content here",
                        "created_at": "2024-01-02T12:00:00Z"
                    })
                ]
            }
            "comments" => {
                vec![
                    serde_json::json!({
                        "id": 1,
                        "post_id": 1,
                        "user_id": 2,
                        "content": "Great post!",
                        "created_at": "2024-01-01T13:00:00Z"
                    }),
                    serde_json::json!({
                        "id": 2,
                        "post_id": 1,
                        "user_id": 1,
                        "content": "Thanks!",
                        "created_at": "2024-01-01T14:00:00Z"
                    })
                ]
            }
            _ => {
                vec![
                    serde_json::json!({
                        "id": 1,
                        "table": node.table,
                        "depth": node.depth,
                        "estimated_rows": node.estimated_rows,
                        "parallel_safe": node.parallel_safe
                    })
                ]
            }
        };
        
        // Simulate realistic database response time
        let delay_ms = std::cmp::min((node.estimated_rows / 100) as u64, 100);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        
        Ok(mock_data)
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