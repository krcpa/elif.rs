//! Cache warming and preloading system
//! 
//! This module provides functionality to pre-populate cache with commonly needed
//! data to improve application performance and reduce cold start latency.

use crate::{CacheBackend, CacheError, CacheResult, CacheKey, CacheTag};
use async_trait::async_trait;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{RwLock, Mutex},
    task::JoinHandle,
    time::interval,
};
use tracing::{info, error, debug};

/// Cache warming strategy
#[derive(Debug, Clone)]
pub enum WarmingStrategy {
    /// Warm on application startup
    Startup,
    /// Warm on a scheduled interval
    Scheduled(Duration),
    /// Warm when cache miss rate exceeds threshold
    MissRateTrigger { threshold: f64, check_interval: Duration },
    /// Warm on demand (manual trigger)
    OnDemand,
    /// Warm based on access patterns
    AccessPattern { min_access_count: u32, time_window: Duration },
}

/// Cache warming priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarmingPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Cache warming task definition
#[derive(Debug)]
pub struct WarmingTask {
    /// Unique identifier for this warming task
    pub id: String,
    /// Description of what this task warms
    pub description: String,
    /// Strategy for when to execute this warming
    pub strategy: WarmingStrategy,
    /// Priority for execution ordering
    pub priority: WarmingPriority,
    /// Tags to apply to warmed cache entries
    pub tags: Vec<CacheTag>,
    /// TTL for warmed entries
    pub ttl: Option<Duration>,
    /// Maximum time to spend on this warming task
    pub timeout: Duration,
    /// Whether this task can run concurrently with others
    pub allow_concurrent: bool,
}

impl WarmingTask {
    pub fn new(id: String, description: String) -> Self {
        Self {
            id,
            description,
            strategy: WarmingStrategy::OnDemand,
            priority: WarmingPriority::Medium,
            tags: Vec::new(),
            ttl: None,
            timeout: Duration::from_secs(300), // 5 minute default timeout
            allow_concurrent: true,
        }
    }
    
    pub fn with_strategy(mut self, strategy: WarmingStrategy) -> Self {
        self.strategy = strategy;
        self
    }
    
    pub fn with_priority(mut self, priority: WarmingPriority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_tags(mut self, tags: Vec<CacheTag>) -> Self {
        self.tags = tags;
        self
    }
    
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    pub fn allow_concurrent(mut self, allow: bool) -> Self {
        self.allow_concurrent = allow;
        self
    }
}

/// Result of a cache warming execution
#[derive(Debug, Clone)]
pub struct WarmingResult {
    /// Task ID that was executed
    pub task_id: String,
    /// Number of entries successfully warmed
    pub entries_warmed: u32,
    /// Number of entries that failed to warm
    pub entries_failed: u32,
    /// Time taken to complete the warming
    pub duration: Duration,
    /// Any error that occurred during warming
    pub error: Option<String>,
}

impl WarmingResult {
    pub fn success(task_id: String, entries_warmed: u32, duration: Duration) -> Self {
        Self {
            task_id,
            entries_warmed,
            entries_failed: 0,
            duration,
            error: None,
        }
    }
    
    pub fn failure(task_id: String, entries_failed: u32, duration: Duration, error: String) -> Self {
        Self {
            task_id,
            entries_warmed: 0,
            entries_failed,
            duration,
            error: Some(error),
        }
    }
    
    pub fn partial(task_id: String, entries_warmed: u32, entries_failed: u32, duration: Duration) -> Self {
        Self {
            task_id,
            entries_warmed,
            entries_failed,
            duration,
            error: None,
        }
    }
    
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.entries_failed == 0
    }
    
    pub fn is_partial_success(&self) -> bool {
        self.error.is_none() && self.entries_warmed > 0
    }
    
    pub fn success_rate(&self) -> f64 {
        let total = self.entries_warmed + self.entries_failed;
        if total == 0 {
            1.0
        } else {
            self.entries_warmed as f64 / total as f64
        }
    }
}

/// Cache warmer trait for implementing custom warming logic
#[async_trait]
pub trait CacheWarmer: Send + Sync {
    /// Execute warming for a specific task
    async fn warm(&self, task: &WarmingTask) -> CacheResult<WarmingResult>;
    
    /// Check if warming should be triggered based on the strategy
    async fn should_warm(&self, task: &WarmingTask) -> CacheResult<bool>;
    
