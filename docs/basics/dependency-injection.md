# Dependency Injection

elif.rs provides a powerful and intuitive IoC (Inversion of Control) container system inspired by modern frameworks like NestJS and Laravel. The container handles service registration, dependency resolution, and lifecycle management with zero ceremony.

## Quick Start

### Basic Service Registration

```rust
use elif_core::container::{IocContainerBuilder, ServiceBinder};

// Create and configure container
let mut builder = IocContainerBuilder::new();

builder
    .bind::<UserRepository, PostgresUserRepository>()  // Interface → Implementation
    .bind_singleton::<EmailService, SmtpEmailService>() // Singleton lifetime
    .bind_transient::<Logger, FileLogger>();            // New instance each time

let container = builder.build()?;

// Resolve services
let user_repo = container.resolve::<UserRepository>()?;
let email_service = container.resolve::<EmailService>()?;
```

### Injectable Decorator (NestJS-Style)

The `#[injectable]` macro eliminates boilerplate by automatically analyzing dependencies:

```rust
use elif_core_derive::injectable;

#[injectable]
pub struct UserService {
    user_repo: Arc<UserRepository>,          // Required dependency
    email_service: Arc<EmailService>,        // Required dependency  
    metrics: Option<Arc<MetricsCollector>>,  // Optional dependency
}

impl UserService {
    pub async fn create_user(&self, data: CreateUserDto) -> Result<User, AppError> {
        let user = self.user_repo.create(data).await?;
        
        if let Some(metrics) = &self.metrics {
            metrics.increment("users.created");
        }
        
        self.email_service.send_welcome_email(&user).await?;
        Ok(user)
    }
}

// Register and resolve
builder.bind_injectable::<UserService>();
let service = container.resolve::<UserService>()?;
```

## Service Lifetimes

### Singleton
Single instance shared across the entire application:

```rust
builder.bind_singleton::<dyn Cache, RedisCache>();

// Both resolve to the same instance
let cache1 = container.resolve::<dyn Cache>()?;
let cache2 = container.resolve::<dyn Cache>()?;
assert!(Arc::ptr_eq(&cache1, &cache2)); // true
```

### Scoped  
Instance per scope (typically per HTTP request):

```rust
builder.bind_scoped::<dyn DatabaseContext, PgContext>();

// Different instances per scope
let scope1 = container.create_scope()?;
let scope2 = container.create_scope()?;

let ctx1 = container.resolve_scoped::<dyn DatabaseContext>(&scope1)?;
let ctx2 = container.resolve_scoped::<dyn DatabaseContext>(&scope2)?;
// ctx1 and ctx2 are different instances
```

### Transient
New instance every time:

```rust
builder.bind_transient::<RequestId, RequestId>();

let id1 = container.resolve::<RequestId>()?;
let id2 = container.resolve::<RequestId>()?;
// id1 and id2 are always different instances
```

## Advanced Registration

### Named Services
Multiple implementations of the same interface:

```rust
builder
    .bind_named::<dyn Cache, RedisCache>("redis")
    .bind_named::<dyn Cache, MemoryCache>("memory")
    .bind::<dyn Cache, RedisCache>(); // Default implementation

// Resolve specific implementations
let redis_cache = container.resolve_named::<dyn Cache>("redis")?;
let memory_cache = container.resolve_named::<dyn Cache>("memory")?;
let default_cache = container.resolve::<dyn Cache>()?; // Gets RedisCache
```

### Factory Registration
Custom creation logic:

```rust
builder.bind_factory::<DatabaseConfig, _, _>(|| {
    Ok(DatabaseConfig {
        url: env::var("DATABASE_URL")?,
        max_connections: 10,
        timeout: Duration::from_secs(30),
    })
});

// Or with container access
builder.bind_factory_with_container::<UserService, _, _>(|container| {
    let repo = container.resolve::<dyn UserRepository>()?;
    let email = container.resolve::<dyn EmailService>()?;
    Ok(UserService::new(repo, email))
});
```

### Instance Registration
Pre-created objects:

```rust
let config = AppConfig::from_file("app.toml")?;
builder.bind_instance::<AppConfig, _>(config);

// Useful for configuration objects, loggers, etc.
```

## Conditional Registration

Register services based on environment or configuration:

```rust
use elif_core::container::{EnvCondition, ConditionalBinding};

builder
    .bind_with::<dyn Cache, RedisCache>()
    .when_env("CACHE_PROVIDER", "redis")
    .in_profile("production")
    .as_default();

builder
    .bind_with::<dyn Cache, MemoryCache>()
    .when_env("CACHE_PROVIDER", "memory")
    .in_profile("development");

// Automatically selects the right implementation
```

## Collection Bindings

Register multiple implementations for collection injection:

```rust
builder.bind_collection::<dyn EventHandler, _>(|collection| {
    collection
        .add::<EmailEventHandler>()
        .add::<SmsEventHandler>()
        .add_named::<SlackEventHandler>("slack")
        .add_conditional::<PushEventHandler>(when_feature_enabled("push"));
});

#[injectable]
pub struct EventBus {
    handlers: Vec<Arc<dyn EventHandler>>, // Gets all registered handlers
}
```

## HTTP Integration

### Controller Dependency Injection

Controllers automatically receive dependencies through the IoC container:

