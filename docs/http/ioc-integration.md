# HTTP and IoC Integration

elif.rs provides seamless integration between the HTTP layer and IoC container, enabling automatic dependency injection for controllers, middleware, and request handlers with zero ceremony.

## Controller Dependency Injection

### Basic Controller with Dependencies

Controllers automatically receive dependencies through the `#[inject]` macro:

```rust
use elif_http_derive::{controller, inject, get, post};
use elif_core_derive::injectable;

// Services that controllers depend on
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    validator: Arc<UserValidator>,
}

#[injectable]
pub struct AuthService {
    jwt_handler: Arc<JwtHandler>,
    session_store: Arc<dyn SessionStore>,
}

// Controller with injected dependencies
#[inject(
    user_service: UserService,
    auth_service: AuthService,
    metrics: Option<MetricsCollector>, // Optional dependency
    config: AppConfig                   // Configuration injection
)]
#[controller("/api/users")]
pub struct UserController;

impl UserController {
    #[get("/{id}")]
    pub async fn get_user(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        // Validate authentication
        let user_context = self.auth_service.authenticate(&req).await?;
        
        // Get user data
        let user_id: u32 = req.path_param("id")?;
        let user = self.user_service.get_user(user_id).await?;
        
        // Optional metrics
        if let Some(metrics) = &self.metrics {
            metrics.increment("api.users.get");
        }
        
        Ok(ElifResponse::ok().json(&user)?)
    }
    
    #[post("")]
    pub async fn create_user(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let create_data: CreateUserDto = req.json().await?;
        
        // Use injected services
        let user = self.user_service.create_user(create_data).await?;
        
        Ok(ElifResponse::created().json(&user)?)
    }
}
```

### Controller Registration

Register controllers with the HTTP server using the IoC container:

```rust
use elif_http::{Server, Router};
use elif_core::container::{IocContainerBuilder, ServiceBinder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build IoC container
    let mut container = IocContainerBuilder::new();
    
    // Register services
    container
        .bind_injectable::<UserService>()
        .bind_injectable::<AuthService>()
        .bind::<dyn UserRepository, PostgresUserRepository>()
        .bind::<UserValidator, UserValidator>()
        .bind::<JwtHandler, JwtHandler>()
        .bind::<dyn SessionStore, RedisSessionStore>()
        .bind_instance::<AppConfig, _>(AppConfig::from_env()?)
        .bind::<MetricsCollector, MetricsCollector>();
    
    let container = container.build()?;
    
    // Create router with IoC-powered controllers
    let mut router = Router::new();
    
    // Controllers are resolved from IoC container automatically
    router.controller_from_container::<UserController>(&container)?;
    
    // Start server
    Server::new()
        .with_router(router)
        .with_container(container) // Provide container to server
        .listen("0.0.0.0:3000")
        .await?;
    
    Ok(())
}
```

## Request-Scoped Services

Services can be scoped to individual HTTP requests, enabling per-request state and context:

### Request Context Service

```rust
#[injectable]
pub struct RequestContext {
    request_id: String,
    user_id: Option<u32>,
    tenant_id: Option<String>,
    start_time: Instant,
    correlation_id: String,
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            user_id: None,
            tenant_id: None,
            start_time: Instant::now(),
            correlation_id: Uuid::new_v4().to_string(),
        }
    }
    
    pub fn with_user(mut self, user_id: u32) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

// Register as scoped service
container.bind_scoped::<RequestContext, RequestContext>();
```

### Using Request-Scoped Services

```rust
#[inject(
    user_service: UserService,
    context: RequestContext  // Automatically scoped to request
)]
#[controller("/api")]
pub struct ApiController;

impl ApiController {
    #[get("/profile")]
    pub async fn get_profile(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        // Context is unique per request
        info!("Request {} started", self.context.request_id);
        
        let user_id = self.context.user_id
            .ok_or_else(|| HttpError::Unauthorized)?;
        
        let profile = self.user_service.get_profile(user_id).await?;
        
        info!("Request {} completed in {:?}", 
              self.context.request_id, 
              self.context.elapsed());
        
        Ok(ElifResponse::ok().json(&profile)?)
    }
}
```

## Middleware with Dependency Injection

Middleware can also receive dependencies through the IoC container:

### IoC-Enabled Middleware

