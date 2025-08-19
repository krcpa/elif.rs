use crate::error::OrmResult;
use super::plan::{QueryPlan, PlanStatistics};
use std::collections::HashSet;

/// Analysis result for a query plan
#[derive(Debug, Clone)]
pub struct PlanAnalysis {
    /// Overall plan complexity score
    pub complexity_score: f64,
    /// Estimated execution time in milliseconds
    pub estimated_execution_time: u64,
    /// Potential bottlenecks
    pub bottlenecks: Vec<String>,
    /// Optimization recommendations
    pub recommendations: Vec<String>,
    /// Risk assessment
    pub risk_level: RiskLevel,
    /// Plan statistics
    pub statistics: PlanStatistics,
}

/// Risk level for query execution
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,     // Simple, fast queries
    Medium,  // Moderate complexity
    High,    // Complex queries that might be slow
    Critical, // Potentially problematic queries
}

/// Optimization strategies
#[derive(Debug, Clone)]
pub enum OptimizationStrategy {
    /// Increase parallel execution
    IncreaseParallelism,
    /// Reduce batch sizes
    ReduceBatchSize,
    /// Add query constraints
    AddConstraints,
    /// Reorder execution phases
    ReorderPhases,
    /// Split large queries
    SplitQueries,
    /// Add indexes (recommendation)
    SuggestIndexes(Vec<String>),
}

