/// Loading optimization modules for the elif ORM
/// Provides batch loading, query deduplication, and query optimization

pub mod batch_loader;
pub mod query_deduplicator;
pub mod query_optimizer;
pub mod optimizer;
pub mod eager_loader;

pub use batch_loader::{BatchLoader, BatchConfig, BatchLoadResult};
pub use query_deduplicator::{QueryDeduplicator, QueryKey, DeduplicationStats};
pub use query_optimizer::{QueryOptimizer, QueryPlan, QueryNode, PlanAnalysis, OptimizedQueryExecutor};
pub use optimizer::{
    QueryNode as NewQueryNode, 
    QueryPlan as NewQueryPlan, 
    PlanAnalysis as NewPlanAnalysis,
    QueryOptimizer as NewQueryOptimizer,
    PlanExecutor,
    ExecutionResult,
    ExecutionStats,
    OptimizationStrategy,
    RiskLevel
};
pub use eager_loader::{OptimizedEagerLoader, EagerLoadConfig, EagerLoadResult, EagerLoadStats};