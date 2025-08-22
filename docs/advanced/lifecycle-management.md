# Service Lifecycle Management

The elif.rs IoC container provides comprehensive lifecycle management for services, including initialization, resource cleanup, and graceful shutdown. This guide covers lifecycle patterns, async operations, and best practices for managing service lifecycles.

## Lifecycle Overview

### Service States

Services in the container go through several states:

```rust
use elif_core::container::ServiceState;

pub enum ServiceState {
    Registered,    // Service is registered but not created
    Creating,      // Service is being instantiated
    Initializing,  // Service is running initialization logic
    Ready,         // Service is fully initialized and ready
    Disposing,     // Service is being cleaned up
    Disposed,      // Service has been disposed
    Failed(String), // Service initialization or disposal failed
}
```

### Lifecycle Interfaces

Implement lifecycle interfaces for custom behavior:

```rust
use elif_core::container::{AsyncInitializable, Disposable, LifecycleManaged};
use async_trait::async_trait;

#[async_trait]
pub trait AsyncInitializable: Send + Sync {
    async fn initialize(&self) -> Result<(), CoreError>;
}

#[async_trait]
pub trait Disposable: Send + Sync {
    async fn dispose(&self) -> Result<(), CoreError>;
}

// Convenience trait combining both
pub trait LifecycleManaged: AsyncInitializable + Disposable + Send + Sync {}
```

## Service Initialization

### Basic Initialization

Implement async initialization for services:

```rust
use elif_core::container::AsyncInitializable;
use async_trait::async_trait;

#[injectable]
pub struct DatabaseService {
    config: DatabaseConfig,
    pool: Arc<Mutex<Option<PgPool>>>,
}

#[async_trait]
impl AsyncInitializable for DatabaseService {
    async fn initialize(&self) -> Result<(), CoreError> {
        println!("Initializing database service...");
        
        // Connect to database
        let pool = PgPool::connect(&self.config.url).await
            .map_err(|e| CoreError::InitializationError {
                service: "DatabaseService".to_string(),
                error: e.to_string(),
            })?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await
            .map_err(|e| CoreError::InitializationError {
                service: "DatabaseService".to_string(), 
                error: format!("Migration failed: {}", e),
            })?;
        
        // Store the connection pool
        *self.pool.lock().await = Some(pool);
        
        println!("Database service initialized successfully");
        Ok(())
    }
}
```

### Initialization with Dependencies

Initialize services that depend on other services:

```rust
#[injectable]
pub struct EmailService {
    smtp_config: SmtpConfig,
    template_engine: Arc<dyn TemplateEngine>,
    connection: Arc<Mutex<Option<SmtpConnection>>>,
}

#[async_trait]
impl AsyncInitializable for EmailService {
    async fn initialize(&self) -> Result<(), CoreError> {
        // Initialize template engine first (dependency)
        self.template_engine.initialize().await?;
        
        // Connect to SMTP server
        let connection = SmtpConnection::connect(&self.smtp_config).await?;
        
        // Test the connection
        connection.test().await
            .map_err(|e| CoreError::InitializationError {
                service: "EmailService".to_string(),
                error: format!("SMTP test failed: {}", e),
            })?;
        
        *self.connection.lock().await = Some(connection);
        Ok(())
    }
}
```

### Initialization Ordering

The container automatically handles initialization ordering:

```rust
// Container will initialize in dependency order:
// 1. ConfigService (no dependencies)
// 2. DatabaseService (depends on ConfigService) 
// 3. CacheService (depends on ConfigService)
// 4. UserService (depends on DatabaseService and CacheService)

let container = builder.build()?;

// Initialize all services with proper ordering
let lifecycle_manager = ServiceLifecycleManager::new(&container);
lifecycle_manager.initialize_all().await?;
```

## Resource Cleanup

### Implementing Disposal

Clean up resources when services are no longer needed:

```rust
#[async_trait]
impl Disposable for DatabaseService {
    async fn dispose(&self) -> Result<(), CoreError> {
        println!("Disposing database service...");
        
        if let Some(pool) = self.pool.lock().await.take() {
            // Close database connections
            pool.close().await;
            println!("Database connections closed");
        }
        
        Ok(())
    }
}

#[async_trait]
impl Disposable for EmailService {
    async fn dispose(&self) -> Result<(), CoreError> {
        println!("Disposing email service...");
        
        if let Some(connection) = self.connection.lock().await.take() {
            // Close SMTP connection
            connection.close().await?;
            println!("SMTP connection closed");
        }
        
        Ok(())
    }
}
```

