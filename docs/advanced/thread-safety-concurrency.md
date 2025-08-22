# Thread Safety and Concurrency

The elif.rs IoC container is designed from the ground up to be thread-safe and efficient in concurrent environments. This guide covers thread safety guarantees, concurrent patterns, and best practices for building multi-threaded applications.

## Thread Safety Guarantees

### Core Thread Safety

All container operations are thread-safe by design:

```rust
use std::sync::Arc;
use std::thread;

// Container can be safely shared across threads
let container = Arc::new(builder.build()?);

// All threads can resolve services concurrently
let handles: Vec<_> = (0..10).map(|i| {
    let container = Arc::clone(&container);
    
    thread::spawn(move || {
        // Safe concurrent resolution
        let user_service = container.resolve::<UserService>().unwrap();
        println!("Thread {}: resolved UserService", i);
    })
}).collect();

for handle in handles {
    handle.join().unwrap();
}
```

### Service Requirements

All services must be `Send + Sync`:

```rust
// All injectable services must be thread-safe
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,  // Arc ensures thread safety
    config: UserConfig,                   // Must implement Send + Sync
}

// Repository trait must be thread-safe
pub trait UserRepository: Send + Sync {
    async fn find_user(&self, id: u32) -> Result<User, DbError>;
}
```

## Concurrent Resolution

### Lock-Free Resolution

The container uses lock-free operations for service resolution:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

// Container tracks resolution statistics atomically
static RESOLUTION_COUNT: AtomicUsize = AtomicUsize::new(0);

// Multiple threads can resolve simultaneously without blocking
let handles: Vec<_> = (0..100).map(|_| {
    let container = Arc::clone(&container);
    
    tokio::spawn(async move {
        let service = container.resolve::<UserService>().unwrap();
        RESOLUTION_COUNT.fetch_add(1, Ordering::Relaxed);
        
        // Use the service
        service.do_work().await
    })
}).collect();

// Wait for all concurrent resolutions
futures::future::join_all(handles).await;
```

### Read-Write Optimization

The container optimizes for read-heavy workloads:

```rust
use std::sync::RwLock;

// Container uses RwLock for read-optimized access
struct OptimizedContainer {
    // Multiple readers can access services simultaneously
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl OptimizedContainer {
    fn resolve<T: 'static>(&self) -> Result<Arc<T>, CoreError> {
        // Read lock allows concurrent reads
        let services = self.services.read().unwrap();
        
        // Fast path: service already exists
        if let Some(service) = services.get(&TypeId::of::<T>()) {
            return Ok(Arc::clone(service.downcast_ref::<Arc<T>>().unwrap()));
        }
        
        // Upgrade to write lock only when necessary
        drop(services);
        let mut services = self.services.write().unwrap();
        
        // Double-check pattern to avoid race conditions
        if let Some(service) = services.get(&TypeId::of::<T>()) {
            return Ok(Arc::clone(service.downcast_ref::<Arc<T>>().unwrap()));
        }
        
        // Create and store the service
        let new_service = self.create_service::<T>()?;
        services.insert(TypeId::of::<T>(), new_service.clone());
        Ok(new_service)
    }
}
```

## Scope Management

### Thread-Safe Scopes

Scopes are designed for concurrent access:

```rust
use elif_core::container::{ScopeId, ConcurrentScopeManager};

let scope_manager = ConcurrentScopeManager::new();

// Multiple threads can create scopes concurrently
let scope_handles: Vec<_> = (0..10).map(|i| {
    let container = Arc::clone(&container);
    let scope_manager = Arc::clone(&scope_manager);
    
    tokio::spawn(async move {
        // Create scope safely from any thread
        let scope_id = scope_manager.create_scope().await?;
        
        // Resolve scoped services
        let db_connection = container.resolve_scoped::<DatabaseConnection>(&scope_id)?;
        
        // Use the scoped service
        perform_database_work(&db_connection).await?;
        
        // Cleanup scope when done
        scope_manager.dispose_scope(&scope_id).await
    })
}).collect();

