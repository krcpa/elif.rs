//! Performance testing utilities
//!
//! Provides tools for load testing, benchmarking, and performance
//! analysis of elif.rs applications.

use std::time::{Duration, Instant};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use tokio::time::sleep;
use crate::{TestResult, client::TestClient};

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Number of concurrent users/connections
    pub concurrent_users: usize,
    /// Duration of the test
    pub duration: Duration,
    /// Requests per second target (0 = unlimited)
    pub target_rps: usize,
    /// Ramp-up time to reach target concurrency
    pub ramp_up: Duration,
    /// Request timeout
    pub timeout: Duration,
}

impl LoadTestConfig {
    /// Create a basic load test configuration
    pub fn basic() -> Self {
        Self {
            concurrent_users: 10,
            duration: Duration::from_secs(30),
            target_rps: 0, // Unlimited
            ramp_up: Duration::from_secs(5),
            timeout: Duration::from_secs(30),
        }
    }
    
    /// Create a light load test
    pub fn light() -> Self {
        Self {
            concurrent_users: 5,
            duration: Duration::from_secs(10),
            target_rps: 50,
            ramp_up: Duration::from_secs(2),
            timeout: Duration::from_secs(10),
        }
    }
    
    /// Create a heavy load test
    pub fn heavy() -> Self {
        Self {
            concurrent_users: 100,
            duration: Duration::from_secs(120),
            target_rps: 1000,
            ramp_up: Duration::from_secs(30),
            timeout: Duration::from_secs(30),
        }
    }
    
    /// Set concurrent users
    pub fn with_concurrent_users(mut self, users: usize) -> Self {
        self.concurrent_users = users;
        self
    }
    
    /// Set test duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
    
    /// Set target requests per second
    pub fn with_target_rps(mut self, rps: usize) -> Self {
        self.target_rps = rps;
        self
    }
}

/// Performance test results
#[derive(Debug, Clone)]
pub struct LoadTestResults {
    /// Total requests made
    pub total_requests: usize,
    /// Successful requests
    pub successful_requests: usize,
    /// Failed requests
    pub failed_requests: usize,
    /// Test duration
    pub duration: Duration,
    /// Average requests per second
    pub avg_rps: f64,
    /// Response time statistics
    pub response_times: ResponseTimeStats,
    /// Error distribution
    pub errors: Vec<(String, usize)>,
}

impl LoadTestResults {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }
    
    /// Check if the test passed based on criteria
    pub fn passes_criteria(&self, min_success_rate: f64, max_avg_response_time: Duration) -> bool {
        self.success_rate() >= min_success_rate &&
        self.response_times.avg <= max_avg_response_time
    }
}

/// Response time statistics
#[derive(Debug, Clone)]
pub struct ResponseTimeStats {
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

impl ResponseTimeStats {
    fn new() -> Self {
        Self {
            min: Duration::ZERO,
            max: Duration::ZERO,
            avg: Duration::ZERO,
            p50: Duration::ZERO,
            p95: Duration::ZERO,
            p99: Duration::ZERO,
        }
    }
    
    fn from_times(mut times: Vec<Duration>) -> Self {
        if times.is_empty() {
            return Self::new();
        }
        
        times.sort();
        let len = times.len();
        
        let min = times[0];
        let max = times[len - 1];
        let avg = Duration::from_nanos(
            times.iter().map(|d| d.as_nanos() as u64).sum::<u64>() / len as u64
        );
        
        let p50 = times[len * 50 / 100];
        let p95 = times[len * 95 / 100];
        let p99 = times[len * 99 / 100];
        
        Self { min, max, avg, p50, p95, p99 }
    }
}

/// Load test runner
pub struct LoadTestRunner {
    config: LoadTestConfig,
    base_client: TestClient,
}

impl LoadTestRunner {
    /// Create a new load test runner
    pub fn new(config: LoadTestConfig) -> Self {
        Self {
            config,
            base_client: TestClient::new(),
        }
    }
    
    /// Set the base URL for testing
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_client = TestClient::with_base_url(url);
        self
    }
    
