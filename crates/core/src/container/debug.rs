use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fmt::Write;

use crate::container::ioc_container::IocContainer;

/// Container inspection utilities
#[derive(Debug)]
pub struct ContainerInspector {
    container: Arc<IocContainer>,
    resolution_tracer: Arc<Mutex<ResolutionTracer>>,
    performance_profiler: Arc<Mutex<PerformanceProfiler>>,
}

impl ContainerInspector {
    /// Create a new container inspector
    pub fn new(container: Arc<IocContainer>) -> Self {
        Self {
            container,
            resolution_tracer: Arc::new(Mutex::new(ResolutionTracer::new())),
            performance_profiler: Arc::new(Mutex::new(PerformanceProfiler::new())),
        }
    }
    
    /// Get basic container information
    pub fn get_container_info(&self) -> ContainerInfo {
        let stats = self.container.get_statistics();
        let services = self.container.get_registered_services();
        
        ContainerInfo {
            is_built: self.container.is_built(),
            service_count: stats.total_services,
            singleton_count: stats.singleton_services,
            scoped_count: stats.scoped_services,
            transient_count: stats.transient_services,
            cached_instances: stats.cached_instances,
            registered_services: services,
        }
    }
    
    /// Get detailed service information
    pub fn inspect_service<T: 'static>(&self) -> Option<ServiceInfo> {
        self.container.get_service_info::<T>()
            .map(|info| ServiceInfo {
                type_name: std::any::type_name::<T>().to_string(),
                registration_info: info,
                is_registered: self.container.contains::<T>(),
                resolution_count: self.get_resolution_count(std::any::type_name::<T>()),
                last_resolved: self.get_last_resolution_time(std::any::type_name::<T>()),
                average_resolution_time: self.get_average_resolution_time(std::any::type_name::<T>()),
            })
    }
    
    /// Get resolution statistics for all services
    pub fn get_resolution_stats(&self) -> HashMap<String, ResolutionStats> {
        if let Ok(tracer) = self.resolution_tracer.lock() {
            tracer.get_all_stats()
        } else {
            HashMap::new()
        }
    }
    
    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        if let Ok(profiler) = self.performance_profiler.lock() {
            profiler.get_metrics()
        } else {
            PerformanceMetrics::default()
        }
    }
    
    /// Enable/disable resolution tracing
    pub fn set_tracing_enabled(&self, enabled: bool) {
        if let Ok(mut tracer) = self.resolution_tracer.lock() {
            tracer.set_enabled(enabled);
        }
    }
    
    /// Enable/disable performance profiling
    pub fn set_profiling_enabled(&self, enabled: bool) {
        if let Ok(mut profiler) = self.performance_profiler.lock() {
            profiler.set_enabled(enabled);
        }
    }
    
    /// Clear all tracing and profiling data
    pub fn clear_debug_data(&self) {
        if let Ok(mut tracer) = self.resolution_tracer.lock() {
            tracer.clear();
        }
        if let Ok(mut profiler) = self.performance_profiler.lock() {
            profiler.clear();
        }
    }
    
    /// Generate a comprehensive debug report
    pub fn generate_debug_report(&self) -> String {
        let mut report = String::new();
        
        writeln!(report, "Container Debug Report").unwrap();
        writeln!(report, "====================").unwrap();
        writeln!(report, "Generated at: {:?}", std::time::SystemTime::now()).unwrap();
        writeln!(report, "").unwrap();
        
        // Container info
        let info = self.get_container_info();
        writeln!(report, "Container Information:").unwrap();
        writeln!(report, "---------------------").unwrap();
        writeln!(report, "Built: {}", info.is_built).unwrap();
        writeln!(report, "Total Services: {}", info.service_count).unwrap();
        writeln!(report, "  - Singletons: {}", info.singleton_count).unwrap();
        writeln!(report, "  - Scoped: {}", info.scoped_count).unwrap();
        writeln!(report, "  - Transient: {}", info.transient_count).unwrap();
        writeln!(report, "Cached Instances: {}", info.cached_instances).unwrap();
        writeln!(report, "").unwrap();
        
        // Resolution statistics
        let stats = self.get_resolution_stats();
        if !stats.is_empty() {
            writeln!(report, "Resolution Statistics:").unwrap();
            writeln!(report, "---------------------").unwrap();
            
            let mut sorted_stats: Vec<_> = stats.iter().collect();
            sorted_stats.sort_by_key(|(_, stat)| std::cmp::Reverse(stat.total_resolutions));
            
            for (service, stat) in sorted_stats.iter().take(10) {
                writeln!(report, "{}: {} resolutions, avg {:.2}ms", 
                    service, 
                    stat.total_resolutions,
                    stat.average_duration_ms
                ).unwrap();
            }
            writeln!(report, "").unwrap();
        }
        
        // Performance metrics
        let metrics = self.get_performance_metrics();
        writeln!(report, "Performance Metrics:").unwrap();
        writeln!(report, "-------------------").unwrap();
        writeln!(report, "Total Resolution Time: {:.2}ms", metrics.total_resolution_time_ms).unwrap();
        writeln!(report, "Average Resolution Time: {:.2}ms", metrics.average_resolution_time_ms).unwrap();
        writeln!(report, "Slowest Resolution: {:.2}ms ({})", 
            metrics.slowest_resolution_ms, 
            metrics.slowest_service.as_deref().unwrap_or("Unknown")
        ).unwrap();
        writeln!(report, "Memory Usage (estimated): {} bytes", metrics.estimated_memory_usage).unwrap();
        writeln!(report, "").unwrap();
        
        // Registered services
        writeln!(report, "Registered Services:").unwrap();
        writeln!(report, "-------------------").unwrap();
        for (i, service) in info.registered_services.iter().enumerate() {
            writeln!(report, "{}. {}", i + 1, service).unwrap();
        }
        
        report
    }
    
    /// Get resolution count for a service
    fn get_resolution_count(&self, service_name: &str) -> usize {
        if let Ok(tracer) = self.resolution_tracer.lock() {
            tracer.get_resolution_count(service_name)
        } else {
            0
        }
    }
    
    /// Get last resolution time for a service
    fn get_last_resolution_time(&self, service_name: &str) -> Option<Instant> {
        if let Ok(tracer) = self.resolution_tracer.lock() {
            tracer.get_last_resolution_time(service_name)
        } else {
            None
        }
    }
    
    /// Get average resolution time for a service
    fn get_average_resolution_time(&self, service_name: &str) -> Option<Duration> {
        if let Ok(tracer) = self.resolution_tracer.lock() {
            tracer.get_average_resolution_time(service_name)
        } else {
            None
        }
    }
}