futures::future::try_join_all(scope_handles).await?;
```

### Request Scoping in Web Applications

Handle concurrent HTTP requests with scoped services:

```rust
use elif_http::middleware::ScopeMiddleware;
use tokio::sync::Semaphore;

// Limit concurrent requests to prevent resource exhaustion
let request_semaphore = Arc::new(Semaphore::new(1000));

async fn handle_request(
    req: ElifRequest,
    container: Arc<IocContainer>,
    semaphore: Arc<Semaphore>
) -> HttpResult<ElifResponse> {
    // Acquire semaphore permit
    let _permit = semaphore.acquire().await.unwrap();
    
    // Create request scope
    let scope_id = container.create_scope()?;
    
    // Initialize request context in scope
    let request_context = RequestContext::new(&req);
    container.register_scoped_instance(&scope_id, request_context)?;
    
    // Process request with scoped services
    let result = process_request_with_scope(&req, &container, &scope_id).await;
    
    // Cleanup scope (automatic via RAII)
    container.dispose_scope(&scope_id).await?;
    
    result
}
```

## Async and Concurrent Patterns

### Async Service Initialization

Handle async initialization safely:

```rust
use tokio::sync::{Mutex, OnceCell};

#[injectable]
pub struct AsyncDatabaseService {
    connection: Arc<OnceCell<DatabaseConnection>>,
    config: DatabaseConfig,
}

impl AsyncDatabaseService {
    async fn get_connection(&self) -> Result<&DatabaseConnection, DbError> {
        self.connection.get_or_try_init(|| async {
            // This initialization happens only once, thread-safely
            DatabaseConnection::connect(&self.config.url).await
        }).await
    }
    
    pub async fn query(&self, sql: &str) -> Result<Vec<Row>, DbError> {
        let conn = self.get_connection().await?;
        conn.query(sql).await
    }
}
```

### Producer-Consumer Patterns

Use services in producer-consumer scenarios:

```rust
use tokio::sync::mpsc;

#[injectable]
pub struct MessageProcessor {
    queue_service: Arc<dyn QueueService>,
    worker_pool: Arc<WorkerPool>,
}

