//! Connection Pool Health Monitoring
//!
//! This module provides comprehensive health monitoring and reporting
//! for database connection pools.

use std::time::{Duration, Instant};

/// Detailed pool health report
#[derive(Debug, Clone)]
pub struct PoolHealthReport {
    pub check_duration: Duration,
    pub total_check_time: Duration,
    pub pool_size: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
    pub total_acquires: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub created_at: Instant,
}

impl PoolHealthReport {
    /// Check if the pool is healthy based on standard metrics
    pub fn is_healthy(&self) -> bool {
        self.is_responsive() && self.has_acceptable_error_rate() && self.has_available_connections()
    }
    
    /// Check if the pool is responsive (health check completed quickly)
    pub fn is_responsive(&self) -> bool {
        self.check_duration < Duration::from_millis(1000) // 1 second threshold
    }
    
    /// Check if the error rate is acceptable
    pub fn has_acceptable_error_rate(&self) -> bool {
        self.error_rate < 5.0 // 5% error rate threshold
    }
    
    /// Check if there are connections available
    pub fn has_available_connections(&self) -> bool {
        self.idle_connections > 0 || self.active_connections < self.pool_size
    }
    
    /// Get pool utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.pool_size > 0 {
            (self.active_connections as f64 / self.pool_size as f64) * 100.0
        } else {
            0.0
        }
    }
    
    /// Get success rate percentage
    pub fn success_rate(&self) -> f64 {
        100.0 - self.error_rate
    }
    
    /// Get uptime since pool creation
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    /// Generate health status summary
    pub fn status_summary(&self) -> HealthStatus {
        if !self.is_responsive() {
            HealthStatus::Unhealthy {
                reason: "Pool is not responsive".to_string(),
            }
        } else if !self.has_acceptable_error_rate() {
            HealthStatus::Degraded {
                reason: format!("High error rate: {:.1}%", self.error_rate),
                severity: if self.error_rate > 25.0 { Severity::High } else { Severity::Medium },
            }
        } else if !self.has_available_connections() {
            HealthStatus::Degraded {
                reason: "Pool is exhausted".to_string(),
                severity: Severity::High,
            }
        } else if self.utilization() > 80.0 {
            HealthStatus::Warning {
                reason: format!("High utilization: {:.1}%", self.utilization()),
            }
        } else {
            HealthStatus::Healthy
        }
    }
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    /// Pool is operating normally
    Healthy,
    /// Pool has minor issues but is functional
    Warning { reason: String },
    /// Pool has significant issues affecting performance
    Degraded { reason: String, severity: Severity },
    /// Pool is not operational
    Unhealthy { reason: String },
}

/// Issue severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "HEALTHY"),
            HealthStatus::Warning { reason } => write!(f, "WARNING: {}", reason),
            HealthStatus::Degraded { reason, severity } => {
                write!(f, "DEGRADED ({:?}): {}", severity, reason)
            },
            HealthStatus::Unhealthy { reason } => write!(f, "UNHEALTHY: {}", reason),
        }
    }
}

/// Health monitoring configuration
#[derive(Debug, Clone)]
pub struct HealthMonitorConfig {
    /// Maximum acceptable response time for health checks
    pub max_response_time: Duration,
    /// Maximum acceptable error rate (percentage)
    pub max_error_rate: f64,
    /// Utilization threshold for warnings (percentage)
    pub warning_utilization_threshold: f64,
    /// How often to perform health checks
    pub check_interval: Duration,
    /// Enable detailed health reports
    pub detailed_reports: bool,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            max_response_time: Duration::from_millis(1000),
            max_error_rate: 5.0,
            warning_utilization_threshold: 80.0,
            check_interval: Duration::from_secs(30),
            detailed_reports: true,
        }
    }
}

/// Health monitor for tracking pool health over time
pub struct HealthMonitor {
    config: HealthMonitorConfig,
    last_check: Option<Instant>,
    history: Vec<PoolHealthReport>,
    max_history: usize,
}

impl HealthMonitor {
    pub fn new(config: HealthMonitorConfig) -> Self {
        Self {
            config,
            last_check: None,
            history: Vec::new(),
            max_history: 100, // Keep last 100 health reports
        }
    }
    
    /// Record a health report
    pub fn record_health_report(&mut self, report: PoolHealthReport) {
        self.last_check = Some(Instant::now());
        self.history.push(report);
        
        // Keep only the most recent reports
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }
    
    /// Get the latest health report
    pub fn latest_report(&self) -> Option<&PoolHealthReport> {
        self.history.last()
    }
    
    /// Get health trend over the last N reports
    pub fn health_trend(&self, count: usize) -> HealthTrend {
        let recent_reports = self.history.iter()
            .rev()
            .take(count)
            .collect::<Vec<_>>();
        
        if recent_reports.is_empty() {
            return HealthTrend::Unknown;
        }
        
        let avg_error_rate = recent_reports.iter()
            .map(|r| r.error_rate)
            .sum::<f64>() / recent_reports.len() as f64;
        
        let avg_utilization = recent_reports.iter()
            .map(|r| r.utilization())
            .sum::<f64>() / recent_reports.len() as f64;
        
        if avg_error_rate > self.config.max_error_rate {
            HealthTrend::Degrading
        } else if avg_utilization > self.config.warning_utilization_threshold {
            HealthTrend::Warning
        } else {
            HealthTrend::Stable
        }
    }
    
    /// Check if it's time for the next health check
    pub fn should_check_health(&self) -> bool {
        match self.last_check {
            Some(last) => last.elapsed() >= self.config.check_interval,
            None => true,
        }
    }
}

/// Health trend indicators
#[derive(Debug, Clone, PartialEq)]
pub enum HealthTrend {
    Unknown,
    Improving,
    Stable,
    Warning,
    Degrading,
}