/// Container information for inspection
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub is_built: bool,
    pub service_count: usize,
    pub singleton_count: usize,
    pub scoped_count: usize,
    pub transient_count: usize,
    pub cached_instances: usize,
    pub registered_services: Vec<String>,
}

/// Service information for inspection
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub type_name: String,
    pub registration_info: String,
    pub is_registered: bool,
    pub resolution_count: usize,
    pub last_resolved: Option<Instant>,
    pub average_resolution_time: Option<Duration>,
}

/// Resolution tracing for debugging dependency resolution
#[derive(Debug)]
pub struct ResolutionTracer {
    enabled: bool,
    traces: HashMap<String, Vec<ResolutionTrace>>,
    stats: HashMap<String, ResolutionStats>,
}

impl ResolutionTracer {
    /// Create a new resolution tracer
    pub fn new() -> Self {
        Self {
            enabled: false,
            traces: HashMap::new(),
            stats: HashMap::new(),
        }
    }
    
    /// Enable or disable tracing
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Record the start of a service resolution
    pub fn start_resolution(&mut self, service_name: &str) -> Option<ResolutionToken> {
        if !self.enabled {
            return None;
        }
        
        Some(ResolutionToken {
            service_name: service_name.to_string(),
            start_time: Instant::now(),
            depth: 0, // This would be calculated based on call stack
        })
    }
    
    /// Record the completion of a service resolution
    pub fn complete_resolution(&mut self, token: ResolutionToken, success: bool, error: Option<String>) {
        if !self.enabled {
            return;
        }
        
        let duration = token.start_time.elapsed();
        
        // Record trace
        let trace = ResolutionTrace {
            service_name: token.service_name.clone(),
            start_time: token.start_time,
            duration,
            success,
            error,
            depth: token.depth,
        };
        
        self.traces.entry(token.service_name.clone())
            .or_default()
            .push(trace);
        
        // Update statistics
        let stats = self.stats.entry(token.service_name.clone())
            .or_insert_with(|| ResolutionStats::new(token.service_name.clone()));
        
        stats.record_resolution(duration, success);
    }
    
