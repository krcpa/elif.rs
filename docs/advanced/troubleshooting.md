# Troubleshooting Guide

This guide covers common issues you might encounter with the elif.rs IoC container and provides step-by-step solutions to resolve them.

## Common Error Messages

### Service Not Found

**Error Message:**
```
CoreError::ServiceNotFound { service_type: "UserService" }
```

**Cause:** The service wasn't registered in the container.

**Solutions:**

1. **Check Service Registration:**
```rust
let mut builder = IocContainerBuilder::new();

// Make sure the service is registered
builder.bind_injectable::<UserService>();
// OR
builder.bind::<UserService, UserService>();

let container = builder.build()?;
```

2. **Interface vs Implementation Registration:**
```rust
// Problem: Registered concrete type but trying to resolve interface
builder.bind::<UserService, UserService>();
let service = container.resolve::<dyn UserRepository>()?; // ❌ Will fail

// Solution: Register the interface binding
builder.bind::<dyn UserRepository, PostgresUserRepository>();
let service = container.resolve::<dyn UserRepository>()?; // ✅ Works
```

3. **Named Service Issues:**
```rust
// Problem: Registered without name but trying to resolve with name
builder.bind::<dyn Cache, RedisCache>();
let cache = container.resolve_named::<dyn Cache>("redis")?; // ❌ Will fail

// Solution: Register with name
builder.bind_named::<dyn Cache, RedisCache>("redis");
let cache = container.resolve_named::<dyn Cache>("redis")?; // ✅ Works
```

### Circular Dependency Detected

**Error Message:**
```
CoreError::CircularDependency { cycle: ["ServiceA", "ServiceB", "ServiceA"] }
```

**Cause:** Services depend on each other in a loop.

**Solutions:**

1. **Break the Cycle with Interfaces:**
```rust
// Problem: Direct circular dependency
#[injectable]
struct ServiceA { b: Arc<ServiceB> }

#[injectable]  
struct ServiceB { a: Arc<ServiceA> } // ❌ Circular!

// Solution: Use interface to break the cycle
trait ServiceAInterface: Send + Sync {
    fn do_something(&self);
}

impl ServiceAInterface for ServiceA {
    fn do_something(&self) { /* ... */ }
}

#[injectable]
struct ServiceB { 
    a: Arc<dyn ServiceAInterface>  // ✅ No more direct circular dependency
}

// Register both
builder
    .bind_injectable::<ServiceA>()
    .bind_injectable::<ServiceB>()  
    .bind::<dyn ServiceAInterface, ServiceA>();
```

2. **Use Event-Driven Architecture:**
```rust
// Problem: Services need to communicate bidirectionally
#[injectable]
struct OrderService { payment: Arc<PaymentService> }

#[injectable]
struct PaymentService { order: Arc<OrderService> } // ❌ Circular!

// Solution: Use event bus
trait EventBus: Send + Sync {
    fn publish(&self, event: Event);
}

#[injectable]
struct OrderService { 
    payment: Arc<PaymentService>,
    event_bus: Arc<dyn EventBus>  // ✅ Communicate via events
}

#[injectable]  
struct PaymentService {
    event_bus: Arc<dyn EventBus>  // ✅ No direct dependency on OrderService
}
```

### Missing Dependencies

**Error Message:**
```
CoreError::DependencyResolution { 
    service: "UserService", 
    missing: "UserRepository" 
}
```

**Cause:** An `#[injectable]` service depends on a service that isn't registered.

**Solutions:**

1. **Register All Dependencies:**
```rust
#[injectable]
struct UserService {
    repository: Arc<dyn UserRepository>,  // This must be registered
    cache: Arc<dyn Cache>,                // This too
}

// Register all dependencies
builder
    .bind::<dyn UserRepository, PostgresUserRepository>()
    .bind::<dyn Cache, RedisCache>()
    .bind_injectable::<UserService>();  // ✅ Now UserService can be resolved
```

2. **Use Optional Dependencies:**
```rust
#[injectable]
struct UserService {
    repository: Arc<dyn UserRepository>,     // Required
    cache: Option<Arc<dyn Cache>>,           // Optional - won't fail if missing
    metrics: Option<Arc<MetricsCollector>>,  // Optional
}
```

### Lifetime Incompatibility

**Error Message:**
```
CoreError::LifetimeIncompatibility { 
    service: "SingletonService", 
    dependency: "TransientService" 
}
```

**Cause:** A longer-lived service depends on a shorter-lived service.

**Solutions:**

1. **Fix Lifetime Hierarchy:**
```rust
// Problem: Singleton depending on Transient
builder
    .bind_singleton::<UserService, UserService>()      // Lives forever
    .bind_transient::<DatabaseConnection, PgConnection>(); // New instance each time

// Solution: Make dependency longer-lived
builder
    .bind_singleton::<UserService, UserService>()
    .bind_singleton::<DatabaseConnection, PgConnection>(); // ✅ Both singletons
```

