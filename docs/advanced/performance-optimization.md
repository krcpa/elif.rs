# Performance Optimization

The elif.rs IoC container is designed for high performance with zero-allocation resolution and minimal overhead. This guide covers performance optimization techniques, monitoring, and best practices for building fast, scalable applications.

## Performance Characteristics

### Zero-Allocation Resolution

The container uses compile-time analysis to eliminate runtime allocations:

```rust
// This generates zero-allocation resolution code
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    cache: Arc<dyn Cache>,
}

// Resolution is O(1) with no heap allocations
let service = container.resolve::<UserService>()?; // Zero allocations!
```

### Memory Efficiency

Services are stored efficiently with optimal memory usage:

```rust
use elif_core::container::MemoryConfig;

let config = MemoryConfig {
    initial_capacity: 256,        // Pre-allocate for known service count
    growth_factor: 1.5,           // Conservative growth to reduce reallocations
    enable_memory_pooling: true,  // Reuse memory for transient services
    compact_on_build: true,       // Remove unused capacity after build
};

let container = IocContainerBuilder::new()
    .with_memory_config(config)
    .build()?;
```

## Service Lifetime Optimization

### Choose Appropriate Lifetimes

Select lifetimes based on usage patterns:

```rust
// Singleton - for stateless services and expensive-to-create objects
builder.bind_singleton::<dyn Cache, RedisCache>();           // Expensive connection
builder.bind_singleton::<dyn Logger, StructuredLogger>();    // Stateless
builder.bind_singleton::<AppConfig, AppConfig>();            // Configuration

// Scoped - for request/transaction context
builder.bind_scoped::<dyn DatabaseConnection, PgConnection>(); // One per request
builder.bind_scoped::<RequestContext, RequestContext>();       // Request data

// Transient - for lightweight objects and per-use services
builder.bind_transient::<RequestId, RequestId>();             // Unique per use
builder.bind_transient::<ValidationResult, ValidationResult>(); // Short-lived
```

### Singleton Optimization

Optimize singleton creation for expensive objects:

```rust
use std::sync::OnceLock;

// Lazy singleton with thread-safe initialization
builder.bind_factory::<dyn Cache, _, _>(|| {
    static CACHE: OnceLock<RedisCache> = OnceLock::new();
    
    let cache = CACHE.get_or_init(|| {
        RedisCache::connect(&config.redis_url)
            .expect("Failed to connect to Redis")
    });
    
    Ok(Arc::clone(cache))
});
```

## String Allocation Optimization

The container provides optimized APIs to avoid string allocations:

```rust
// Avoid: Creates String allocation
let service = container.resolve_named::<dyn Cache>("redis".to_string())?;

// Good: Uses &str directly - zero allocations
let service = container.resolve_named::<dyn Cache>("redis")?;

// Best: Use static strings when possible
const CACHE_NAME: &str = "redis";
let service = container.resolve_named::<dyn Cache>(CACHE_NAME)?;
```

### Named Service Performance

Optimize named service resolution:

```rust
use elif_core::container::NamedServiceCache;

// Enable named service caching for frequent lookups
let cache = NamedServiceCache::new()
    .with_capacity(100)
    .with_ttl(Duration::from_secs(300));

builder.with_named_service_cache(cache);

// Pre-register frequently used names
builder.preregister_names(&["redis", "memory", "default"]);
```

## Factory Performance

### Efficient Factory Functions

Write performant factory functions:

```rust
// Good: Minimal allocations, fast execution
builder.bind_factory::<DatabaseConfig, _, _>(|| {
    Ok(DatabaseConfig {
        url: env::var("DATABASE_URL").unwrap_or_default(),
        pool_size: 10,
        timeout: Duration::from_secs(30),
    })
});

// Avoid: Expensive operations in factories
builder.bind_factory::<ExpensiveService, _, _>(|| {
    // Don't do this - runs on every resolution!
    let heavy_computation = perform_heavy_computation();
    Ok(ExpensiveService::new(heavy_computation))
});

// Better: Cache expensive computations
builder.bind_singleton_factory::<ExpensiveService, _, _>(|| {
    // Runs once and cached
    let heavy_computation = perform_heavy_computation();
    Ok(ExpensiveService::new(heavy_computation))
});
```

### Factory with Dependencies

Optimize factories that resolve dependencies:

```rust
// Efficient dependency resolution in factories
builder.bind_factory_with_container::<UserService, _, _>(|container| {
    // Resolve dependencies once, not repeatedly
    let repo = container.resolve::<dyn UserRepository>()?;
    let cache = container.resolve::<dyn Cache>()?;
    let config = container.resolve::<ServiceConfig>()?;
    
    Ok(UserService::new(repo, cache, config))
});
```

## Dependency Graph Optimization