    /// Get resolution traces for a service
    pub fn get_traces(&self, service_name: &str) -> Vec<&ResolutionTrace> {
        self.traces.get(service_name)
            .map(|traces| traces.iter().collect())
            .unwrap_or_default()
    }
    
    /// Get resolution statistics for a service
    pub fn get_stats(&self, service_name: &str) -> Option<&ResolutionStats> {
        self.stats.get(service_name)
    }
    
    /// Get all resolution statistics
    pub fn get_all_stats(&self) -> HashMap<String, ResolutionStats> {
        self.stats.clone()
    }
    
    /// Clear all tracing data
    pub fn clear(&mut self) {
        self.traces.clear();
        self.stats.clear();
    }
    
    /// Get resolution count for a service
    pub fn get_resolution_count(&self, service_name: &str) -> usize {
        self.stats.get(service_name)
            .map(|s| s.total_resolutions)
            .unwrap_or(0)
    }
    
    /// Get last resolution time
    pub fn get_last_resolution_time(&self, service_name: &str) -> Option<Instant> {
        self.traces.get(service_name)?
            .last()
            .map(|trace| trace.start_time)
    }
    
    /// Get average resolution time
    pub fn get_average_resolution_time(&self, service_name: &str) -> Option<Duration> {
        self.stats.get(service_name)
            .map(|s| Duration::from_nanos((s.average_duration_ms * 1_000_000.0) as u64))
    }
}

impl Default for ResolutionTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// Token representing an ongoing service resolution
#[derive(Debug)]
pub struct ResolutionToken {
    service_name: String,
    start_time: Instant,
    depth: usize,
}

/// Trace record for a single service resolution
#[derive(Debug, Clone)]
pub struct ResolutionTrace {
    pub service_name: String,
    pub start_time: Instant,
    pub duration: Duration,
    pub success: bool,
    pub error: Option<String>,
    pub depth: usize,
}

/// Resolution statistics for a service
#[derive(Debug, Clone)]
pub struct ResolutionStats {
    pub service_name: String,
    pub total_resolutions: usize,
    pub successful_resolutions: usize,
    pub failed_resolutions: usize,
    pub total_duration_ms: f64,
    pub average_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub last_resolution: Option<Instant>,
}

impl ResolutionStats {
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            total_resolutions: 0,
            successful_resolutions: 0,
            failed_resolutions: 0,
            total_duration_ms: 0.0,
            average_duration_ms: 0.0,
            min_duration_ms: f64::MAX,
            max_duration_ms: 0.0,
            last_resolution: None,
        }
    }
    
    pub fn record_resolution(&mut self, duration: Duration, success: bool) {
        let duration_ms = duration.as_secs_f64() * 1000.0;
        
        self.total_resolutions += 1;
        if success {
            self.successful_resolutions += 1;
        } else {
            self.failed_resolutions += 1;
        }
        
        self.total_duration_ms += duration_ms;
        self.average_duration_ms = self.total_duration_ms / self.total_resolutions as f64;
        
        self.min_duration_ms = self.min_duration_ms.min(duration_ms);
        self.max_duration_ms = self.max_duration_ms.max(duration_ms);
        
        self.last_resolution = Some(Instant::now());
    }
}

/// Performance profiler for container operations
#[derive(Debug)]
pub struct PerformanceProfiler {
    enabled: bool,
    resolution_times: Vec<(String, Duration)>,
    memory_snapshots: Vec<(Instant, usize)>,
    start_time: Option<Instant>,
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new() -> Self {
        Self {
            enabled: false,
            resolution_times: Vec::new(),
            memory_snapshots: Vec::new(),
            start_time: None,
        }
    }
    