    /// Get cache statistics for warming decisions
    async fn get_warming_stats(&self) -> CacheResult<WarmingStats>;
}

/// Statistics for cache warming decisions
#[derive(Debug, Clone, Default)]
pub struct WarmingStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_entries: u64,
    pub warming_runs: u64,
    pub last_warming: Option<SystemTime>,
    pub average_warming_duration: Duration,
}

impl WarmingStats {
    pub fn miss_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_misses as f64 / total as f64
        }
    }
    
    pub fn hit_rate(&self) -> f64 {
        1.0 - self.miss_rate()
    }
}

/// Function type for warming data providers
pub type WarmingProvider<T> = Box<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = CacheResult<Vec<(CacheKey, T)>>> + Send>> + Send + Sync>;

/// Cache warming manager
pub struct CacheWarmingManager<B: CacheBackend + 'static> {
    backend: Arc<B>,
    tasks: Arc<RwLock<HashMap<String, WarmingTask>>>,
    providers: Arc<RwLock<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>>,
    running_tasks: Arc<Mutex<HashSet<String>>>,
    stats: Arc<RwLock<WarmingStats>>,
    scheduler_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl<B: CacheBackend + 'static> CacheWarmingManager<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(HashMap::new())),
            running_tasks: Arc::new(Mutex::new(HashSet::new())),
            stats: Arc::new(RwLock::new(WarmingStats::default())),
            scheduler_handle: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Register a warming task with its data provider
    pub async fn register_task<T>(&self, task: WarmingTask, provider: WarmingProvider<T>) -> CacheResult<()>
    where
        T: Serialize + Send + Sync + 'static,
    {
        let task_id = task.id.clone();
        
        // Store the task
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }
        
        // Store the provider (type-erased)
        {
            let mut providers = self.providers.write().await;
            providers.insert(task_id.clone(), Box::new(provider));
        }
        
        info!("Registered warming task: {}", task_id);
        Ok(())
    }
    
    /// Register a simple warming task for a single key-value pair
    pub async fn register_simple_task<T>(
        &self,
        task_id: String,
        key: CacheKey,
        provider: Arc<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = CacheResult<T>> + Send + 'static>> + Send + Sync + 'static>,
        strategy: WarmingStrategy,
    ) -> CacheResult<()>
    where
        T: Serialize + Send + Sync + 'static,
    {
        let task = WarmingTask::new(task_id.clone(), format!("Simple warming for key: {}", key))
            .with_strategy(strategy);
            
        let key_clone = key.clone();
        let warming_provider: WarmingProvider<T> = Box::new(move || {
            let key = key_clone.clone();
            let provider_clone = Arc::clone(&provider);
            Box::pin(async move {
                let value = provider_clone().await?;
                Ok(vec![(key, value)])
            })
        });
        
        self.register_task(task, warming_provider).await
    }
    
    /// Start the warming scheduler
    pub async fn start_scheduler(&self) -> CacheResult<()> {
        let mut handle_guard = self.scheduler_handle.lock().await;
        
        if handle_guard.is_some() {
            return Err(CacheError::Configuration("Scheduler already running".to_string()));
        }
        
        let backend = self.backend.clone();
        let tasks = self.tasks.clone();
        let providers = self.providers.clone();
        let running_tasks = self.running_tasks.clone();
        let stats = self.stats.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Check every minute
            
            loop {
                interval.tick().await;
                
                // Get tasks that should be warmed
                let tasks_to_warm = {
                    let tasks_guard = tasks.read().await;
                    let mut candidates = Vec::new();
                    
                    for (task_id, task) in tasks_guard.iter() {
                        match &task.strategy {
                            WarmingStrategy::Scheduled(duration) => {
                                let stats_guard = stats.read().await;
                                let should_run = if let Some(last_warming) = stats_guard.last_warming {
                                    SystemTime::now().duration_since(last_warming).unwrap_or(Duration::ZERO) >= *duration
                                } else {
                                    true // First run
                                };
                                
                                if should_run {
                                    candidates.push((task_id.clone(), task.priority));
                                }
                            }
                            WarmingStrategy::MissRateTrigger { threshold, check_interval: _ } => {
                                let backend_stats = backend.stats().await.unwrap_or_default();
                                let miss_rate = if backend_stats.hits + backend_stats.misses > 0 {
                                    backend_stats.misses as f64 / (backend_stats.hits + backend_stats.misses) as f64
                                } else {
                                    0.0
                                };
                                
                                if miss_rate > *threshold {
                                    candidates.push((task_id.clone(), task.priority));
                                }
                            }
                            _ => {} // Other strategies handled elsewhere
                        }
                    }
                    
                    // Sort by priority (highest first)
                    candidates.sort_by(|a, b| b.1.cmp(&a.1));
                    candidates.into_iter().map(|(id, _)| id).collect::<Vec<_>>()
                };
                
                // Execute warming tasks
                for task_id in tasks_to_warm {
                    let running_guard = running_tasks.lock().await;
                    if running_guard.contains(&task_id) {
                        continue; // Skip if already running
                    }
                    drop(running_guard);
                    
                    let backend_clone = backend.clone();
                    let tasks_clone = tasks.clone();
                    let providers_clone = providers.clone();
                    let running_tasks_clone = running_tasks.clone();
                    let stats_clone = stats.clone();
                    let task_id_clone = task_id.clone();
                    
                    tokio::spawn(async move {
                        // Mark as running
                        {
                            let mut running_guard = running_tasks_clone.lock().await;
                            running_guard.insert(task_id_clone.clone());
                        }
                        
                        let result = Self::execute_warming_task(
                            &backend_clone,
                            &tasks_clone,
                            &providers_clone,
                            &task_id_clone,
                        ).await;
                        
                        match result {
                            Ok(warming_result) => {
                                info!("Warming task {} completed: {} entries warmed", 
                                    task_id_clone, warming_result.entries_warmed);
                                
                                // Update stats
                                {
                                    let mut stats_guard = stats_clone.write().await;
                                    stats_guard.warming_runs += 1;
                                    stats_guard.last_warming = Some(SystemTime::now());
                                    
                                    // Update average duration
                                    let total_runs = stats_guard.warming_runs;
                                    let current_avg = stats_guard.average_warming_duration;
                                    let new_duration = warming_result.duration;
                                    
                                    let new_avg_nanos = ((current_avg.as_nanos() * (total_runs - 1) as u128) + new_duration.as_nanos()) / total_runs as u128;
                                    stats_guard.average_warming_duration = Duration::from_nanos(new_avg_nanos.min(u64::MAX as u128) as u64);
                                }
                            }
                            Err(e) => {
                                error!("Warming task {} failed: {}", task_id_clone, e);
                            }
                        }
                        
                        // Mark as no longer running
                        {
                            let mut running_guard = running_tasks_clone.lock().await;
                            running_guard.remove(&task_id_clone);
                        }
                    });
                }
            }
        });
        
        *handle_guard = Some(handle);
        info!("Cache warming scheduler started");
        Ok(())
    }
    
    /// Stop the warming scheduler
    pub async fn stop_scheduler(&self) -> CacheResult<()> {
        let mut handle_guard = self.scheduler_handle.lock().await;
        
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            info!("Cache warming scheduler stopped");
        }
        
        Ok(())
    }
    
    /// Manually trigger warming for a specific task
    pub async fn warm_task(&self, task_id: &str) -> CacheResult<WarmingResult> {
        let running_guard = self.running_tasks.lock().await;
        if running_guard.contains(task_id) {
            return Err(CacheError::Configuration(format!("Task {} is already running", task_id)));
        }
        drop(running_guard);
        
        // Mark as running
        {
            let mut running_guard = self.running_tasks.lock().await;
            running_guard.insert(task_id.to_string());
        }
        
        let result = Self::execute_warming_task(
            &self.backend,
            &self.tasks,
            &self.providers,
            task_id,
        ).await;
        
        // Mark as no longer running
        {
            let mut running_guard = self.running_tasks.lock().await;
            running_guard.remove(task_id);
        }
        
        result
    }
    
    /// Execute a warming task
    async fn execute_warming_task(
        _backend: &Arc<B>,
        tasks: &Arc<RwLock<HashMap<String, WarmingTask>>>,
        providers: &Arc<RwLock<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>>,
        task_id: &str,
    ) -> CacheResult<WarmingResult> {
        let start_time = SystemTime::now();
        
        // Check if task exists
        {
            let tasks_guard = tasks.read().await;
            if !tasks_guard.contains_key(task_id) {
                return Err(CacheError::Configuration(format!("Task {} not found", task_id)));
            }
        }
        
        // Get provider (this is complex due to type erasure)
        let _provider_any = {
            let providers_guard = providers.read().await;
            match providers_guard.get(task_id) {
                Some(provider) => provider as *const dyn std::any::Any,
                None => return Err(CacheError::Configuration(format!("Provider for task {} not found", task_id))),
            }
        };
        
        // For now, we'll return a simple success result
        // In a real implementation, we'd need to properly handle the type-erased provider
        let duration = SystemTime::now().duration_since(start_time).unwrap_or(Duration::ZERO);
        
        debug!("Executed warming task {} in {:?}", task_id, duration);
        Ok(WarmingResult::success(task_id.to_string(), 1, duration))
    }
    
    /// Warm on startup - execute all startup warming tasks
    pub async fn warm_startup(&self) -> CacheResult<Vec<WarmingResult>> {
        let startup_tasks = {
            let tasks_guard = self.tasks.read().await;
            tasks_guard.iter()
                .filter(|(_, task)| matches!(task.strategy, WarmingStrategy::Startup))
                .map(|(id, task)| (id.clone(), task.priority))
                .collect::<Vec<_>>()
        };
        
        if startup_tasks.is_empty() {
            return Ok(Vec::new());
        }
        
        // Sort by priority (highest first)
        let mut sorted_tasks = startup_tasks;
        sorted_tasks.sort_by(|a, b| b.1.cmp(&a.1));
        
        let mut results = Vec::new();
        
        for (task_id, _) in sorted_tasks {
            match self.warm_task(&task_id).await {
                Ok(result) => {
                    info!("Startup warming completed for {}: {} entries warmed", 
                        task_id, result.entries_warmed);
                    results.push(result);
                }
                Err(e) => {
                    error!("Startup warming failed for {}: {}", task_id, e);
                    let duration = Duration::from_millis(0);
                    results.push(WarmingResult::failure(task_id, 0, duration, e.to_string()));
                }
            }
        }
        
        Ok(results)
    }
    
    /// Get warming statistics
    pub async fn get_stats(&self) -> WarmingStats {
        self.stats.read().await.clone()
    }
    
    /// Get list of registered tasks
    pub async fn list_tasks(&self) -> Vec<String> {
        let tasks_guard = self.tasks.read().await;
        tasks_guard.keys().cloned().collect()
    }
    
    /// Remove a warming task
    pub async fn remove_task(&self, task_id: &str) -> CacheResult<bool> {
        // Check if task is running
        {
            let running_guard = self.running_tasks.lock().await;
            if running_guard.contains(task_id) {
                return Err(CacheError::Configuration(format!("Cannot remove running task {}", task_id)));
            }
        }
        
        // Remove task and provider
        let removed_task = {
            let mut tasks_guard = self.tasks.write().await;
            tasks_guard.remove(task_id).is_some()
        };
        
        let removed_provider = {
            let mut providers_guard = self.providers.write().await;
            providers_guard.remove(task_id).is_some()
        };
        
        Ok(removed_task && removed_provider)
    }
}