/// Query optimizer and analyzer
pub struct QueryOptimizer {
    /// Maximum allowed complexity score
    max_complexity: f64,
    /// Target execution time threshold (ms)
    target_execution_time: u64,
}

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new() -> Self {
        Self {
            max_complexity: 100.0,
            target_execution_time: 5000, // 5 seconds
        }
    }

    /// Create optimizer with custom settings
    pub fn with_settings(
        max_complexity: f64,
        target_execution_time: u64,
    ) -> Self {
        Self {
            max_complexity,
            target_execution_time,
        }
    }

    /// Analyze a query plan and provide recommendations
    pub fn analyze_plan(&self, plan: &QueryPlan) -> OrmResult<PlanAnalysis> {
        let statistics = plan.statistics();
        let complexity_score = plan.complexity_score();
        
        // Estimate execution time based on complexity and row counts
        let estimated_execution_time = self.estimate_execution_time(plan);
        
        // Identify bottlenecks
        let bottlenecks = self.identify_bottlenecks(plan);
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(plan, &bottlenecks);
        
        // Assess risk level
        let risk_level = self.assess_risk_level(complexity_score, estimated_execution_time, &bottlenecks);

        Ok(PlanAnalysis {
            complexity_score,
            estimated_execution_time,
            bottlenecks,
            recommendations,
            risk_level,
            statistics,
        })
    }

    /// Optimize a query plan using various strategies
    pub fn optimize_plan(&self, plan: &mut QueryPlan) -> OrmResult<Vec<OptimizationStrategy>> {
        let mut applied_strategies = Vec::new();
        
        // Analyze the plan first
        let analysis = self.analyze_plan(plan)?;
        
        // Apply optimizations based on analysis
        if analysis.complexity_score > self.max_complexity {
            // Try to reduce complexity
            if self.can_increase_parallelism(plan) {
                self.increase_parallelism(plan)?;
                applied_strategies.push(OptimizationStrategy::IncreaseParallelism);
            }
            
            if self.should_split_queries(plan) {
                applied_strategies.push(OptimizationStrategy::SplitQueries);
            }
        }
        
        if analysis.estimated_execution_time > self.target_execution_time {
            // Try to reduce execution time
            if self.can_reorder_phases(plan) {
                self.reorder_phases(plan)?;
                applied_strategies.push(OptimizationStrategy::ReorderPhases);
            }
            
            // Suggest indexes for frequently accessed tables
            let index_suggestions = self.suggest_indexes(plan);
            if !index_suggestions.is_empty() {
                applied_strategies.push(OptimizationStrategy::SuggestIndexes(index_suggestions));
            }
        }
        
        // Rebuild execution phases after optimizations
        plan.build_execution_phases()?;
        
        Ok(applied_strategies)
    }

    /// Estimate execution time for a plan (in milliseconds)
    fn estimate_execution_time(&self, plan: &QueryPlan) -> u64 {
        
        // Base time per node (database connection overhead)
        let base_time_per_node = 10; // 10ms per node
        
        // Time based on estimated rows
        let row_processing_time = plan.total_estimated_rows as u64 / 1000; // 1ms per 1000 rows
        
        // Depth penalty (more nesting = more complexity)
        let depth_penalty = (plan.max_depth as u64) * 50; // 50ms per depth level
        
        // Phase execution time (sequential vs parallel)
        let phase_time: u64 = plan.execution_phases.iter().map(|phase| {
            if phase.len() == 1 {
                base_time_per_node * 2 // Sequential execution penalty
            } else {
                base_time_per_node // Parallel execution
            }
        }).sum();
        
        let total_time = base_time_per_node * plan.nodes.len() as u64
            + row_processing_time
            + depth_penalty
            + phase_time;
        
        total_time
    }

    /// Identify potential bottlenecks in the plan
    fn identify_bottlenecks(&self, plan: &QueryPlan) -> Vec<String> {
        let mut bottlenecks = Vec::new();
        
        // Check for nodes with high estimated row counts
        for (id, node) in &plan.nodes {
            if node.estimated_rows > 10000 {
                bottlenecks.push(format!("High row count in node '{}': {} rows", id, node.estimated_rows));
            }
        }
        
        // Check for deep nesting
        if plan.max_depth > 5 {
            bottlenecks.push(format!("Deep nesting detected: {} levels", plan.max_depth));
        }
        
        // Check for sequential execution bottlenecks
        for (phase_idx, phase) in plan.execution_phases.iter().enumerate() {
            if phase.len() == 1 {
                let node_id = &phase[0];
                if let Some(node) = plan.nodes.get(node_id) {
                    if !node.parallel_safe {
                        bottlenecks.push(format!("Sequential bottleneck in phase {}: node '{}'", phase_idx, node_id));
                    }
                }
            }
        }
        
        // Check for unbalanced execution phases
        let avg_phase_size: f64 = plan.execution_phases.iter()
            .map(|p| p.len())
            .sum::<usize>() as f64 / plan.execution_phases.len() as f64;
        
        for (phase_idx, phase) in plan.execution_phases.iter().enumerate() {
            if phase.len() as f64 > avg_phase_size * 3.0 {
                bottlenecks.push(format!("Unbalanced phase {}: {} nodes (avg: {:.1})", 
                    phase_idx, phase.len(), avg_phase_size));
            }
        }
        
        bottlenecks
    }

    /// Generate optimization recommendations
    fn generate_recommendations(&self, plan: &QueryPlan, bottlenecks: &[String]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Recommendations based on plan characteristics
        if plan.max_depth > 3 {
            recommendations.push("Consider limiting relationship depth to improve performance".to_string());
        }
        
        if plan.total_estimated_rows > 50000 {
            recommendations.push("Consider adding query constraints to reduce data volume".to_string());
        }
        
        let parallel_nodes = plan.nodes.values().filter(|n| n.parallel_safe).count();
        let total_nodes = plan.nodes.len();
        if parallel_nodes < total_nodes / 2 {
            recommendations.push("Consider making more queries parallel-safe to improve throughput".to_string());
        }
        
        // Recommendations based on bottlenecks
        for bottleneck in bottlenecks {
            if bottleneck.contains("High row count") {
                recommendations.push("Consider adding pagination or filtering to reduce row counts".to_string());
            } else if bottleneck.contains("Deep nesting") {
                recommendations.push("Consider flattening the relationship structure or using separate queries".to_string());
            } else if bottleneck.contains("Sequential bottleneck") {
                recommendations.push("Consider optimizing sequential queries for parallel execution".to_string());
            }
        }
        
        // Database-specific recommendations
        recommendations.push("Ensure proper indexes exist on foreign key columns".to_string());
        recommendations.push("Consider using connection pooling for better resource utilization".to_string());
        
        // Remove duplicates
        recommendations.sort();
        recommendations.dedup();
        
        recommendations
    }

    /// Assess risk level based on various factors
    fn assess_risk_level(
        &self,
        complexity_score: f64,
        estimated_time: u64,
        bottlenecks: &[String],
    ) -> RiskLevel {
        let bottleneck_count = bottlenecks.len();
        
        if complexity_score > self.max_complexity * 2.0 
            || estimated_time > self.target_execution_time * 3
            || bottleneck_count > 5 {
            RiskLevel::Critical
        } else if complexity_score > self.max_complexity 
            || estimated_time > self.target_execution_time
            || bottleneck_count > 2 {
            RiskLevel::High
        } else if complexity_score > self.max_complexity * 0.7 
            || estimated_time > (self.target_execution_time as f64 * 0.7) as u64
            || bottleneck_count > 0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }

    /// Check if we can increase parallelism
    fn can_increase_parallelism(&self, plan: &QueryPlan) -> bool {
        plan.nodes.values().any(|node| !node.parallel_safe)
    }

    /// Increase parallelism where possible
    fn increase_parallelism(&self, plan: &mut QueryPlan) -> OrmResult<()> {
        for node in plan.nodes.values_mut() {
            // Make nodes parallel-safe if they don't have constraints
            if !node.parallel_safe && node.constraints.is_empty() {
                node.set_parallel_safe(true);
            }
        }
        Ok(())
    }

    /// Check if queries should be split
    fn should_split_queries(&self, plan: &QueryPlan) -> bool {
        plan.nodes.values().any(|node| node.estimated_rows > 50000)
    }

    /// Check if phases can be reordered
    fn can_reorder_phases(&self, plan: &QueryPlan) -> bool {
        plan.execution_phases.len() > 1
    }

    /// Reorder phases for better performance
    fn reorder_phases(&self, plan: &mut QueryPlan) -> OrmResult<()> {
        // Sort phases by estimated complexity (simpler phases first)
        plan.execution_phases.sort_by(|a, b| {
            let a_complexity: usize = a.iter()
                .filter_map(|id| plan.nodes.get(id))
                .map(|node| node.estimated_rows)
                .sum();
            let b_complexity: usize = b.iter()
                .filter_map(|id| plan.nodes.get(id))
                .map(|node| node.estimated_rows)
                .sum();
            
            a_complexity.cmp(&b_complexity)
        });
        
        Ok(())
    }

    /// Suggest database indexes
    fn suggest_indexes(&self, plan: &QueryPlan) -> Vec<String> {
        let mut suggestions = Vec::new();
        let mut suggested_tables = HashSet::new();
        
        for node in plan.nodes.values() {
            if !suggested_tables.contains(&node.table) {
                suggestions.push(format!("CREATE INDEX idx_{}_id ON {} (id)", node.table, node.table));
                
                if let Some(fk) = &node.foreign_key {
                    suggestions.push(format!("CREATE INDEX idx_{}_{} ON {} ({})", 
                        node.table, fk, node.table, fk));
                }
                
                suggested_tables.insert(node.table.clone());
            }
        }
        
        suggestions
    }
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}