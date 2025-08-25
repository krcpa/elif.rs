/// Loading optimization modules for the elif ORM
/// Provides batch loading, query deduplication, and query optimization
pub mod batch_loader;
pub mod eager_loader;
pub mod optimizer;
pub mod query_deduplicator;
pub mod query_optimizer;

pub use batch_loader::{BatchConfig, BatchLoadResult, BatchLoader};
pub use eager_loader::{EagerLoadConfig, EagerLoadResult, EagerLoadStats, OptimizedEagerLoader};
pub use optimizer::{
    ExecutionResult, ExecutionStats, OptimizationStrategy, PlanAnalysis as NewPlanAnalysis,
    PlanExecutor, QueryNode as NewQueryNode, QueryOptimizer as NewQueryOptimizer,
    QueryPlan as NewQueryPlan, RiskLevel,
};
pub use query_deduplicator::{DeduplicationStats, QueryDeduplicator, QueryKey};
pub use query_optimizer::{
    OptimizedQueryExecutor, PlanAnalysis, QueryNode, QueryOptimizer, QueryPlan,
};