### Minimize Dependency Depth

Keep dependency chains shallow:

```rust
// Good: Shallow dependency tree
// Controller -> Service -> Repository
#[injectable]
pub struct UserController {
    service: Arc<UserService>,
}

#[injectable]  
pub struct UserService {
    repository: Arc<dyn UserRepository>,
}

// Avoid: Deep dependency chains
// Controller -> Service -> Manager -> Provider -> Repository -> Connection -> Pool
```

### Reduce Dependency Count

Minimize dependencies per service:

```rust
// Good: Focused service with minimal dependencies
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    validator: Arc<UserValidator>,
}

// Avoid: God service with many dependencies
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    cache: Arc<dyn Cache>,
    logger: Arc<dyn Logger>,
    metrics: Arc<dyn Metrics>,
    config: Arc<AppConfig>,
    email: Arc<dyn EmailService>,
    auth: Arc<dyn AuthService>,
    // ... 10+ more dependencies - refactor this!
}
```

## Scoped Service Performance

### Efficient Scope Management

Optimize scope creation and disposal:

```rust
use elif_core::container::{ScopeConfig, ScopePool};

// Configure scope performance
let scope_config = ScopeConfig {
    initial_capacity: 50,         // Pre-allocate for expected services
    enable_pooling: true,         // Reuse scope objects
    cleanup_threshold: 1000,      // Clean up after N scopes
    background_cleanup: true,     // Async cleanup
};

builder.with_scope_config(scope_config);
```

### Request Scoping in HTTP

Optimize scoped services for web applications:

```rust
use elif_http::middleware::IoCScopeMiddleware;

// Efficient HTTP request scoping
let scope_middleware = IoCScopeMiddleware::new()
    .with_lazy_initialization()    // Create services only when needed
    .with_early_disposal()         // Dispose as soon as response is sent
    .with_service_preloading(&[    // Pre-load common services
        "RequestContext",
        "DatabaseConnection",
        "UserSession"
    ]);
```

## Memory Management

### Monitor Memory Usage

Track container memory consumption:

```rust
use elif_core::container::{MemoryTracker, MemoryReport};

let tracker = MemoryTracker::new()
    .track_service_allocation()
    .track_dependency_overhead()
    .track_scope_memory();

container.set_memory_tracker(tracker);

// Periodic memory reporting
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        
        let memory_report = container.get_memory_report();
        log::info!("Container memory usage: {} MB", memory_report.total_mb());
        
        if memory_report.total_mb() > 500 {
            log::warn!("High container memory usage: {}", memory_report);
        }
    }
});
```

### Memory-Efficient Service Design

Design services to minimize memory overhead:

```rust
// Good: Minimal memory footprint
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,  // Arc adds minimal overhead
    config: ServiceConfig,                // Small config struct
}

// Avoid: Memory-heavy services
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    cache: HashMap<String, User>,         // Large in-memory cache
    cached_results: Vec<QueryResult>,     // Growing collections
    heavy_data: LargeDataStructure,       // Expensive to clone
}
```

## Concurrent Access Optimization

### Thread-Safe Performance

Optimize for concurrent access:

```rust
use elif_core::container::ConcurrencyConfig;

let concurrency_config = ConcurrencyConfig {
    lock_striping: true,           // Reduce contention with multiple locks
    read_write_optimization: true, // Optimize for read-heavy workloads
    lock_free_resolution: true,    // Use atomic operations where possible
};

builder.with_concurrency_config(concurrency_config);
```

### Minimize Lock Contention

Reduce threading bottlenecks:

```rust
// Good: Thread-local state where appropriate
thread_local! {
    static REQUEST_CONTEXT: RefCell<Option<RequestContext>> = RefCell::new(None);
}

// Use lock-free operations for hot paths
use std::sync::atomic::{AtomicU64, Ordering};

struct PerformanceCounter {
    requests: AtomicU64,
}

impl PerformanceCounter {
    fn increment(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
    }
}
```

## Performance Monitoring

### Built-in Profiling

Enable performance profiling:

```rust
use elif_core::container::{PerformanceProfiler, ProfilerConfig};

let profiler_config = ProfilerConfig {
    sample_rate: 0.1,              // Profile 10% of resolutions
    slow_threshold: Duration::from_millis(10),
    enable_stack_traces: cfg!(debug_assertions),
    track_allocation_patterns: true,
};

let profiler = PerformanceProfiler::new(profiler_config);
container.set_profiler(profiler);

// Get performance metrics
let metrics = container.get_performance_metrics();
println!("Average resolution time: {:?}", metrics.average_resolution_time);
println!("99th percentile: {:?}", metrics.p99_resolution_time);
println!("Slowest services: {:?}", metrics.slowest_services);
```

### Custom Benchmarking

