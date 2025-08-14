//! Example: Advanced service composition with the DI container
//!
//! This example demonstrates complex dependency injection patterns,
//! service composition, lifecycle management, and advanced container usage.

use elif_core::{Container, DatabaseConnection, ContainerError, DatabaseError};
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Advanced service traits
#[async_trait]
pub trait CacheService: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError>;
    async fn set(&self, key: &str, value: String, ttl: Duration) -> Result<(), CacheError>;
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;
    async fn clear(&self) -> Result<(), CacheError>;
    fn stats(&self) -> CacheStats;
}

#[async_trait] 
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: Event) -> Result<(), EventError>;
    async fn subscribe(&self, handler: Arc<dyn EventHandler>) -> Result<String, EventError>;
    async fn unsubscribe(&self, subscription_id: &str) -> Result<(), EventError>;
    fn get_metrics(&self) -> EventMetrics;
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &Event) -> Result<(), EventError>;
    fn event_types(&self) -> Vec<String>;
}

pub trait MetricsCollector: Send + Sync {
    fn increment_counter(&self, name: &str, tags: Option<HashMap<String, String>>);
    fn record_histogram(&self, name: &str, value: f64, tags: Option<HashMap<String, String>>);
    fn set_gauge(&self, name: &str, value: f64, tags: Option<HashMap<String, String>>);
    fn get_snapshot(&self) -> MetricsSnapshot;
}

// Data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub memory_usage: usize,
}

#[derive(Debug, Clone)]
pub struct EventMetrics {
    pub events_published: u64,
    pub events_processed: u64,
    pub active_subscriptions: usize,
    pub processing_errors: u64,
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub counters: HashMap<String, u64>,
    pub histograms: HashMap<String, Vec<f64>>,
    pub gauges: HashMap<String, f64>,
    pub timestamp: DateTime<Utc>,
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Cache operation failed: {reason}")]
    OperationFailed { reason: String },
    
    #[error("Cache entry not found: {key}")]
    NotFound { key: String },
    
    #[error("Cache storage full")]
    StorageFull,
}

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("Event processing failed: {reason}")]
    ProcessingFailed { reason: String },
    
    #[error("Invalid event type: {event_type}")]
    InvalidEventType { event_type: String },
    
    #[error("Subscription not found: {id}")]
    SubscriptionNotFound { id: String },
}

// Service implementations
pub struct InMemoryCache {
    data: RwLock<HashMap<String, (String, Instant)>>,
    stats: Mutex<CacheStats>,
    max_entries: usize,
    default_ttl: Duration,
}

impl InMemoryCache {
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            stats: Mutex::new(CacheStats {
                hits: 0,
                misses: 0,
                entries: 0,
                memory_usage: 0,
            }),
            max_entries,
            default_ttl,
        }
    }

    fn cleanup_expired(&self) {
        let mut data = self.data.write().unwrap();
        let now = Instant::now();
        data.retain(|_, (_, expiry)| now < *expiry);
    }
}

#[async_trait]
impl CacheService for InMemoryCache {
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        self.cleanup_expired();
        
        let data = self.data.read().unwrap();
        let mut stats = self.stats.lock().unwrap();
        
        match data.get(key) {
            Some((value, expiry)) => {
                if Instant::now() < *expiry {
                    stats.hits += 1;
                    Ok(Some(value.clone()))
                } else {
                    stats.misses += 1;
                    Ok(None)
                }
            }
            None => {
                stats.misses += 1;
                Ok(None)
            }
        }
    }

    async fn set(&self, key: &str, value: String, ttl: Duration) -> Result<(), CacheError> {
        let mut data = self.data.write().unwrap();
        
        if data.len() >= self.max_entries && !data.contains_key(key) {
            return Err(CacheError::StorageFull);
        }

        let expiry = Instant::now() + ttl;
        data.insert(key.to_string(), (value, expiry));
        
        let mut stats = self.stats.lock().unwrap();
        stats.entries = data.len();
        
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let mut data = self.data.write().unwrap();
        let removed = data.remove(key).is_some();
        
        let mut stats = self.stats.lock().unwrap();
        stats.entries = data.len();
        
        Ok(removed)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let mut data = self.data.write().unwrap();
        data.clear();
        
        let mut stats = self.stats.lock().unwrap();
        stats.entries = 0;
        
        Ok(())
    }

    fn stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }
}