### Automatic Cleanup

Services are automatically disposed in reverse initialization order:

```rust
// During container shutdown:
// 1. UserService disposed first
// 2. CacheService disposed  
// 3. DatabaseService disposed
// 4. ConfigService disposed last

let lifecycle_manager = ServiceLifecycleManager::new(&container);

// Graceful shutdown
lifecycle_manager.shutdown_all().await?;
```

## Scoped Lifecycle Management

### Request-Scoped Services

Manage lifecycle for request-scoped services:

```rust
#[injectable]
pub struct RequestContext {
    request_id: String,
    user_session: Option<UserSession>,
    transaction: Arc<Mutex<Option<DatabaseTransaction>>>,
}

#[async_trait]
impl AsyncInitializable for RequestContext {
    async fn initialize(&self) -> Result<(), CoreError> {
        // Initialize request tracking
        metrics::counter!("requests.started").increment(1);
        
        // Start database transaction
        let db_service = /* get from container */;
        let transaction = db_service.begin_transaction().await?;
        *self.transaction.lock().await = Some(transaction);
        
        Ok(())
    }
}

#[async_trait] 
impl Disposable for RequestContext {
    async fn dispose(&self) -> Result<(), CoreError> {
        // Commit or rollback transaction
        if let Some(transaction) = self.transaction.lock().await.take() {
            transaction.commit().await?;
        }
        
        // Update metrics
        metrics::counter!("requests.completed").increment(1);
        
        Ok(())
    }
}
```

### Scope-Aware Cleanup

Handle scope disposal with proper cleanup:

```rust
use elif_http::middleware::ScopeLifecycleMiddleware;

async fn handle_request(req: ElifRequest) -> HttpResult<ElifResponse> {
    // Create request scope
    let scope = container.create_scope()?;
    
    // Scope will be automatically cleaned up
    let _guard = ScopeGuard::new(&container, &scope);
    
    // Initialize scoped services
    let lifecycle_manager = ScopedLifecycleManager::new(&container, &scope);
    lifecycle_manager.initialize_scope_services().await?;
    
    // Process request with scoped services
    let response = process_request_with_scope(&req, &scope).await?;
    
    // Cleanup happens automatically via guard
    Ok(response)
}

struct ScopeGuard {
    container: Arc<IocContainer>,
    scope: ScopeId,
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        // Dispose scope asynchronously
        let container = Arc::clone(&self.container);
        let scope = self.scope.clone();
        
        tokio::spawn(async move {
            if let Err(e) = container.dispose_scope(&scope).await {
                log::error!("Failed to dispose scope {}: {}", scope, e);
            }
        });
    }
}
```

## Advanced Lifecycle Patterns

### Background Service Management

Manage long-running background services:

```rust
#[injectable]
pub struct BackgroundTaskService {
    task_queue: Arc<dyn TaskQueue>,
    worker_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    shutdown_signal: Arc<tokio::sync::Notify>,
}

#[async_trait]
impl AsyncInitializable for BackgroundTaskService {
    async fn initialize(&self) -> Result<(), CoreError> {
        let mut handles = Vec::new();
        
        // Start worker tasks
        for worker_id in 0..num_cpus::get() {
            let queue = Arc::clone(&self.task_queue);
            let shutdown = Arc::clone(&self.shutdown_signal);
            
            let handle = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        // Process tasks
                        task = queue.receive() => {
                            if let Ok(task) = task {
                                if let Err(e) = process_task(task).await {
                                    log::error!("Worker {} task failed: {}", worker_id, e);
                                }
                            }
                        }
                        
                        // Handle shutdown
                        _ = shutdown.notified() => {
                            log::info!("Worker {} shutting down", worker_id);
                            break;
                        }
                    }
                }
            });
            
            handles.push(handle);
        }
        
        *self.worker_handles.lock().await = handles;
        Ok(())
    }
}

#[async_trait]
impl Disposable for BackgroundTaskService {
    async fn dispose(&self) -> Result<(), CoreError> {
        // Signal shutdown to all workers
        self.shutdown_signal.notify_waiters();
        
        // Wait for workers to complete
        let handles = std::mem::take(&mut *self.worker_handles.lock().await);
        for handle in handles {
            if let Err(e) = handle.await {
                log::error!("Worker task panicked: {:?}", e);
            }
        }
        
        Ok(())
    }
}
```