```rust
use elif_http_derive::{controller, inject, get, post};

#[inject(
    user_service: UserService,
    auth_service: AuthService,
    metrics: Option<MetricsService>
)]
#[controller("/users")]
pub struct UserController;

impl UserController {
    #[get("/{id}")]
    pub async fn get_user(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let user_id: u32 = req.path_param("id")?;
        
        // Use injected services
        self.auth_service.require_permission(&req, "users.read")?;
        let user = self.user_service.get_user(user_id).await?;
        
        if let Some(metrics) = &self.metrics {
            metrics.increment("api.users.get");
        }
        
        Ok(ElifResponse::ok().json(&user)?)
    }
    
    #[post("")]  
    pub async fn create_user(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let data: CreateUserDto = req.json().await?;
        let user = self.user_service.create_user(data).await?;
        Ok(ElifResponse::created().json(&user)?)
    }
}
```

### Request-Scoped Services

Services can be scoped to individual HTTP requests:

```rust
#[injectable]
pub struct RequestContext {
    request_id: String,
    user_id: Option<u32>,
    tenant_id: String,
}

// Register as scoped - new instance per request
builder.bind_scoped::<RequestContext, RequestContext>();

// Automatically available in controllers
#[inject(context: RequestContext)]
#[controller("/api")]
pub struct ApiController;
```

## Testing with Mocks

Override services for testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    
    mock! {
        UserService {
            async fn get_user(&self, id: u32) -> Result<User, AppError>;
            async fn create_user(&self, data: CreateUserDto) -> Result<User, AppError>;
        }
    }
    
    #[tokio::test]
    async fn test_user_controller() {
        let mut container = IocContainerBuilder::new();
        
        // Use mock instead of real service
        let mut mock_service = MockUserService::new();
        mock_service
            .expect_get_user()
            .with(eq(123))
            .times(1)
            .returning(|_| Ok(User { id: 123, name: "John".into() }));
        
        container.bind_instance::<UserService, _>(mock_service);
        let container = container.build()?;
        
        // Test controller with mocked dependencies
        let controller = UserController::from_ioc_container(&container, None)?;
        // ... test implementation
    }
}
```

## Error Handling

The container provides detailed error messages for common issues:

```rust
// Service not registered
match container.resolve::<UnregisteredService>() {
    Err(CoreError::ServiceNotFound { service_type }) => {
        println!("Service not found: {}", service_type);
    }
    _ => unreachable!(),
}

// Circular dependencies detected at build time
match builder.build() {
    Err(CoreError::CircularDependency { cycle }) => {
        println!("Circular dependency: {:?}", cycle);
    }
    _ => {},
}

// Missing dependencies for Injectable services
match container.resolve::<ServiceWithMissingDeps>() {
    Err(CoreError::DependencyResolution { service, missing }) => {
        println!("Service {} missing dependency: {}", service, missing);
    }
    _ => {},
}
```

## Best Practices

### 1. Use Interfaces
Define traits for your services to enable testing and flexibility:

```rust
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: u32) -> Result<Option<User>, DbError>;
    async fn create(&self, user: CreateUserDto) -> Result<User, DbError>;
}

pub struct PostgresUserRepository {
    pool: Arc<PgPool>,
}

impl UserRepository for PostgresUserRepository {
    // Implementation...
}

// Register interface → implementation
builder.bind::<dyn UserRepository, PostgresUserRepository>();
```

### 2. Minimize Constructor Dependencies  
Keep services focused with minimal dependencies:

```rust
// Good - focused service
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    validator: Arc<UserValidator>,
}

// Avoid - too many dependencies suggests service is doing too much
#[injectable] 
pub struct GodService {
    user_repo: Arc<dyn UserRepository>,
    email_service: Arc<dyn EmailService>,
    sms_service: Arc<dyn SmsService>,
    push_service: Arc<dyn PushService>,
    analytics: Arc<dyn Analytics>,
    cache: Arc<dyn Cache>,
    logger: Arc<dyn Logger>,
    // ... 10+ more dependencies
}
```

### 3. Use Scoped Services for Request Context
Keep request-specific data in scoped services:

```rust
#[injectable]
pub struct RequestContext {
    request_id: String,
    authenticated_user: Option<User>,
    permissions: Vec<Permission>,
}

// Register as scoped
builder.bind_scoped::<RequestContext, RequestContext>();
```

### 4. Configuration-Driven Service Selection
Use conditional bindings for environment-specific services:

```rust
// Production
builder
    .bind_with::<dyn EmailService, SmtpEmailService>()
    .when_env("EMAIL_PROVIDER", "smtp")
    .in_profile("production");

// Development  
builder
    .bind_with::<dyn EmailService, LogEmailService>()
    .when_env("EMAIL_PROVIDER", "log")
    .in_profile("development");

// Testing
builder
    .bind_with::<dyn EmailService, MockEmailService>()
    .in_profile("test");
```

The IoC container in elif.rs provides enterprise-grade dependency injection with a Laravel-like developer experience. It handles complex scenarios while maintaining simplicity for common use cases.

## Next Steps

- [Service Modules](../advanced/service-modules.md) - Organize services into modules
- [Container Validation](../advanced/container-validation.md) - Validate dependency graphs
- [HTTP Integration](../http/ioc-integration.md) - Deep dive into HTTP layer integration