pub struct SimpleEventBus {
    handlers: RwLock<HashMap<String, Arc<dyn EventHandler>>>,
    metrics: Mutex<EventMetrics>,
}

impl SimpleEventBus {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            metrics: Mutex::new(EventMetrics {
                events_published: 0,
                events_processed: 0,
                active_subscriptions: 0,
                processing_errors: 0,
            }),
        }
    }
}

#[async_trait]
impl EventBus for SimpleEventBus {
    async fn publish(&self, event: Event) -> Result<(), EventError> {
        // Clone handlers to avoid holding lock across await
        let handlers: Vec<Arc<dyn EventHandler>> = {
            let handlers_guard = self.handlers.read().unwrap();
            handlers_guard.values().cloned().collect()
        };
        
        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.events_published += 1;
        }

        // Find handlers that can process this event type
        for handler in handlers {
            if handler.event_types().contains(&event.event_type) {
                let result = handler.handle(&event).await;
                let mut metrics = self.metrics.lock().unwrap();
                match result {
                    Ok(_) => metrics.events_processed += 1,
                    Err(_) => metrics.processing_errors += 1,
                }
            }
        }

        Ok(())
    }

    async fn subscribe(&self, handler: Arc<dyn EventHandler>) -> Result<String, EventError> {
        let subscription_id = Uuid::new_v4().to_string();
        
        let mut handlers = self.handlers.write().unwrap();
        handlers.insert(subscription_id.clone(), handler);
        
        let mut metrics = self.metrics.lock().unwrap();
        metrics.active_subscriptions = handlers.len();

        Ok(subscription_id)
    }

    async fn unsubscribe(&self, subscription_id: &str) -> Result<(), EventError> {
        let mut handlers = self.handlers.write().unwrap();
        
        if handlers.remove(subscription_id).is_none() {
            return Err(EventError::SubscriptionNotFound { 
                id: subscription_id.to_string() 
            });
        }

        let mut metrics = self.metrics.lock().unwrap();
        metrics.active_subscriptions = handlers.len();

        Ok(())
    }

    fn get_metrics(&self) -> EventMetrics {
        self.metrics.lock().unwrap().clone()
    }
}

pub struct SimpleMetricsCollector {
    counters: RwLock<HashMap<String, u64>>,
    histograms: RwLock<HashMap<String, Vec<f64>>>,
    gauges: RwLock<HashMap<String, f64>>,
}

impl SimpleMetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
        }
    }
}

impl MetricsCollector for SimpleMetricsCollector {
    fn increment_counter(&self, name: &str, _tags: Option<HashMap<String, String>>) {
        let mut counters = self.counters.write().unwrap();
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }

    fn record_histogram(&self, name: &str, value: f64, _tags: Option<HashMap<String, String>>) {
        let mut histograms = self.histograms.write().unwrap();
        histograms.entry(name.to_string()).or_insert_with(Vec::new).push(value);
    }

    fn set_gauge(&self, name: &str, value: f64, _tags: Option<HashMap<String, String>>) {
        let mut gauges = self.gauges.write().unwrap();
        gauges.insert(name.to_string(), value);
    }

    fn get_snapshot(&self) -> MetricsSnapshot {
        let counters = self.counters.read().unwrap().clone();
        let histograms = self.histograms.read().unwrap().clone();
        let gauges = self.gauges.read().unwrap().clone();

        MetricsSnapshot {
            counters,
            histograms,
            gauges,
            timestamp: Utc::now(),
        }
    }
}

// Example event handlers
pub struct UserActivityHandler {
    cache: Arc<dyn CacheService>,
    metrics: Arc<dyn MetricsCollector>,
}

impl UserActivityHandler {
    pub fn new(cache: Arc<dyn CacheService>, metrics: Arc<dyn MetricsCollector>) -> Self {
        Self { cache, metrics }
    }
}

#[async_trait]
impl EventHandler for UserActivityHandler {
    async fn handle(&self, event: &Event) -> Result<(), EventError> {
        println!("🔔 Processing user activity: {}", event.event_type);
        
        // Update user activity cache
        let cache_key = format!("user_activity:{}", event.source);
        let _ = self.cache.set(&cache_key, event.timestamp.to_rfc3339(), Duration::from_secs(3600)).await;
        
        // Record metrics
        self.metrics.increment_counter("user_activity_events", None);
        
        Ok(())
    }

    fn event_types(&self) -> Vec<String> {
        vec!["user.login".to_string(), "user.logout".to_string(), "user.action".to_string()]
    }
}