```rust
use elif_http::middleware::{Middleware, MiddlewareContext};

#[injectable]
pub struct AuthMiddleware {
    auth_service: Arc<AuthService>,
    session_store: Arc<dyn SessionStore>,
    config: Arc<AuthConfig>,
}

impl AuthMiddleware {
    pub async fn handle(
        &self,
        req: ElifRequest,
        next: Next,
        ctx: &mut MiddlewareContext,
    ) -> HttpResult<ElifResponse> {
        // Extract authorization header
        let auth_header = req.header("authorization")
            .ok_or_else(|| HttpError::Unauthorized.with_message("Missing authorization header"))?;
        
        // Validate token using injected auth service
        let token = auth_header.strip_prefix("Bearer ")
            .ok_or_else(|| HttpError::Unauthorized.with_message("Invalid authorization format"))?;
        
        let user_context = self.auth_service.validate_token(token).await?;
        
        // Store user context in request scope
        ctx.set("user_id", user_context.user_id);
        ctx.set("permissions", user_context.permissions);
        
        // Continue to next middleware/handler
        next(req).await
    }
}

// Register middleware with IoC container
container.bind_injectable::<AuthMiddleware>();
```

### Middleware Registration

```rust
use elif_http::middleware::MiddlewareRegistry;

// Create middleware registry with IoC container
let mut middleware_registry = MiddlewareRegistry::new(&container);

// Register middleware - resolved from IoC container
middleware_registry.register::<AuthMiddleware>("auth")?;
middleware_registry.register::<LoggingMiddleware>("logging")?;
middleware_registry.register::<CorsMiddleware>("cors")?;

// Apply to router
router
    .middleware_from_registry(&middleware_registry, &["logging", "cors"])
    .controller_from_container::<UserController>(&container)?
    .middleware_from_registry(&middleware_registry, &["auth"]) // Auth after public endpoints
    .controller_from_container::<AdminController>(&container)?;
```

## Advanced HTTP Integration

### Custom Request Factory

Create custom request objects with dependency injection:

```rust
#[injectable]
pub struct AuthenticatedRequest {
    base_request: ElifRequest,
    user_context: UserContext,
    permissions: Vec<Permission>,
}

impl AuthenticatedRequest {
    pub fn user_id(&self) -> u32 {
        self.user_context.user_id
    }
    
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p.name == permission)
    }
    
    pub fn require_permission(&self, permission: &str) -> HttpResult<()> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(HttpError::Forbidden.with_message(&format!(
                "Permission '{}' required", permission
            )))
        }
    }
}

// Factory for creating authenticated requests
#[injectable]
pub struct AuthenticatedRequestFactory {
    auth_service: Arc<AuthService>,
}

impl AuthenticatedRequestFactory {
    pub async fn create(&self, req: ElifRequest) -> HttpResult<AuthenticatedRequest> {
        let user_context = self.auth_service.authenticate(&req).await?;
        let permissions = self.auth_service.get_user_permissions(user_context.user_id).await?;
        
        Ok(AuthenticatedRequest {
            base_request: req,
            user_context,
            permissions,
        })
    }
}
```

### Controller Method with Custom Request

```rust
#[inject(
    user_service: UserService,
    request_factory: AuthenticatedRequestFactory
)]
#[controller("/api/admin")]
pub struct AdminController;

impl AdminController {
    #[post("/users/{id}/ban")]
    pub async fn ban_user(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        // Create authenticated request with permissions
        let auth_req = self.request_factory.create(req).await?;
        
        // Require admin permission
        auth_req.require_permission("users.ban")?;
        
        let user_id: u32 = auth_req.base_request.path_param("id")?;
        let ban_data: BanUserDto = auth_req.base_request.json().await?;
        
        self.user_service.ban_user(user_id, ban_data).await?;
        
        Ok(ElifResponse::ok().json(&json!({ 
            "message": "User banned successfully" 
        }))?)
    }
}
```

## Performance Considerations

### Connection Pooling with Scoped Services

Use scoped services for database connections to avoid connection leaks:

```rust
#[injectable]
pub struct DatabaseConnection {
    pool: Arc<PgPool>,
    connection: Option<PoolConnection<Postgres>>,
}

impl DatabaseConnection {
    pub async fn get_connection(&mut self) -> Result<&mut PoolConnection<Postgres>, DbError> {
        if self.connection.is_none() {
            self.connection = Some(self.pool.acquire().await?);
        }
        Ok(self.connection.as_mut().unwrap())
    }
}

// Register as scoped - new connection per request
container.bind_scoped::<DatabaseConnection, DatabaseConnection>();

// Automatic cleanup when request scope ends
impl Drop for DatabaseConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            // Connection automatically returned to pool
        }
    }
}
```

### Lazy Service Resolution

Defer expensive service creation until needed:

```rust
#[injectable]
pub struct LazyAnalyticsService {
    factory: Arc<AnalyticsServiceFactory>,
    instance: Arc<Mutex<Option<Arc<AnalyticsService>>>>,
}

impl LazyAnalyticsService {
    pub async fn get(&self) -> Arc<AnalyticsService> {
        let mut guard = self.instance.lock().await;
        if guard.is_none() {
            *guard = Some(self.factory.create().await);
        }
        guard.as_ref().unwrap().clone()
    }
}
```