    /// Set authentication for all requests
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.base_client = self.base_client.authenticated_with_token(token);
        self
    }
    
    /// Run a load test with a custom scenario
    pub async fn run_scenario<F, Fut>(&self, scenario: F) -> TestResult<LoadTestResults>
    where
        F: Fn(TestClient) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = TestResult<Duration>> + Send,
    {
        let start_time = Instant::now();
        let end_time = start_time + self.config.duration;
        
        // Shared counters
        let total_requests = Arc::new(AtomicUsize::new(0));
        let successful_requests = Arc::new(AtomicUsize::new(0));
        let failed_requests = Arc::new(AtomicUsize::new(0));
        let response_times = Arc::new(tokio::sync::Mutex::new(Vec::<Duration>::new()));
        let errors = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        
        // Calculate delays for ramp-up
        let ramp_up_delay = if self.config.concurrent_users > 0 {
            self.config.ramp_up.as_millis() / self.config.concurrent_users as u128
        } else {
            0
        };
        
        let target_rps = self.config.target_rps;
        
        // Spawn worker tasks
        let mut handles = Vec::new();
        for i in 0..self.config.concurrent_users {
            let scenario = scenario.clone();
            let client = self.base_client.clone();
            let total_requests = total_requests.clone();
            let successful_requests = successful_requests.clone();
            let failed_requests = failed_requests.clone();
            let response_times = response_times.clone();
            let errors = errors.clone();
            let end_time = end_time;
            
            let handle = tokio::spawn(async move {
                // Ramp-up delay
                if ramp_up_delay > 0 {
                    sleep(Duration::from_millis((i as u128 * ramp_up_delay) as u64)).await;
                }
                
                while Instant::now() < end_time {
                    total_requests.fetch_add(1, Ordering::Relaxed);
                    
                    match scenario(client.clone()).await {
                        Ok(duration) => {
                            successful_requests.fetch_add(1, Ordering::Relaxed);
                            response_times.lock().await.push(duration);
                        },
                        Err(e) => {
                            failed_requests.fetch_add(1, Ordering::Relaxed);
                            errors.lock().await.push(e.to_string());
                        }
                    }
                    
                    // Rate limiting if specified
                    if target_rps > 0 {
                        let delay = Duration::from_millis(1000 / target_rps as u64);
                        sleep(delay).await;
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
        
        let actual_duration = start_time.elapsed();
        let total = total_requests.load(Ordering::Relaxed);
        let successful = successful_requests.load(Ordering::Relaxed);
        let failed = failed_requests.load(Ordering::Relaxed);
        
        let avg_rps = if actual_duration.as_secs() > 0 {
            total as f64 / actual_duration.as_secs_f64()
        } else {
            0.0
        };
        
        // Process response times
        let times = response_times.lock().await.clone();
        let response_time_stats = ResponseTimeStats::from_times(times);
        
        // Process errors
        let error_list = errors.lock().await.clone();
        let mut error_counts = std::collections::HashMap::new();
        for error in error_list {
            *error_counts.entry(error).or_insert(0) += 1;
        }
        let error_distribution: Vec<(String, usize)> = error_counts.into_iter().collect();
        
        Ok(LoadTestResults {
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            duration: actual_duration,
            avg_rps,
            response_times: response_time_stats,
            errors: error_distribution,
        })
    }
    
    /// Run a simple GET request load test
    pub async fn run_get_test(&self, path: impl Into<String>) -> TestResult<LoadTestResults> {
        let path = path.into();
        self.run_scenario(move |client| {
            let path = path.clone();
            async move {
                let start = Instant::now();
                client.get(path).send().await?;
                Ok(start.elapsed())
            }
        }).await
    }
    
    /// Run a POST request load test
    pub async fn run_post_test<T: serde::Serialize + Clone + Send + Sync + 'static>(
        &self, 
        path: impl Into<String>, 
        data: T
    ) -> TestResult<LoadTestResults> {
        let path = path.into();
        self.run_scenario(move |client| {
            let path = path.clone();
            let data = data.clone();
            async move {
                let start = Instant::now();
                client.post(path).json(&data).send().await?;
                Ok(start.elapsed())
            }
        }).await
    }
}

/// Benchmark utilities for micro-benchmarks
pub struct Benchmark {
    name: String,
    iterations: usize,
}

impl Benchmark {
    /// Create a new benchmark
    pub fn new(name: impl Into<String>, iterations: usize) -> Self {
        Self {
            name: name.into(),
            iterations,
        }
    }
    
    /// Run a synchronous benchmark
    pub fn run_sync<F>(&self, mut operation: F) -> BenchmarkResult
    where
        F: FnMut() -> (),
    {
        let mut times = Vec::with_capacity(self.iterations);
        
        for _ in 0..self.iterations {
            let start = Instant::now();
            operation();
            times.push(start.elapsed());
        }
        
        BenchmarkResult {
            name: self.name.clone(),
            iterations: self.iterations,
            stats: ResponseTimeStats::from_times(times),
        }
    }
    
    /// Run an async benchmark
    pub async fn run_async<F, Fut>(&self, operation: F) -> BenchmarkResult
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let mut times = Vec::with_capacity(self.iterations);
        
        for _ in 0..self.iterations {
            let start = Instant::now();
            operation().await;
            times.push(start.elapsed());
        }
        
        BenchmarkResult {
            name: self.name.clone(),
            iterations: self.iterations,
            stats: ResponseTimeStats::from_times(times),
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub stats: ResponseTimeStats,
}

impl BenchmarkResult {
    /// Get operations per second
    pub fn ops_per_second(&self) -> f64 {
        if self.stats.avg.as_nanos() > 0 {
            1_000_000_000.0 / self.stats.avg.as_nanos() as f64
        } else {
            0.0
        }
    }
    
    /// Print benchmark results
    pub fn print(&self) {
        println!("Benchmark: {}", self.name);
        println!("Iterations: {}", self.iterations);
        println!("Average time: {:?}", self.stats.avg);
        println!("Min time: {:?}", self.stats.min);
        println!("Max time: {:?}", self.stats.max);
        println!("95th percentile: {:?}", self.stats.p95);
        println!("99th percentile: {:?}", self.stats.p99);
        println!("Ops/sec: {:.2}", self.ops_per_second());
        println!("---");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_load_test_config() {
        let config = LoadTestConfig::basic()
            .with_concurrent_users(20)
            .with_duration(Duration::from_secs(60));
            
        assert_eq!(config.concurrent_users, 20);
        assert_eq!(config.duration, Duration::from_secs(60));
    }
    
    #[test]
    fn test_response_time_stats() {
        let times = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];
        
        let stats = ResponseTimeStats::from_times(times);
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(50));
        assert_eq!(stats.avg, Duration::from_millis(30));
    }
    
    #[test]
    fn test_load_test_results() {
        let results = LoadTestResults {
            total_requests: 100,
            successful_requests: 95,
            failed_requests: 5,
            duration: Duration::from_secs(10),
            avg_rps: 10.0,
            response_times: ResponseTimeStats::new(),
            errors: vec![],
        };
        
        assert_eq!(results.success_rate(), 95.0);
    }
    
    #[tokio::test]
    async fn test_benchmark() {
        let benchmark = Benchmark::new("test_operation", 100);
        
        let result = benchmark.run_sync(|| {
            // Simulate some work
            std::thread::sleep(Duration::from_micros(1));
        });
        
        assert_eq!(result.name, "test_operation");
        assert_eq!(result.iterations, 100);
        assert!(result.ops_per_second() > 0.0);
    }
    
    #[tokio::test]
    async fn test_async_benchmark() {
        let benchmark = Benchmark::new("async_test_operation", 10);
        
        let result = benchmark.run_async(|| async {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }).await;
        
        assert_eq!(result.iterations, 10);
    }
    
    #[tokio::test]
    async fn test_load_test_runner_creation() {
        let config = LoadTestConfig::light();
        let runner = LoadTestRunner::new(config.clone());
        
        assert_eq!(runner.config.concurrent_users, config.concurrent_users);
    }
}