2. **Use Factory Pattern:**
```rust
// Problem: Singleton needs fresh instances
#[injectable]
struct CacheService {
    connection_factory: Arc<dyn ConnectionFactory>, // ✅ Factory instead of instance
}

trait ConnectionFactory: Send + Sync {
    fn create_connection(&self) -> Result<DatabaseConnection, DbError>;
}
```

## Compilation Issues

### Injectable Macro Errors

**Error Message:**
```
error: #[injectable] can only be applied to structs
```

**Cause:** Using `#[injectable]` on unsupported types.

**Solutions:**

1. **Only Use on Structs:**
```rust
// ❌ Wrong - can't use on enums
#[injectable]
enum ServiceType {
    User,
    Email,
}

// ✅ Correct - use on structs only
#[injectable]
struct UserService {
    // ...
}
```

2. **Field Type Requirements:**
```rust
// ❌ Wrong - non-injectable field types
#[injectable]
struct UserService {
    count: u32,                    // ❌ Primitive types not supported
    callback: Box<dyn Fn()>,       // ❌ Non-Send/Sync types not supported
}

// ✅ Correct - use Arc for dependencies
#[injectable]  
struct UserService {
    repository: Arc<dyn UserRepository>, // ✅ Arc<dyn Trait>
    config: UserConfig,                  // ✅ Value types that are Send + Sync
}
```

### Type Resolution Errors

**Error Message:**
```
error: the trait bound `UserService: Send` is not satisfied
```

**Cause:** Service doesn't implement required thread safety traits.

**Solutions:**

1. **Ensure All Fields are Send + Sync:**
```rust
// Problem: Non-Send/Sync field
struct BadService {
    resource: Rc<Resource>,  // ❌ Rc is not Send
}

// Solution: Use Arc instead
#[injectable]
struct GoodService {
    resource: Arc<Resource>, // ✅ Arc is Send + Sync
}
```

2. **Add Trait Bounds:**
```rust
// If you need custom types, ensure they're thread-safe
pub struct MyResource {
    data: String,
}

// Explicitly implement thread safety if data allows it
unsafe impl Send for MyResource {}
unsafe impl Sync for MyResource {}

// Or better, design for thread safety
use parking_lot::Mutex;

pub struct ThreadSafeResource {
    data: Arc<Mutex<String>>,
}
```

## Runtime Issues

### Performance Problems

**Symptoms:**
- Slow application startup
- High memory usage  
- Long service resolution times

**Diagnosis:**

1. **Enable Performance Profiling:**
```rust
use elif_core::container::PerformanceProfiler;

let profiler = PerformanceProfiler::new()
    .with_detailed_timing()
    .with_memory_tracking();

container.set_profiler(profiler);

// After some usage, check metrics
let metrics = container.get_performance_metrics();
println!("Slow services: {:?}", metrics.slowest_services);
println!("Memory usage: {} MB", metrics.memory_usage_mb);
```

2. **Common Solutions:**
```rust
// Problem: Too many transient services
builder.bind_transient::<ExpensiveService, ExpensiveService>(); // ❌ Recreated every time

// Solution: Use singleton for expensive services
builder.bind_singleton::<ExpensiveService, ExpensiveService>(); // ✅ Created once

// Problem: Expensive factory functions  
builder.bind_factory::<ConfigService, _, _>(|| {
    expensive_computation(); // ❌ Runs every resolution
    Ok(ConfigService::new())
});

// Solution: Cache the result
use std::sync::OnceLock;
static CONFIG: OnceLock<ConfigService> = OnceLock::new();

builder.bind_factory::<ConfigService, _, _>(|| {
    let config = CONFIG.get_or_init(|| {
        expensive_computation();
        ConfigService::new()
    });
    Ok(config.clone()) // ✅ Expensive operation runs once
});
```

### Memory Leaks

**Symptoms:**
- Continuously growing memory usage
- Scopes not being cleaned up

**Diagnosis:**

1. **Enable Memory Tracking:**
```rust
use elif_core::container::MemoryTracker;

let tracker = MemoryTracker::new()
    .track_scope_memory()
    .track_service_allocations();

container.set_memory_tracker(tracker);

// Check for leaks
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let report = container.get_memory_report();
        
        if report.scope_count > 1000 {
            log::warn!("Possible scope leak: {} active scopes", report.scope_count);
        }
    }
});
```

