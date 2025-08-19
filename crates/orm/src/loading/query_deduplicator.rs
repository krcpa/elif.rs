use crate::{
    error::{OrmError, OrmResult},
};
use serde_json::Value as JsonValue;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Represents a unique query that can be deduplicated
#[derive(Debug, Clone)]
pub struct QueryKey {
    /// Table being queried
    pub table: String,
    /// Type of query (e.g., "select", "relationship")
    pub query_type: String,
    /// Conditions or parameters that make this query unique
    pub conditions: HashMap<String, Vec<Value>>,
}

impl QueryKey {
    /// Create a new query key for a relationship query
    pub fn relationship(
        table: &str,
        foreign_key: &str,
        parent_ids: &[Value],
    ) -> Self {
        let mut conditions = HashMap::new();
        conditions.insert(foreign_key.to_string(), parent_ids.to_vec());
        
        Self {
            table: table.to_string(),
            query_type: "relationship".to_string(),
            conditions,
        }
    }

    /// Create a new query key for a batch select query
    pub fn batch_select(table: &str, ids: &[Value]) -> Self {
        let mut conditions = HashMap::new();
        conditions.insert("id".to_string(), ids.to_vec());
        
        Self {
            table: table.to_string(),
            query_type: "batch_select".to_string(),
            conditions,
        }
    }
}

impl PartialEq for QueryKey {
    fn eq(&self, other: &Self) -> bool {
        self.table == other.table
            && self.query_type == other.query_type
            && self.conditions == other.conditions
    }
}

impl Eq for QueryKey {}

impl Hash for QueryKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.table.hash(state);
        self.query_type.hash(state);
        
        // Sort conditions for consistent hashing
        let mut sorted_conditions: Vec<_> = self.conditions.iter().collect();
        sorted_conditions.sort_by_key(|(k, _)| k.as_str());
        
        for (key, values) in sorted_conditions {
            key.hash(state);
            for value in values {
                // Hash the JSON representation for consistency
                serde_json::to_string(value).unwrap_or_default().hash(state);
            }
        }
    }
}

/// Tracks pending queries to enable deduplication
#[derive(Debug)]
struct PendingQuery {
    /// The result future that will be shared among all waiters
    result: Arc<Mutex<Option<OrmResult<Vec<JsonValue>>>>>,
    /// Number of requests waiting for this query
    waiter_count: usize,
}

/// Query deduplicator that prevents executing identical queries multiple times
pub struct QueryDeduplicator {
    /// Currently executing queries
    pending_queries: Arc<RwLock<HashMap<QueryKey, PendingQuery>>>,
    /// Statistics about deduplication
    stats: Arc<RwLock<DeduplicationStats>>,
}

impl QueryDeduplicator {
    /// Create a new query deduplicator
    pub fn new() -> Self {
        Self {
            pending_queries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(DeduplicationStats::default())),
        }
    }

    /// Execute a query with deduplication
    /// If an identical query is already running, wait for its result instead
    pub async fn execute_deduplicated<F, Fut>(
        &self,
        query_key: QueryKey,
        execute_fn: F,
    ) -> OrmResult<Vec<JsonValue>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = OrmResult<Vec<JsonValue>>>,
    {
        // Check if query is already pending
        {
            let mut pending = self.pending_queries.write().await;
            if let Some(pending_query) = pending.get_mut(&query_key) {
                // Query is already running, increment waiter count
                pending_query.waiter_count += 1;
                let result_mutex = pending_query.result.clone();
                
                // Update stats
                let mut stats = self.stats.write().await;
                stats.queries_deduplicated += 1;
                drop(stats);
                drop(pending);
                
                // Wait for the result
                let mut result_guard = result_mutex.lock().await;
                while result_guard.is_none() {
                    // Release lock and wait
                    drop(result_guard);
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    result_guard = result_mutex.lock().await;
                }
                
                // Clone the result and return
                return result_guard
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .map(|v| v.clone())
                    .map_err(|e| OrmError::Query(e.to_string()));
            } else {
                // New query, add to pending
                let result_mutex = Arc::new(Mutex::new(None));
                pending.insert(
                    query_key.clone(),
                    PendingQuery {
                        result: result_mutex.clone(),
                        waiter_count: 1,
                    },
                );
                
                // Update stats
                let mut stats = self.stats.write().await;
                stats.unique_queries_executed += 1;
                drop(stats);
                drop(pending);
                
                // Execute the query
                let result = execute_fn().await;
                
                // Store result and clean up
                let mut pending = self.pending_queries.write().await;
                if let Some(pending_query) = pending.get(&query_key) {
                    let mut result_guard = pending_query.result.lock().await;
                    *result_guard = Some(result.clone());
                }
                pending.remove(&query_key);
                
                return result;
            }
        }
    }

    /// Get deduplication statistics
    pub async fn stats(&self) -> DeduplicationStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = DeduplicationStats::default();
    }

    /// Check if any queries are currently pending
    pub async fn has_pending_queries(&self) -> bool {
        !self.pending_queries.read().await.is_empty()
    }

    /// Get the number of pending queries
    pub async fn pending_query_count(&self) -> usize {
        self.pending_queries.read().await.len()
    }
}

/// Statistics about query deduplication
#[derive(Debug, Clone, Default)]
pub struct DeduplicationStats {
    /// Number of unique queries executed
    pub unique_queries_executed: usize,
    /// Number of duplicate queries that were deduplicated
    pub queries_deduplicated: usize,
    /// Total queries saved by deduplication
    pub queries_saved: usize,
}

impl DeduplicationStats {
    /// Calculate the deduplication ratio
    pub fn deduplication_ratio(&self) -> f64 {
        let total = self.unique_queries_executed + self.queries_deduplicated;
        if total == 0 {
            0.0
        } else {
            self.queries_deduplicated as f64 / total as f64
        }
    }

    /// Get total queries processed
    pub fn total_queries(&self) -> usize {
        self.unique_queries_executed + self.queries_deduplicated
    }
}

impl Display for DeduplicationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "QueryDeduplicator Stats: {} unique queries, {} deduplicated ({:.1}% dedup rate)",
            self.unique_queries_executed,
            self.queries_deduplicated,
            self.deduplication_ratio() * 100.0
        )
    }
}

