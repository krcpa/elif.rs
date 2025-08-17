//! Connection Pool Statistics
//!
//! This module provides detailed statistics tracking for connection pools.

use std::time::Instant;
use crate::backends::DatabasePoolStats;

/// Extended pool statistics with additional metrics
#[derive(Debug, Clone)]
pub struct ExtendedPoolStats {
    pub pool_stats: DatabasePoolStats,
    pub acquire_count: u64,
    pub acquire_errors: u64,
    pub created_at: Instant,
}

impl ExtendedPoolStats {
    /// Calculate the error rate as a percentage
    pub fn error_rate(&self) -> f64 {
        if self.acquire_count > 0 {
            (self.acquire_errors as f64 / self.acquire_count as f64) * 100.0
        } else {
            0.0
        }
    }
    
    /// Get the pool uptime
    pub fn uptime(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }
    
    /// Calculate success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        100.0 - self.error_rate()
    }
    
    /// Get pool utilization as a percentage (active / total)
    pub fn utilization(&self) -> f64 {
        if self.pool_stats.total_connections > 0 {
            (self.pool_stats.active_connections as f64 / self.pool_stats.total_connections as f64) * 100.0
        } else {
            0.0
        }
    }
    
    /// Check if the pool is under stress (high utilization)
    pub fn is_under_stress(&self, threshold: f64) -> bool {
        self.utilization() > threshold
    }
    
    /// Check if the error rate is concerning
    pub fn has_concerning_error_rate(&self, threshold: f64) -> bool {
        self.error_rate() > threshold
    }
}

/// Statistics aggregator for multiple pools
#[derive(Debug, Default)]
pub struct PoolStatsAggregator {
    pools: std::collections::HashMap<String, ExtendedPoolStats>,
}

impl PoolStatsAggregator {
    pub fn new() -> Self {
        Self {
            pools: std::collections::HashMap::new(),
        }
    }
    
    /// Add or update statistics for a named pool
    pub fn update_stats(&mut self, pool_name: String, stats: ExtendedPoolStats) {
        self.pools.insert(pool_name, stats);
    }
    
    /// Get statistics for a specific pool
    pub fn get_stats(&self, pool_name: &str) -> Option<&ExtendedPoolStats> {
        self.pools.get(pool_name)
    }
    
    /// Get all pool names
    pub fn pool_names(&self) -> Vec<&String> {
        self.pools.keys().collect()
    }
    
    /// Calculate aggregate statistics across all pools
    pub fn aggregate_stats(&self) -> AggregatedStats {
        let mut total_connections = 0;
        let mut total_active = 0;
        let mut total_idle = 0;
        let mut total_acquires = 0;
        let mut total_errors = 0;
        
        for stats in self.pools.values() {
            total_connections += stats.pool_stats.total_connections;
            total_active += stats.pool_stats.active_connections;
            total_idle += stats.pool_stats.idle_connections;
            total_acquires += stats.acquire_count;
            total_errors += stats.acquire_errors;
        }
        
        let error_rate = if total_acquires > 0 {
            (total_errors as f64 / total_acquires as f64) * 100.0
        } else {
            0.0
        };
        
        let utilization = if total_connections > 0 {
            (total_active as f64 / total_connections as f64) * 100.0
        } else {
            0.0
        };
        
        AggregatedStats {
            pool_count: self.pools.len(),
            total_connections,
            total_active,
            total_idle,
            total_acquires,
            total_errors,
            error_rate,
            utilization,
        }
    }
}

/// Aggregated statistics across multiple pools
#[derive(Debug, Clone)]
pub struct AggregatedStats {
    pub pool_count: usize,
    pub total_connections: u32,
    pub total_active: u32,
    pub total_idle: u32,
    pub total_acquires: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub utilization: f64,
}