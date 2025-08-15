/// Query optimization module for nested relationship loading
/// 
/// This module provides query planning, execution plan optimization,
/// and parallel execution capabilities for complex nested queries.

pub mod plan;
pub mod executor;
pub mod analyzer;

pub use plan::{QueryNode, QueryPlan};
pub use executor::{PlanExecutor, ExecutionResult, ExecutionStats};
pub use analyzer::{PlanAnalysis, QueryOptimizer, OptimizationStrategy, RiskLevel};