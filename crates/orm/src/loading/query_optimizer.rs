// Legacy compatibility layer - use the new optimizer module for new code
// This file maintains backward compatibility while redirecting to the new modular structure

pub use crate::loading::optimizer::{
    QueryNode, QueryPlan, PlanAnalysis, QueryOptimizer, OptimizationStrategy,
    PlanExecutor, ExecutionResult, ExecutionStats, RiskLevel
};

use crate::{
    error::OrmResult,
    loading::{
        batch_loader::BatchLoader,
        optimizer::{
            executor::PlanExecutor as NewPlanExecutor,
            analyzer::QueryOptimizer as NewQueryOptimizer,
        },
    },
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Legacy OptimizedQueryExecutor - now wraps the new modular implementation
pub struct OptimizedQueryExecutor {
    executor: NewPlanExecutor,
    optimizer: NewQueryOptimizer,
}

impl OptimizedQueryExecutor {
    /// Create a new optimized query executor
    pub fn new(batch_loader: BatchLoader) -> Self {
        Self {
            executor: NewPlanExecutor::new(batch_loader),
            optimizer: NewQueryOptimizer::new(),
        }
    }

    /// Execute a query plan with optimization
    pub async fn execute_optimized(
        &self,
        plan: &mut QueryPlan,
        connection: &sqlx::PgPool,
    ) -> OrmResult<HashMap<String, Vec<JsonValue>>> {
        // Optimize the plan first
        let _strategies = self.optimizer.optimize_plan(plan)?;
        
        // Execute the optimized plan
        let result = self.executor.execute_plan(plan, connection).await?;
        
        Ok(result.results)
    }

    /// Execute plan with detailed analysis
    pub async fn execute_with_analysis(
        &self,
        plan: &mut QueryPlan,
        connection: &sqlx::PgPool,
    ) -> OrmResult<(HashMap<String, Vec<JsonValue>>, ExecutionStats)> {
        // Optimize the plan first
        let _strategies = self.optimizer.optimize_plan(plan)?;
        
        // Execute the optimized plan
        let result = self.executor.execute_plan(plan, connection).await?;
        
        Ok((result.results, result.stats))
    }

    /// Analyze a query plan without executing it
    pub fn analyze_plan(&self, plan: &QueryPlan) -> OrmResult<PlanAnalysis> {
        self.optimizer.analyze_plan(plan)
    }
}