    /// Enable or disable profiling
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled && self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
    }
    
    /// Record a service resolution time
    pub fn record_resolution_time(&mut self, service_name: &str, duration: Duration) {
        if self.enabled {
            self.resolution_times.push((service_name.to_string(), duration));
        }
    }
    
    /// Record a memory snapshot
    pub fn record_memory_snapshot(&mut self, memory_usage: usize) {
        if self.enabled {
            self.memory_snapshots.push((Instant::now(), memory_usage));
        }
    }
    
    /// Get performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        if self.resolution_times.is_empty() {
            return PerformanceMetrics::default();
        }
        
        let total_time: Duration = self.resolution_times.iter()
            .map(|(_, duration)| *duration)
            .sum();
        
        let avg_time = total_time / self.resolution_times.len() as u32;
        
        let (slowest_service, slowest_time) = self.resolution_times.iter()
            .max_by_key(|(_, duration)| *duration)
            .map(|(name, duration)| (name.clone(), *duration))
            .unwrap_or_else(|| ("Unknown".to_string(), Duration::default()));
        
        let estimated_memory = self.memory_snapshots.last()
            .map(|(_, memory)| *memory)
            .unwrap_or(0);
        
        PerformanceMetrics {
            total_resolution_time_ms: total_time.as_secs_f64() * 1000.0,
            average_resolution_time_ms: avg_time.as_secs_f64() * 1000.0,
            slowest_resolution_ms: slowest_time.as_secs_f64() * 1000.0,
            slowest_service: Some(slowest_service),
            total_resolutions: self.resolution_times.len(),
            estimated_memory_usage: estimated_memory,
            profiling_duration: self.start_time.map(|start| start.elapsed()),
        }
    }
    
    /// Get slowest resolutions
    pub fn get_slowest_resolutions(&self, count: usize) -> Vec<(String, Duration)> {
        let mut sorted = self.resolution_times.clone();
        sorted.sort_by_key(|(_, duration)| std::cmp::Reverse(*duration));
        sorted.into_iter().take(count).collect()
    }
    
    /// Get most frequent resolutions
    pub fn get_most_frequent(&self, count: usize) -> Vec<(String, usize)> {
        let mut frequency: HashMap<String, usize> = HashMap::new();
        
        for (service_name, _) in &self.resolution_times {
            *frequency.entry(service_name.clone()).or_insert(0) += 1;
        }
        
        let mut sorted: Vec<_> = frequency.into_iter().collect();
        sorted.sort_by_key(|(_, freq)| std::cmp::Reverse(*freq));
        sorted.into_iter().take(count).collect()
    }
    
    /// Clear all profiling data
    pub fn clear(&mut self) {
        self.resolution_times.clear();
        self.memory_snapshots.clear();
        self.start_time = if self.enabled { Some(Instant::now()) } else { None };
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics summary
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub total_resolution_time_ms: f64,
    pub average_resolution_time_ms: f64,
    pub slowest_resolution_ms: f64,
    pub slowest_service: Option<String>,
    pub total_resolutions: usize,
    pub estimated_memory_usage: usize,
    pub profiling_duration: Option<Duration>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_resolution_time_ms: 0.0,
            average_resolution_time_ms: 0.0,
            slowest_resolution_ms: 0.0,
            slowest_service: None,
            total_resolutions: 0,
            estimated_memory_usage: 0,
            profiling_duration: None,
        }
    }
}

/// Health check system for container
pub struct ContainerHealthChecker {
    container: Arc<IocContainer>,
    checks: Vec<Box<dyn HealthCheck>>,
}

impl ContainerHealthChecker {
    /// Create a new health checker
    pub fn new(container: Arc<IocContainer>) -> Self {
        let mut checker = Self {
            container,
            checks: Vec::new(),
        };
        
        // Add default health checks
        checker.add_check(Box::new(CircularDependencyCheck));
        checker.add_check(Box::new(MemoryUsageCheck { max_memory_mb: 512 }));
        checker.add_check(Box::new(SingletonHealthCheck));
        
        checker
    }
    
    /// Add a custom health check
    pub fn add_check(&mut self, check: Box<dyn HealthCheck>) {
        self.checks.push(check);
    }
    