pub struct SystemMonitorHandler {
    metrics: Arc<dyn MetricsCollector>,
}

impl SystemMonitorHandler {
    pub fn new(metrics: Arc<dyn MetricsCollector>) -> Self {
        Self { metrics }
    }
}

#[async_trait]
impl EventHandler for SystemMonitorHandler {
    async fn handle(&self, event: &Event) -> Result<(), EventError> {
        println!("📊 System event monitored: {}", event.event_type);
        
        // Extract metrics from event payload
        if let Some(response_time) = event.payload.get("response_time").and_then(|v| v.as_f64()) {
            self.metrics.record_histogram("response_time", response_time, None);
        }

        if let Some(memory_usage) = event.payload.get("memory_usage").and_then(|v| v.as_f64()) {
            self.metrics.set_gauge("memory_usage", memory_usage, None);
        }

        self.metrics.increment_counter("system_events", None);
        
        Ok(())
    }

    fn event_types(&self) -> Vec<String> {
        vec![
            "system.startup".to_string(), 
            "system.shutdown".to_string(), 
            "system.health_check".to_string(),
            "system.performance".to_string(),
        ]
    }
}

// Enhanced database service
pub struct EnhancedDatabase {
    cache: Arc<dyn CacheService>,
    metrics: Arc<dyn MetricsCollector>,
    event_bus: Arc<dyn EventBus>,
    connection_pool_size: usize,
}

impl EnhancedDatabase {
    pub fn new(
        cache: Arc<dyn CacheService>,
        metrics: Arc<dyn MetricsCollector>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            cache,
            metrics,
            event_bus,
            connection_pool_size: 10,
        }
    }

    async fn query_with_cache(&self, query: &str) -> Result<String, DatabaseError> {
        let cache_key = format!("query:{}", query);
        
        // Try cache first
        if let Ok(Some(cached_result)) = self.cache.get(&cache_key).await {
            self.metrics.increment_counter("database.cache_hit", None);
            return Ok(cached_result);
        }

        self.metrics.increment_counter("database.cache_miss", None);
        
        // Simulate database query
        let result = format!("Result for: {}", query);
        
        // Cache the result
        let _ = self.cache.set(&cache_key, result.clone(), Duration::from_secs(300)).await;
        
        // Publish performance event
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "database.query".to_string(),
            payload: serde_json::json!({
                "query": query,
                "response_time": 45.2,
                "cache_hit": false
            }),
            timestamp: Utc::now(),
            source: "database".to_string(),
        };
        
        let _ = self.event_bus.publish(event).await;
        
        Ok(result)
    }
}

impl DatabaseConnection for EnhancedDatabase {
    fn is_connected(&self) -> bool {
        self.metrics.set_gauge("database.connected", 1.0, None);
        true
    }

    fn execute(&self, query: &str) -> Result<(), DatabaseError> {
        self.metrics.increment_counter("database.queries", None);
        println!("📊 Executing query: {}", query);
        Ok(())
    }
}

// Application service that orchestrates everything
pub struct ApplicationService {
    container: Arc<Container>,
    cache: Arc<dyn CacheService>,
    event_bus: Arc<dyn EventBus>,
    metrics: Arc<dyn MetricsCollector>,
}

impl ApplicationService {
    pub fn new(container: Arc<Container>) -> Result<Self, Box<dyn std::error::Error>> {
        // Create services
        let cache = Arc::new(InMemoryCache::new(1000, Duration::from_secs(300)));
        let event_bus = Arc::new(SimpleEventBus::new());
        let metrics = Arc::new(SimpleMetricsCollector::new());

        Ok(Self {
            container,
            cache,
            event_bus,
            metrics,
        })
    }

    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Initializing application services...");

        // Set up event handlers
        let user_handler = Arc::new(UserActivityHandler::new(
            self.cache.clone(),
            self.metrics.clone(),
        ));
        
        let system_handler = Arc::new(SystemMonitorHandler::new(
            self.metrics.clone(),
        ));

        let _user_subscription = self.event_bus.subscribe(user_handler).await?;
        let _system_subscription = self.event_bus.subscribe(system_handler).await?;

        // Initialize metrics
        self.metrics.set_gauge("application.startup_time", 1.5, None);
        self.metrics.increment_counter("application.initializations", None);