## Testing HTTP with IoC

### Integration Tests

Test controllers with mocked dependencies:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use elif_http::testing::TestServer;
    
    mock! {
        UserService {
            async fn get_user(&self, id: u32) -> Result<User, ServiceError>;
            async fn create_user(&self, data: CreateUserDto) -> Result<User, ServiceError>;
        }
    }
    
    #[tokio::test]
    async fn test_get_user_endpoint() {
        // Create container with mocked services
        let mut container = IocContainerBuilder::new();
        
        let mut mock_user_service = MockUserService::new();
        mock_user_service
            .expect_get_user()
            .with(eq(123))
            .times(1)
            .returning(|_| Ok(User { id: 123, name: "John".into() }));
        
        container
            .bind_instance::<UserService, _>(mock_user_service)
            .bind_instance::<AuthService, _>(MockAuthService::new())
            .bind_instance::<AppConfig, _>(AppConfig::test());
        
        let container = container.build()?;
        
        // Create test server with IoC container
        let server = TestServer::new()
            .with_container(container)
            .with_controller::<UserController>();
        
        // Test endpoint
        let response = server
            .get("/api/users/123")
            .header("Authorization", "Bearer test-token")
            .send()
            .await?;
        
        assert_eq!(response.status(), 200);
        
        let user: User = response.json().await?;
        assert_eq!(user.id, 123);
        assert_eq!(user.name, "John");
    }
}
```

### Middleware Testing

Test middleware with dependency injection:

```rust
#[tokio::test]
async fn test_auth_middleware() {
    let mut container = IocContainerBuilder::new();
    
    let mut mock_auth_service = MockAuthService::new();
    mock_auth_service
        .expect_validate_token()
        .with(eq("valid-token"))
        .returning(|_| Ok(UserContext { user_id: 123 }));
    
    container.bind_instance::<AuthService, _>(mock_auth_service);
    
    let container = container.build()?;
    
    let auth_middleware = AuthMiddleware::from_ioc_container(&container, None)?;
    
    // Test middleware behavior
    let req = ElifRequest::builder()
        .header("authorization", "Bearer valid-token")
        .build();
    
    let mut ctx = MiddlewareContext::new();
    
    let next = |req: ElifRequest| async move {
        Ok(ElifResponse::ok().body("Success"))
    };
    
    let response = auth_middleware.handle(req, next, &mut ctx).await?;
    
    assert_eq!(response.status(), 200);
    assert_eq!(ctx.get::<u32>("user_id"), Some(&123));
}
```

## Best Practices

### 1. Use Scoped Services for Request State
Keep request-specific data in scoped services:

```rust
// Good - request-scoped
#[injectable]
pub struct RequestMetrics {
    request_id: String,
    start_time: Instant,
    counters: HashMap<String, u64>,
}

// Avoid - singleton with shared state
#[injectable] 
pub struct GlobalRequestMetrics {
    all_requests: Arc<Mutex<HashMap<String, RequestData>>>, // Shared mutable state
}
```

### 2. Separate Concerns
Keep HTTP concerns separate from business logic:

```rust
// Good - clean separation
#[inject(user_service: UserService)]
#[controller("/users")]
pub struct UserController; // HTTP concerns

#[injectable]
pub struct UserService {    // Business logic
    repository: Arc<dyn UserRepository>,
}

// Avoid - mixing concerns
pub struct UserController {
    db_pool: Arc<PgPool>,    // HTTP controller shouldn't know about database details
}
```

### 3. Use Typed Configurations
Inject typed configuration instead of environment variables:

```rust
// Good - typed configuration
#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiry: Duration,
    pub require_https: bool,
}

#[injectable]
pub struct AuthService {
    config: Arc<AuthConfig>, // Typed, validated config
}

// Avoid - reading environment variables directly
impl AuthService {
    pub fn new() -> Self {
        let jwt_secret = env::var("JWT_SECRET").unwrap(); // Error-prone
        // ...
    }
}
```

### 4. Handle Scope Cleanup
Implement proper cleanup for scoped resources:

```rust
#[injectable]
pub struct RequestLogger {
    file_handle: Arc<Mutex<File>>,
    request_id: String,
}

impl Drop for RequestLogger {
    fn drop(&mut self) {
        // Flush logs when request ends
        if let Ok(mut file) = self.file_handle.try_lock() {
            let _ = file.flush();
        }
    }
}
```

The HTTP and IoC integration in elif.rs provides powerful dependency injection capabilities while maintaining clean separation between HTTP concerns and business logic. This enables testable, maintainable web applications with minimal ceremony.