    /// Run all health checks
    pub async fn check_health(&self) -> HealthReport {
        let mut results = Vec::new();
        let mut overall_status = HealthStatus::Healthy;
        
        for check in &self.checks {
            let result = check.check(&self.container).await;
            
            // Update overall status
            match result.status {
                HealthStatus::Unhealthy => overall_status = HealthStatus::Unhealthy,
                HealthStatus::Warning if overall_status == HealthStatus::Healthy => {
                    overall_status = HealthStatus::Warning;
                },
                _ => {}
            }
            
            results.push(result);
        }
        
        HealthReport {
            overall_status,
            checks: results,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Health check trait
pub trait HealthCheck: Send + Sync {
    /// Run the health check
    fn check(&self, container: &IocContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheckResult> + Send + '_>>;
    
    /// Get the name of this health check
    fn name(&self) -> &str;
    
    /// Get the description of what this check does
    fn description(&self) -> &str;
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub details: Option<String>,
    pub duration: Duration,
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Unhealthy,
}

/// Overall health report
#[derive(Debug)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheckResult>,
    pub timestamp: std::time::SystemTime,
}

impl HealthReport {
    /// Generate a human-readable health report
    pub fn to_string(&self) -> String {
        let mut report = String::new();
        
        writeln!(report, "Container Health Report").unwrap();
        writeln!(report, "======================").unwrap();
        writeln!(report, "Timestamp: {:?}", self.timestamp).unwrap();
        writeln!(report, "Overall Status: {:?}", self.overall_status).unwrap();
        writeln!(report, "").unwrap();
        
        let status_symbol = match self.overall_status {
            HealthStatus::Healthy => "✅",
            HealthStatus::Warning => "⚠️ ",
            HealthStatus::Unhealthy => "❌",
        };
        
        writeln!(report, "{} Container is {:?}", status_symbol, self.overall_status).unwrap();
        writeln!(report, "").unwrap();
        
        writeln!(report, "Individual Checks:").unwrap();
        writeln!(report, "------------------").unwrap();
        
        for check in &self.checks {
            let symbol = match check.status {
                HealthStatus::Healthy => "✅",
                HealthStatus::Warning => "⚠️ ",
                HealthStatus::Unhealthy => "❌",
            };
            
            writeln!(report, "{} {}: {}", symbol, check.name, check.message).unwrap();
            if let Some(details) = &check.details {
                writeln!(report, "   Details: {}", details).unwrap();
            }
            writeln!(report, "   Duration: {:.2}ms", check.duration.as_secs_f64() * 1000.0).unwrap();
            writeln!(report, "").unwrap();
        }
        
        report
    }
}

/// Circular dependency health check
struct CircularDependencyCheck;

impl HealthCheck for CircularDependencyCheck {
    fn check(&self, container: &IocContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheckResult> + Send + '_>> {
        let start = Instant::now();
        let name = self.name().to_string();
        
        match container.validate() {
            Ok(()) => Box::pin(async move {
                HealthCheckResult {
                    name,
                    status: HealthStatus::Healthy,
                    message: "No circular dependencies detected".to_string(),
                    details: None,
                    duration: start.elapsed(),
                }
            }),
            Err(e) => Box::pin(async move {
                HealthCheckResult {
                    name,
                    status: HealthStatus::Unhealthy,
                    message: "Circular dependency detected".to_string(),
                    details: Some(e.to_string()),
                    duration: start.elapsed(),
                }
            }),
        }
    }
    
    fn name(&self) -> &str {
        "Circular Dependency Check"
    }
    
    fn description(&self) -> &str {
        "Checks for circular dependencies in the service graph"
    }
}

/// Memory usage health check
struct MemoryUsageCheck {
    max_memory_mb: usize,
}

impl HealthCheck for MemoryUsageCheck {
    fn check(&self, _container: &IocContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheckResult> + Send + '_>> {
        let start = Instant::now();
        let name = self.name().to_string();
        let max_memory_mb = self.max_memory_mb;
        
        Box::pin(async move {
            // This is a simplified memory check - in a real implementation,
            // you'd get actual memory usage from the system
            let estimated_memory = 64; // MB
            
            let status = if estimated_memory > max_memory_mb {
                HealthStatus::Unhealthy
            } else if estimated_memory > max_memory_mb / 2 {
                HealthStatus::Warning
            } else {
                HealthStatus::Healthy
            };
            
            HealthCheckResult {
                name,
                status,
                message: format!("Memory usage: {} MB", estimated_memory),
                details: Some(format!("Limit: {} MB", max_memory_mb)),
                duration: start.elapsed(),
            }
        })
    }
    
    fn name(&self) -> &str {
        "Memory Usage Check"
    }
    
    fn description(&self) -> &str {
        "Monitors container memory usage"
    }
}

/// Singleton health check
struct SingletonHealthCheck;

impl HealthCheck for SingletonHealthCheck {
    fn check(&self, container: &IocContainer) -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheckResult> + Send + '_>> {
        let start = Instant::now();
        let stats = container.get_statistics();
        let name = self.name().to_string();
        
        Box::pin(async move {
            // Check if there are too many singletons (potential memory issues)
            let singleton_ratio = if stats.total_services > 0 {
                stats.singleton_services as f64 / stats.total_services as f64
            } else {
                0.0
            };
            
            let status = if singleton_ratio > 0.8 {
                HealthStatus::Warning
            } else {
                HealthStatus::Healthy
            };
            
            HealthCheckResult {
                name,
                status,
                message: format!("Singleton ratio: {:.1}%", singleton_ratio * 100.0),
                details: Some(format!("{} singletons out of {} total services", 
                    stats.singleton_services, stats.total_services)),
                duration: start.elapsed(),
            }
        })
    }
    
    fn name(&self) -> &str {
        "Singleton Health Check"
    }
    
    fn description(&self) -> &str {
        "Monitors singleton service ratio"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ioc_container::IocContainer;
    use std::time::Duration;

    #[test]
    fn test_resolution_tracer() {
        let mut tracer = ResolutionTracer::new();
        tracer.set_enabled(true);
        
        let token = tracer.start_resolution("TestService").unwrap();
        std::thread::sleep(Duration::from_millis(1)); // Small delay for testing
        tracer.complete_resolution(token, true, None);
        
        let stats = tracer.get_stats("TestService").unwrap();
        assert_eq!(stats.total_resolutions, 1);
        assert_eq!(stats.successful_resolutions, 1);
        assert!(stats.average_duration_ms > 0.0);
    }

    #[test]
    fn test_performance_profiler() {
        let mut profiler = PerformanceProfiler::new();
        profiler.set_enabled(true);
        
        profiler.record_resolution_time("Service1", Duration::from_millis(10));
        profiler.record_resolution_time("Service2", Duration::from_millis(5));
        profiler.record_resolution_time("Service1", Duration::from_millis(15));
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.total_resolutions, 3);
        assert_eq!(metrics.total_resolution_time_ms, 30.0);
        assert_eq!(metrics.average_resolution_time_ms, 10.0);
        
        let slowest = profiler.get_slowest_resolutions(2);
        assert_eq!(slowest.len(), 2);
        assert_eq!(slowest[0].1, Duration::from_millis(15));
        
        let frequent = profiler.get_most_frequent(2);
        assert_eq!(frequent.len(), 2);
        assert_eq!(frequent[0].1, 2); // Service1 appears twice
    }

    #[test]
    fn test_container_inspector() {
        let container = IocContainer::new();
        let inspector = ContainerInspector::new(Arc::new(container));
        
        let info = inspector.get_container_info();
        assert_eq!(info.service_count, 0);
        assert!(!info.is_built);
        
        let report = inspector.generate_debug_report();
        assert!(report.contains("Container Debug Report"));
        assert!(report.contains("Container Information"));
    }

    #[tokio::test]
    async fn test_health_checker() {
        let container = IocContainer::new();
        let health_checker = ContainerHealthChecker::new(Arc::new(container));
        
        let report = health_checker.check_health().await;
        
        // Should have at least the default health checks
        assert!(!report.checks.is_empty());
        
        let report_str = report.to_string();
        assert!(report_str.contains("Container Health Report"));
        assert!(report_str.contains("Overall Status"));
    }

    #[test]
    fn test_resolution_stats() {
        let mut stats = ResolutionStats::new("TestService".to_string());
        
        stats.record_resolution(Duration::from_millis(10), true);
        stats.record_resolution(Duration::from_millis(20), true);
        stats.record_resolution(Duration::from_millis(5), false);
        
        assert_eq!(stats.total_resolutions, 3);
        assert_eq!(stats.successful_resolutions, 2);
        assert_eq!(stats.failed_resolutions, 1);
        assert_eq!(stats.average_duration_ms, (10.0 + 20.0 + 5.0) / 3.0);
        assert_eq!(stats.min_duration_ms, 5.0);
        assert_eq!(stats.max_duration_ms, 20.0);
    }
}