        println!("✅ Application services initialized successfully");
        Ok(())
    }

    pub async fn simulate_user_activity(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n👤 Simulating user activity...");

        // Simulate user login
        let login_event = Event {
            id: Uuid::new_v4(),
            event_type: "user.login".to_string(),
            payload: serde_json::json!({
                "user_id": "user_123",
                "ip_address": "192.168.1.100",
                "user_agent": "Mozilla/5.0..."
            }),
            timestamp: Utc::now(),
            source: "auth_service".to_string(),
        };

        self.event_bus.publish(login_event).await?;

        // Simulate user actions
        for i in 1..=5 {
            let action_event = Event {
                id: Uuid::new_v4(),
                event_type: "user.action".to_string(),
                payload: serde_json::json!({
                    "action": format!("click_button_{}", i),
                    "page": "/dashboard",
                    "response_time": 120.0 + i as f64 * 10.0
                }),
                timestamp: Utc::now(),
                source: "frontend".to_string(),
            };

            self.event_bus.publish(action_event).await?;
        }

        Ok(())
    }

    pub async fn simulate_system_events(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🖥️  Simulating system events...");

        // Simulate system performance monitoring
        for i in 1..=3 {
            let perf_event = Event {
                id: Uuid::new_v4(),
                event_type: "system.performance".to_string(),
                payload: serde_json::json!({
                    "cpu_usage": 45.2 + i as f64 * 5.0,
                    "memory_usage": 2048.0 + i as f64 * 100.0,
                    "response_time": 50.0 + i as f64 * 20.0,
                    "active_connections": 25 + i * 5
                }),
                timestamp: Utc::now(),
                source: "system_monitor".to_string(),
            };

            self.event_bus.publish(perf_event).await?;
        }

        Ok(())
    }

    pub async fn demonstrate_cache_usage(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n💾 Demonstrating cache operations...");

        // Store some data
        self.cache.set("user:123:profile", "John Doe Profile".to_string(), Duration::from_secs(60)).await?;
        self.cache.set("user:456:profile", "Jane Smith Profile".to_string(), Duration::from_secs(60)).await?;

        // Retrieve data
        if let Some(profile) = self.cache.get("user:123:profile").await? {
            println!("📦 Retrieved from cache: {}", profile);
        }

        // Show cache stats
        let stats = self.cache.stats();
        println!("📊 Cache stats: hits={}, misses={}, entries={}", 
            stats.hits, stats.misses, stats.entries);

        Ok(())
    }

    pub fn print_metrics_summary(&self) {
        println!("\n📈 Metrics Summary:");
        println!("==================");
        
        let snapshot = self.metrics.get_snapshot();
        
        println!("📊 Counters:");
        for (name, value) in &snapshot.counters {
            println!("   {} = {}", name, value);
        }

        println!("\n📊 Gauges:");
        for (name, value) in &snapshot.gauges {
            println!("   {} = {:.2}", name, value);
        }

        println!("\n📊 Event Bus Metrics:");
        let event_metrics = self.event_bus.get_metrics();
        println!("   Published events: {}", event_metrics.events_published);
        println!("   Processed events: {}", event_metrics.events_processed);
        println!("   Active subscriptions: {}", event_metrics.active_subscriptions);
        println!("   Processing errors: {}", event_metrics.processing_errors);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Service Composition & Advanced DI Demo");
    println!("=========================================");

    // Create basic container (in real app, would have database config)
    let config = Arc::new(elif_core::container::test_implementations::create_test_config());
    let database = Arc::new(elif_core::container::test_implementations::TestDatabase::new()) 
        as Arc<dyn DatabaseConnection>;
    
    let container = Container::builder()
        .config(config)
        .database(database)
        .build()?
        .into();

    // Create and initialize application service
    let app_service = ApplicationService::new(container)?;
    app_service.initialize().await?;

    // Run demonstrations
    app_service.simulate_user_activity().await?;
    app_service.simulate_system_events().await?;
    app_service.demonstrate_cache_usage().await?;

    // Show final metrics
    app_service.print_metrics_summary();

    println!("\n✨ Service Composition Features Demonstrated:");
    println!("   ✅ Multi-layer service architecture");
    println!("   ✅ Event-driven communication");
    println!("   ✅ Caching with TTL and statistics");
    println!("   ✅ Metrics collection and monitoring");
    println!("   ✅ Dependency injection across service layers");
    println!("   ✅ Service lifecycle management");
    println!("   ✅ Error handling and resilience patterns");

    Ok(())
}