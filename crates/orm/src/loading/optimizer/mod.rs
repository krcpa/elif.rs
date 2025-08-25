pub mod analyzer;
pub mod executor;
/// Query optimization module for nested relationship loading
///
/// This module provides query planning, execution plan optimization,
/// and parallel execution capabilities for complex nested queries.
pub mod plan;

pub use analyzer::{OptimizationStrategy, PlanAnalysis, QueryOptimizer, RiskLevel};
pub use executor::{ExecutionResult, ExecutionStats, PlanExecutor};
pub use plan::{QueryNode, QueryPlan};