impl MessageProcessor {
    pub async fn start_processing(&self) -> Result<(), ProcessingError> {
        let (tx, mut rx) = mpsc::channel::<Message>(1000);
        
        // Producer task
        let queue_service = Arc::clone(&self.queue_service);
        let producer = tokio::spawn(async move {
            loop {
                match queue_service.receive_message().await {
                    Ok(message) => {
                        if tx.send(message).await.is_err() {
                            break; // Channel closed
                        }
                    }
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
        
        // Consumer tasks
        let worker_pool = Arc::clone(&self.worker_pool);
        let consumers: Vec<_> = (0..num_cpus::get()).map(|worker_id| {
            let mut rx = rx.clone();
            let worker_pool = Arc::clone(&worker_pool);
            
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    if let Err(e) = worker_pool.process_message(worker_id, message).await {
                        log::error!("Worker {} failed to process message: {}", worker_id, e);
                    }
                }
            })
        }).collect();
        
        // Wait for all tasks
        let _ = futures::future::join_all(consumers).await;
        producer.await.unwrap();
        
        Ok(())
    }
}
```

## Race Condition Prevention

### Atomic Operations

Use atomic operations for shared state:

```rust
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

#[injectable]
pub struct ThreadSafeCounter {
    count: AtomicU64,
    is_active: AtomicBool,
}

impl ThreadSafeCounter {
    pub fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            is_active: AtomicBool::new(true),
        }
    }
    
    pub fn increment(&self) -> u64 {
        // Atomic increment prevents race conditions
        self.count.fetch_add(1, Ordering::SeqCst)
    }
    
    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::SeqCst)
    }
    
    pub fn shutdown(&self) {
        self.is_active.store(false, Ordering::SeqCst);
    }
    
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }
}
```

### Double-Checked Locking

Implement safe double-checked locking:

```rust
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct LazyInitializedService<T> {
    initialized: AtomicBool,
    data: Mutex<Option<Arc<T>>>,
    initializer: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T> LazyInitializedService<T> {
    pub fn get(&self) -> Arc<T> {
        // Fast path: already initialized
        if self.initialized.load(Ordering::Acquire) {
            return Arc::clone(self.data.lock().unwrap().as_ref().unwrap());
        }
        
        // Slow path: need to initialize
        let mut data = self.data.lock().unwrap();
        
        // Double-check: another thread might have initialized
        if let Some(ref service) = *data {
            return Arc::clone(service);
        }
        
        // Initialize the service
        let service = Arc::new((self.initializer)());
        *data = Some(Arc::clone(&service));
        
        // Mark as initialized (release ordering ensures visibility)
        self.initialized.store(true, Ordering::Release);
        
        service
    }
}
```

## Deadlock Prevention

### Lock Ordering

Prevent deadlocks with consistent lock ordering:

```rust
use std::sync::{Mutex, MutexGuard};
use std::cmp::Ordering;

// Services with consistent lock ordering
pub struct OrderedLockService {
    resource_a: Mutex<ResourceA>,
    resource_b: Mutex<ResourceB>,
}

impl OrderedLockService {
    pub fn operation_requiring_both(&self) -> Result<(), ServiceError> {
        // Always acquire locks in the same order to prevent deadlocks
        // Order by memory address to ensure consistency
        let (first, second) = if (&self.resource_a as *const _) < (&self.resource_b as *const _) {
            (
                self.resource_a.lock().unwrap(),
                self.resource_b.lock().unwrap()
            )
        } else {
            // Note: In practice, you'd use try_lock with timeout
            let b = self.resource_b.lock().unwrap();
            let a = self.resource_a.lock().unwrap();
            (a, b)
        };
        
        // Perform operation with both locks held
        self.perform_operation(&first, &second);
        
        // Locks automatically released when guards go out of scope
        Ok(())
    }
}
```

### Timeout-Based Locking

Use timeouts to prevent indefinite blocking:

```rust
use std::time::Duration;
use tokio::sync::Mutex;

#[injectable]
pub struct TimeoutService {
    resource: Arc<Mutex<SharedResource>>,
}

impl TimeoutService {
    pub async fn safe_operation(&self) -> Result<(), ServiceError> {
        // Try to acquire lock with timeout
        match tokio::time::timeout(
            Duration::from_secs(5),
            self.resource.lock()
        ).await {
            Ok(guard) => {
                // Perform operation with lock held
                self.perform_operation(&guard).await?;
                Ok(())
            }
            Err(_) => {
                // Timeout occurred - avoid deadlock
                Err(ServiceError::LockTimeout)
            }
        }
    }
}
```

## Performance in Concurrent Environments

### Lock Striping

Reduce contention with lock striping:

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use std::hash::{Hash, Hasher};

const NUM_STRIPES: usize = 32;

pub struct StripedHashMap<K, V> {
    stripes: [Mutex<HashMap<K, V>>; NUM_STRIPES],
}

impl<K: Hash + Eq, V> StripedHashMap<K, V> {
    pub fn new() -> Self {
        // Initialize array of mutexes
        let stripes = std::array::from_fn(|_| Mutex::new(HashMap::new()));
        Self { stripes }
    }
    
    fn get_stripe_index(&self, key: &K) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % NUM_STRIPES
    }
    
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let stripe_index = self.get_stripe_index(&key);
        let mut stripe = self.stripes[stripe_index].lock().unwrap();
        stripe.insert(key, value)
    }
    
    pub fn get(&self, key: &K) -> Option<V> 
    where
        V: Clone,
    {
        let stripe_index = self.get_stripe_index(key);
        let stripe = self.stripes[stripe_index].lock().unwrap();
        stripe.get(key).cloned()
    }
}
```

### Lock-Free Data Structures

Use lock-free collections where appropriate:

```rust
use crossbeam::queue::SegQueue;
use std::sync::Arc;

#[injectable]
pub struct LockFreeQueueService {
    queue: Arc<SegQueue<WorkItem>>,
    workers: Arc<AtomicUsize>,
}

impl LockFreeQueueService {
    pub fn enqueue(&self, item: WorkItem) {
        // Lock-free enqueue operation
        self.queue.push(item);
    }
    
    pub fn try_dequeue(&self) -> Option<WorkItem> {
        // Lock-free dequeue operation
        self.queue.pop()
    }
    
    pub async fn process_items(&self) {
        let worker_id = self.workers.fetch_add(1, Ordering::SeqCst);
        
        loop {
            match self.try_dequeue() {
                Some(item) => {
                    // Process item
                    if let Err(e) = self.process_work_item(item).await {
                        log::error!("Worker {} failed to process item: {}", worker_id, e);
                    }
                }
                None => {
                    // No work available, yield to other tasks
                    tokio::task::yield_now().await;
                }
            }
        }
    }
}
```

## Best Practices

### 1. Design for Concurrency

Design services with concurrency in mind:

```rust
// Good: Thread-safe by design
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,    // Immutable after creation
    cache: Arc<dyn Cache>,                  // Thread-safe cache implementation
    metrics: Arc<MetricsCollector>,         // Thread-safe metrics
}

// Avoid: Mutable state without synchronization
pub struct UnsafeService {
    counter: u32,                           // Data race!
    cache: HashMap<String, String>,         // Not thread-safe!
}
```

### 2. Minimize Shared Mutable State

Reduce the need for synchronization:

```rust
// Good: Immutable configuration
#[injectable]
pub struct ConfigurableService {
    config: ServiceConfig,                  // Immutable after injection
    processor: Arc<MessageProcessor>,       // Stateless processor
}

// Better: Use message passing instead of shared state
#[injectable]  
pub struct MessageBasedService {
    command_sender: mpsc::Sender<Command>,  // Send commands to worker
    result_receiver: mpsc::Receiver<Result>, // Receive results
}
```

### 3. Use Appropriate Synchronization Primitives

Choose the right tool for the job:

```rust
use tokio::sync::{RwLock, Mutex, Semaphore};

#[injectable]
pub struct OptimizedService {
    // Read-heavy data: use RwLock
    config: Arc<RwLock<Configuration>>,
    
    // Write-heavy data: use Mutex  
    state: Arc<Mutex<ServiceState>>,
    
    // Resource limiting: use Semaphore
    connection_pool: Arc<Semaphore>,
}
```

### 4. Test Concurrent Behavior

Write tests for concurrent scenarios:

```rust
#[tokio::test]
async fn test_concurrent_service_resolution() {
    let container = Arc::new(create_test_container());
    
    // Test concurrent resolution
    let handles: Vec<_> = (0..100).map(|i| {
        let container = Arc::clone(&container);
        tokio::spawn(async move {
            let service = container.resolve::<UserService>().unwrap();
            assert_eq!(service.get_id(), i % 10); // Verify service state
        })
    }).collect();
    
    // All should complete without deadlock or race conditions
    futures::future::join_all(handles).await;
}

#[tokio::test]
async fn test_scope_isolation() {
    let container = Arc::new(create_test_container());
    
    // Create multiple scopes concurrently
    let scope_handles: Vec<_> = (0..50).map(|i| {
        let container = Arc::clone(&container);
        tokio::spawn(async move {
            let scope = container.create_scope().unwrap();
            let service = container.resolve_scoped::<ScopedService>(&scope).unwrap();
            
            // Verify scope isolation
            service.set_value(i);
            assert_eq!(service.get_value(), i);
            
            container.dispose_scope(&scope).await.unwrap();
        })
    }).collect();
    
    futures::future::join_all(scope_handles).await;
}
```

The elif.rs IoC container provides robust thread safety guarantees and efficient concurrent access patterns. By following these guidelines and using the provided synchronization primitives, you can build highly concurrent, scalable applications with confidence.