Benchmark your specific use cases:

```rust
use std::time::Instant;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_container_resolution(c: &mut Criterion) {
    let container = create_test_container();
    
    c.bench_function("resolve_user_service", |b| {
        b.iter(|| {
            let service = container.resolve::<UserService>().unwrap();
            black_box(service);
        });
    });
    
    c.bench_function("resolve_with_complex_dependencies", |b| {
        b.iter(|| {
            let service = container.resolve::<ComplexService>().unwrap();
            black_box(service);
        });
    });
}

criterion_group!(benches, benchmark_container_resolution);
criterion_main!(benches);
```

## Production Optimization

### Build-Time Optimizations

Optimize container at build time:

```rust
let container = IocContainerBuilder::new()
    // ... service registrations ...
    .optimize_dependency_graph()     // Reorder for optimal resolution
    .precompute_resolution_paths()   // Cache resolution strategies
    .enable_dead_service_elimination() // Remove unused services
    .compact_memory_layout()         // Optimize memory layout
    .build()?;
```

### Runtime Performance Tuning

Tune performance based on application characteristics:

```rust
use elif_core::container::RuntimeConfig;

let runtime_config = RuntimeConfig {
    resolution_cache_size: 1000,    // Cache frequently resolved services
    enable_jit_optimization: true,  // JIT optimize hot resolution paths
    gc_threshold: 10000,            // Garbage collect after N resolutions
    background_optimization: true,  // Optimize in background thread
};

container.set_runtime_config(runtime_config);
```

## Performance Patterns

### Lazy Loading

Implement lazy loading for expensive services:

```rust
use std::sync::LazyLock;

#[injectable]
pub struct ExpensiveService {
    #[lazy]
    heavy_resource: LazyLock<HeavyResource>,
    cheap_resource: Arc<CheapResource>,
}

impl ExpensiveService {
    fn get_heavy_resource(&self) -> &HeavyResource {
        &self.heavy_resource  // Initialized on first access
    }
}
```

### Service Pooling

Pool expensive-to-create services:

```rust
use elif_core::container::ServicePool;

// Pool database connections
let connection_pool = ServicePool::<DatabaseConnection>::new()
    .with_capacity(10, 50)         // Min 10, max 50 connections
    .with_timeout(Duration::from_secs(30))
    .with_health_check(|conn| conn.ping())
    .with_cleanup_interval(Duration::from_mins(5));

builder.bind_pool::<dyn DatabaseConnection, _>(connection_pool);
```

### Batch Resolution

Resolve multiple services efficiently:

```rust
// Efficient batch resolution
let services = container.resolve_batch(&[
    TypeId::of::<UserService>(),
    TypeId::of::<EmailService>(),
    TypeId::of::<CacheService>(),
])?;

// Safer typed version
let (user_service, email_service, cache_service) = container.resolve_tuple::<(
    UserService,
    EmailService, 
    CacheService
)>()?;
```

## Performance Best Practices

### 1. Profile First, Optimize Second

Always measure before optimizing:

```rust
#[cfg(debug_assertions)]
{
    let start = Instant::now();
    let service = container.resolve::<ServiceType>()?;
    let duration = start.elapsed();
    
    if duration > Duration::from_millis(10) {
        log::warn!("Slow resolution: {} took {:?}", type_name::<ServiceType>(), duration);
    }
}
```

### 2. Use Appropriate Data Structures

Choose the right collections:

```rust
// Good for frequent lookups
use std::collections::HashMap;

// Better for small collections
use std::collections::BTreeMap;

// Best for known keys at compile time
use phf::Map;

static SERVICES: phf::Map<&'static str, ServiceType> = phf_map! {
    "user" => ServiceType::User,
    "email" => ServiceType::Email,
};
```

### 3. Minimize Dynamic Allocations

Avoid allocations in hot paths:

```rust
// Good: Pre-allocated buffer
struct ServiceBuffer {
    buffer: Vec<Service>,
}

impl ServiceBuffer {
    fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(100), // Pre-allocate
        }
    }
    
    fn add_service(&mut self, service: Service) {
        if self.buffer.len() < self.buffer.capacity() {
            self.buffer.push(service); // No allocation
        } else {
            // Handle overflow appropriately
        }
    }
}
```

### 4. Optimize Critical Paths

Focus optimization on frequently used services:

```rust
// Identify critical services through profiling
let critical_services = profiler.get_top_resolved_services(10);

for service_type in critical_services {
    // Apply specific optimizations
    builder.optimize_service_resolution(service_type);
}
```

The elif.rs IoC container is designed for high performance out of the box, but following these optimization techniques will help you build even faster, more scalable applications. Focus on measuring performance, choosing appropriate service lifetimes, and optimizing your most critical resolution paths.