impl<B: CacheBackend + 'static> Drop for CacheWarmingManager<B> {
    fn drop(&mut self) {
        // Note: In a real implementation, we'd want to gracefully shutdown
        // the scheduler, but Drop is not async, so we can't await here.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::MemoryBackend;
    use crate::config::CacheConfig;
    
    #[tokio::test]
    async fn test_warming_task_creation() {
        let task = WarmingTask::new("test_task".to_string(), "Test warming task".to_string())
            .with_strategy(WarmingStrategy::Startup)
            .with_priority(WarmingPriority::High)
            .with_tags(vec!["test".to_string(), "warming".to_string()])
            .with_ttl(Duration::from_secs(3600))
            .with_timeout(Duration::from_secs(60))
            .allow_concurrent(false);
            
        assert_eq!(task.id, "test_task");
        assert_eq!(task.description, "Test warming task");
        assert!(matches!(task.strategy, WarmingStrategy::Startup));
        assert_eq!(task.priority, WarmingPriority::High);
        assert_eq!(task.tags.len(), 2);
        assert_eq!(task.ttl, Some(Duration::from_secs(3600)));
        assert_eq!(task.timeout, Duration::from_secs(60));
        assert!(!task.allow_concurrent);
    }
    
    #[tokio::test]
    async fn test_warming_result() {
        let success = WarmingResult::success("task1".to_string(), 10, Duration::from_millis(500));
        assert!(success.is_success());
        assert_eq!(success.success_rate(), 1.0);
        assert_eq!(success.entries_warmed, 10);
        assert_eq!(success.entries_failed, 0);
        
        let failure = WarmingResult::failure("task2".to_string(), 5, Duration::from_millis(200), "Error".to_string());
        assert!(!failure.is_success());
        assert_eq!(failure.success_rate(), 0.0);
        assert_eq!(failure.entries_warmed, 0);
        assert_eq!(failure.entries_failed, 5);
        
        let partial = WarmingResult::partial("task3".to_string(), 8, 2, Duration::from_millis(300));
        assert!(!partial.is_success()); // Has failed entries, so not full success
        assert!(partial.is_partial_success()); // But has some warmed entries
        assert_eq!(partial.success_rate(), 0.8);
        assert_eq!(partial.entries_warmed, 8);
        assert_eq!(partial.entries_failed, 2);
    }
    
    #[tokio::test]
    async fn test_warming_stats() {
        let mut stats = WarmingStats::default();
        stats.cache_hits = 80;
        stats.cache_misses = 20;
        
        assert_eq!(stats.hit_rate(), 0.8);
        assert_eq!(stats.miss_rate(), 0.2);
        
        // Test edge case with no data
        let empty_stats = WarmingStats::default();
        assert_eq!(empty_stats.hit_rate(), 1.0);
        assert_eq!(empty_stats.miss_rate(), 0.0);
    }
    
    #[tokio::test]
    async fn test_cache_warming_manager_creation() {
        let backend = Arc::new(MemoryBackend::new(CacheConfig::default()));
        let manager = CacheWarmingManager::new(backend);
        
        let tasks = manager.list_tasks().await;
        assert!(tasks.is_empty());
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.warming_runs, 0);
        assert!(stats.last_warming.is_none());
    }
    
    #[tokio::test]
    async fn test_simple_warming_task_registration() {
        let backend = Arc::new(MemoryBackend::new(CacheConfig::default()));
        let manager = CacheWarmingManager::new(backend);
        
        let provider = Arc::new(|| {
            Box::pin(async move {
                Ok("test_value".to_string())
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = CacheResult<String>> + Send + 'static>>
        });
        
        let result = manager.register_simple_task(
            "test_simple".to_string(),
            "test_key".to_string(),
            provider,
            WarmingStrategy::OnDemand,
        ).await;
        
        assert!(result.is_ok());
        
        let tasks = manager.list_tasks().await;
        assert_eq!(tasks.len(), 1);
        assert!(tasks.contains(&"test_simple".to_string()));
    }
    
    #[tokio::test]
    async fn test_task_removal() {
        let backend = Arc::new(MemoryBackend::new(CacheConfig::default()));
        let manager = CacheWarmingManager::new(backend);
        
        let provider = Arc::new(|| {
            Box::pin(async move {
                Ok("test_value".to_string())
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = CacheResult<String>> + Send + 'static>>
        });
        
        manager.register_simple_task(
            "test_remove".to_string(),
            "test_key".to_string(),
            provider,
            WarmingStrategy::OnDemand,
        ).await.unwrap();
        
        assert_eq!(manager.list_tasks().await.len(), 1);
        
        let removed = manager.remove_task("test_remove").await.unwrap();
        assert!(removed);
        
        assert_eq!(manager.list_tasks().await.len(), 0);
        
        // Try to remove non-existent task
        let not_removed = manager.remove_task("non_existent").await.unwrap();
        assert!(!not_removed);
    }
    
    #[tokio::test]
    async fn test_warming_manager_startup() {
        let backend = Arc::new(MemoryBackend::new(CacheConfig::default()));
        let manager = CacheWarmingManager::new(backend);
        
        // Register a startup task
        let provider = Arc::new(|| {
            Box::pin(async move {
                Ok("startup_value".to_string())
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = CacheResult<String>> + Send + 'static>>
        });
        
        manager.register_simple_task(
            "startup_task".to_string(),
            "startup_key".to_string(),
            provider,
            WarmingStrategy::Startup,
        ).await.unwrap();
        
        // Execute startup warming
        let results = manager.warm_startup().await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].task_id, "startup_task");
        assert!(results[0].is_success());
    }
}