2. **Common Solutions:**
```rust
// Problem: Forgetting to dispose scopes
async fn handle_request() {
    let scope = container.create_scope()?;
    let service = container.resolve_scoped::<DatabaseConnection>(&scope)?;
    
    // ... do work ...
    
    // ❌ Forgot to dispose scope - memory leak!
}

// Solution: Use RAII pattern
struct ScopeGuard {
    container: Arc<IocContainer>,
    scope_id: ScopeId,
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        // Dispose scope automatically
        if let Err(e) = self.container.dispose_scope(&self.scope_id) {
            log::error!("Failed to dispose scope: {}", e);
        }
    }
}

// Or use async drop pattern
async fn handle_request() {
    let scope_guard = ScopeGuard::new(&container).await?;
    let service = container.resolve_scoped::<DatabaseConnection>(&scope_guard.scope_id())?;
    
    // ... do work ...
    
    // ✅ Scope automatically disposed when guard drops
}
```

### Thread Safety Issues

**Symptoms:**
- Race conditions
- Deadlocks
- Data corruption

**Diagnosis:**

1. **Enable Concurrent Debugging:**
```rust
// Add to Cargo.toml for development
[dependencies]
parking_lot = "0.12"

// Use debug-aware locks in development
#[cfg(debug_assertions)]
type DebugMutex<T> = parking_lot::Mutex<T>;

#[cfg(not(debug_assertions))]  
type DebugMutex<T> = std::sync::Mutex<T>;
```

2. **Common Solutions:**
```rust
// Problem: Shared mutable state
struct UnsafeService {
    counter: u32,  // ❌ Race condition!
}

// Solution: Use atomic types
use std::sync::atomic::{AtomicU32, Ordering};

#[injectable]
struct SafeService {
    counter: AtomicU32,  // ✅ Thread-safe
}

impl SafeService {
    fn increment(&self) -> u32 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}
```

## Debugging Tools

### Container Inspection

Use built-in inspection tools:

```rust
use elif_core::container::{ContainerInspector, InspectionLevel};

let inspector = ContainerInspector::new(&container);

// Quick health check
if !inspector.is_healthy() {
    let issues = inspector.get_health_issues();
    for issue in issues {
        println!("Issue: {}", issue.description);
        if let Some(fix) = issue.suggested_fix {
            println!("Suggested fix: {}", fix);
        }
    }
}

// Detailed analysis
let report = inspector.inspect(InspectionLevel::Detailed)?;
println!("Container Report:\n{}", report);
```

### Dependency Visualization

Generate visual dependency graphs:

```rust
use elif_core::container::{DependencyVisualizer, VisualizationFormat};

let visualizer = DependencyVisualizer::new(&container);

// Generate ASCII tree for quick inspection
let ascii = visualizer.visualize(VisualizationFormat::Ascii, Default::default())?;
println!("Dependencies:\n{}", ascii);

// Generate DOT file for detailed analysis
let dot = visualizer.visualize(VisualizationFormat::Dot, Default::default())?;
std::fs::write("dependencies.dot", dot)?;
// Then: dot -Tpng dependencies.dot -o dependencies.png
```

### Logging and Tracing

Enable detailed logging:

```rust
// Add to Cargo.toml
[dependencies]
log = "0.4"
env_logger = "0.10"

// Enable container logging
fn setup_logging() {
    env_logger::Builder::new()
        .filter_module("elif_core::container", log::LevelFilter::Debug)
        .init();
}

// Set environment variable
// RUST_LOG=elif_core::container=debug cargo run
```

## Prevention Strategies

### 1. Use Container Validation in Tests

```rust
#[test]
fn validate_production_container() {
    let container = create_production_container();
    
    let inspector = ContainerInspector::new(&container);
    let report = inspector.validate_all();
    
    assert!(report.is_valid(), "Container validation failed: {:?}", report.errors);
    assert!(report.warnings.is_empty(), "Container has warnings: {:?}", report.warnings);
}
```

### 2. Automated Dependency Analysis

```rust
// In your CI pipeline
#[test]
fn analyze_dependency_complexity() {
    let container = create_production_container();
    let analyzer = DependencyAnalyzer::new(&container);
    
    let complexity = analyzer.calculate_complexity();
    assert!(complexity.max_depth < 10, "Dependency tree too deep: {}", complexity.max_depth);
    assert!(complexity.avg_dependencies < 5.0, "Too many dependencies per service: {}", complexity.avg_dependencies);
}
```

### 3. Performance Regression Testing

```rust
#[test]
fn performance_regression_test() {
    let container = create_production_container();
    
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = container.resolve::<CriticalService>()?;
    }
    let duration = start.elapsed();
    
    assert!(duration < Duration::from_millis(100), 
            "Service resolution too slow: {:?}", duration);
}
```

By following these troubleshooting steps and prevention strategies, you can quickly identify and resolve issues with the elif.rs IoC container, ensuring your application runs smoothly in all environments.