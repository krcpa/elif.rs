/// Configuration for batch loading operations
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of records to load in a single query
    pub max_batch_size: usize,
    /// Maximum depth of nested relationships to load
    pub max_depth: usize,
    /// Enable parallel execution of independent queries
    pub parallel_execution: bool,
    /// Enable query deduplication
    pub deduplicate_queries: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            max_depth: 10,
            parallel_execution: true,
            deduplicate_queries: true,
        }
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub cached_queries: usize,
    pub total_cached_records: usize,
}