### Dependency-Aware Disposal

Handle complex disposal dependencies:

```rust
pub struct ServiceDependencyGraph {
    dependencies: HashMap<TypeId, Vec<TypeId>>,
}

impl ServiceDependencyGraph {
    pub async fn dispose_all_in_order(&self, container: &IocContainer) -> Result<(), CoreError> {
        // Calculate reverse topological order for disposal
        let disposal_order = self.calculate_disposal_order()?;
        
        for service_type in disposal_order {
            if let Ok(service) = container.try_resolve_by_type_id(&service_type) {
                if let Some(disposable) = service.as_any().downcast_ref::<dyn Disposable>() {
                    if let Err(e) = disposable.dispose().await {
                        log::error!("Failed to dispose service {:?}: {}", service_type, e);
                        // Continue with other services - don't let one failure stop cleanup
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## Lifecycle Configuration

### Initialization Timeouts

Configure timeouts for service initialization:

```rust
use elif_core::container::{LifecycleConfig, InitializationTimeout};

let lifecycle_config = LifecycleConfig {
    // Overall initialization timeout
    global_timeout: Duration::from_secs(60),
    
    // Per-service timeouts
    service_timeouts: [
        (TypeId::of::<DatabaseService>(), Duration::from_secs(30)),
        (TypeId::of::<CacheService>(), Duration::from_secs(10)),
        (TypeId::of::<EmailService>(), Duration::from_secs(15)),
    ].iter().cloned().collect(),
    
    // Retry configuration
    max_retry_attempts: 3,
    retry_delay: Duration::from_secs(5),
    
    // Failure handling
    continue_on_failure: false, // Stop initialization on first failure
    critical_services: vec![    // Must initialize successfully
        TypeId::of::<DatabaseService>(),
        TypeId::of::<ConfigService>(),
    ],
};

let lifecycle_manager = ServiceLifecycleManager::new(&container)
    .with_config(lifecycle_config);
```

### Health Check Integration

Integrate lifecycle management with health checks:

```rust
#[async_trait]
impl AsyncInitializable for DatabaseService {
    async fn initialize(&self) -> Result<(), CoreError> {
        // Initialize database
        let pool = PgPool::connect(&self.config.url).await?;
        *self.pool.lock().await = Some(pool);
        
        // Register health check
        let pool_ref = Arc::downgrade(&self.pool);
        container.register_health_check("database", move || {
            Box::pin(async move {
                if let Some(pool) = pool_ref.upgrade() {
                    if let Some(pool) = pool.lock().await.as_ref() {
                        return pool.ping().await.is_ok();
                    }
                }
                false
            })
        });
        
        Ok(())
    }
}

// Monitor service health during runtime
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        
        let health_report = container.check_all_health().await;
        if !health_report.is_healthy() {
            log::warn!("Service health issues: {:?}", health_report.issues);
        }
    }
});
```

## Error Handling

### Initialization Error Recovery

Handle initialization failures gracefully:

```rust
pub struct RobustLifecycleManager {
    container: Arc<IocContainer>,
    retry_policy: RetryPolicy,
}

impl RobustLifecycleManager {
    pub async fn initialize_with_recovery(&self) -> Result<(), CoreError> {
        let services = self.container.get_initializable_services();
        
        for service in services {
            match self.initialize_service_with_retry(service).await {
                Ok(()) => {
                    log::info!("Service {} initialized successfully", service.type_name());
                }
                Err(e) => {
                    if self.is_critical_service(&service) {
                        log::error!("Critical service {} failed to initialize: {}", service.type_name(), e);
                        return Err(e);
                    } else {
                        log::warn!("Non-critical service {} failed to initialize: {}", service.type_name(), e);
                        // Mark as failed but continue with other services
                        self.container.mark_service_as_failed(&service, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn initialize_service_with_retry(&self, service: &dyn AsyncInitializable) -> Result<(), CoreError> {
        let mut attempts = 0;
        let max_attempts = self.retry_policy.max_attempts;
        
        loop {
            match service.initialize().await {
                Ok(()) => return Ok(()),
                Err(e) if attempts < max_attempts => {
                    attempts += 1;
                    log::warn!("Service initialization attempt {} failed: {}", attempts, e);
                    tokio::time::sleep(self.retry_policy.delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

### Partial Failure Handling

Handle scenarios where some services fail:

```rust
pub struct PartialInitializationResult {
    pub successful: Vec<String>,
    pub failed: Vec<(String, CoreError)>,
    pub is_operational: bool,
}

impl RobustLifecycleManager {
    pub async fn initialize_best_effort(&self) -> PartialInitializationResult {
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        
        let services = self.container.get_initializable_services();
        
        // Initialize services in dependency order, but don't stop on failures
        for service in services {
            match service.initialize().await {
                Ok(()) => {
                    successful.push(service.type_name().to_string());
                }
                Err(e) => {
                    failed.push((service.type_name().to_string(), e));
                }
            }
        }
        
        // Determine if the application is operational
        let critical_failed = failed.iter()
            .any(|(name, _)| self.is_critical_service_by_name(name));
        
        PartialInitializationResult {
            successful,
            failed,
            is_operational: !critical_failed,
        }
    }
}
```

## Best Practices

### 1. Implement Idempotent Operations

Make initialization and disposal operations idempotent:

```rust
#[injectable]
pub struct IdempotentService {
    initialized: Arc<AtomicBool>,
    disposed: Arc<AtomicBool>,
}

#[async_trait]
impl AsyncInitializable for IdempotentService {
    async fn initialize(&self) -> Result<(), CoreError> {
        // Check if already initialized
        if self.initialized.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already initialized
        }
        
        // Perform initialization
        self.do_initialization().await?;
        Ok(())
    }
}

#[async_trait] 
impl Disposable for IdempotentService {
    async fn dispose(&self) -> Result<(), CoreError> {
        // Check if already disposed
        if self.disposed.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already disposed
        }
        
        // Perform disposal
        self.do_disposal().await?;
        Ok(())
    }
}
```

### 2. Use Timeout Protection

Always use timeouts for initialization:

```rust
#[async_trait]
impl AsyncInitializable for TimeoutProtectedService {
    async fn initialize(&self) -> Result<(), CoreError> {
        match tokio::time::timeout(Duration::from_secs(30), self.do_init()).await {
            Ok(result) => result,
            Err(_) => Err(CoreError::InitializationError {
                service: "TimeoutProtectedService".to_string(),
                error: "Initialization timed out".to_string(),
            }),
        }
    }
}
```

### 3. Log Lifecycle Events

Provide visibility into lifecycle operations:

```rust
#[async_trait]
impl AsyncInitializable for WellLoggedService {
    async fn initialize(&self) -> Result<(), CoreError> {
        log::info!("Starting initialization of WellLoggedService");
        let start_time = Instant::now();
        
        match self.do_initialization().await {
            Ok(()) => {
                let duration = start_time.elapsed();
                log::info!("WellLoggedService initialized successfully in {:?}", duration);
                Ok(())
            }
            Err(e) => {
                log::error!("WellLoggedService initialization failed: {}", e);
                Err(e)
            }
        }
    }
}
```

### 4. Test Lifecycle Behavior

Write comprehensive tests for lifecycle behavior:

```rust
#[tokio::test]
async fn test_service_lifecycle() {
    let container = create_test_container();
    let service = container.resolve::<TestService>()?;
    
    // Test initialization
    assert!(!service.is_initialized());
    service.initialize().await?;
    assert!(service.is_initialized());
    
    // Test idempotency
    service.initialize().await?; // Should not fail
    
    // Test disposal
    assert!(!service.is_disposed());
    service.dispose().await?;
    assert!(service.is_disposed());
    
    // Test idempotency
    service.dispose().await?; // Should not fail
}

#[tokio::test]
async fn test_initialization_failure_recovery() {
    let container = create_test_container();
    
    // Inject failure condition
    container.configure_service::<FailingService>(|config| {
        config.should_fail_initialization(true);
    });
    
    let lifecycle_manager = RobustLifecycleManager::new(&container);
    let result = lifecycle_manager.initialize_best_effort().await;
    
    assert!(!result.is_operational);
    assert!(result.failed.iter().any(|(name, _)| name == "FailingService"));
}
```

Effective lifecycle management is crucial for building robust, scalable applications. The elif.rs IoC container provides comprehensive lifecycle support to ensure your services initialize correctly, clean up properly, and handle failures gracefully.