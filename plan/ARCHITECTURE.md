# elif.rs Architecture Design

## System Overview

elif.rs follows a modular, dependency-injection-based architecture inspired by Laravel and NestJS, but optimized for Rust's type system and AI agent development.

## Core Architecture Principles

### 1. Dependency Injection First
Everything in the framework is resolved through a central service container, enabling:
- Testable components through interface injection
- Plugin architecture for extensibility
- Clean separation of concerns

### 2. Module-Based Organization
Applications are organized into feature modules (similar to NestJS modules):
```rust
pub struct UserModule;
impl Module for UserModule {
    fn register(&self, container: &mut Container) {
        container.bind::<UserRepository, DatabaseUserRepository>();
        container.bind::<UserService, DefaultUserService>();
    }
    
    fn routes(&self) -> Vec<Route> {
        vec![
            Route::get("/users", UserController::index),
            Route::post("/users", UserController::store),
        ]
    }
}
```

### 3. AI-Safe Design Patterns
- **MARKER Blocks**: Designated areas for AI code modification
- **Rich Introspection**: Runtime metadata for AI understanding
- **Semantic Routing**: Routes carry business meaning, not just HTTP paths
- **Context-Aware Generation**: Framework provides context for intelligent scaffolding

## System Components

### Core Framework Layer
```rust
elif-core/
├── application.rs       # Application bootstrapping and lifecycle
├── container.rs         # Dependency injection container
├── module.rs           # Module system and registration
├── config.rs           # Configuration management
└── lifecycle.rs        # Application lifecycle hooks
```

### HTTP Layer
```rust
elif-http/
├── router.rs           # Request routing with middleware support
├── controller.rs       # Base controller with validation
├── middleware.rs       # Middleware stack and composition
├── request.rs          # Request abstraction with validation
├── response.rs         # Response abstraction with transformation
└── server.rs          # HTTP server with graceful shutdown
```

### Database Layer
```rust
elif-db/
├── model.rs           # Base model with relationships
├── query.rs           # Query builder with type safety
├── migration.rs       # Schema migrations with rollbacks
├── connection.rs      # Connection pooling and management
├── repository.rs      # Repository pattern implementation
└── events.rs         # Model events and observers
```

### Security Layer
```rust
elif-auth/
├── guard.rs           # Authentication guards (JWT, session, API)
├── policy.rs          # Authorization policies
├── middleware.rs      # Security middleware (CORS, CSRF, rate limiting)
├── validation.rs      # Input validation and sanitization
└── hash.rs           # Password hashing and verification
```

## Request Lifecycle

```
1. HTTP Request → Router
2. Router → Middleware Stack
3. Middleware → Controller Resolution (via DI)
4. Controller → Service Layer (via DI)
5. Service → Repository Layer (via DI)
6. Repository → Database/External APIs
7. Response → Middleware Stack (reverse order)
8. Middleware → HTTP Response
```

## Dependency Injection Container

### Service Registration
```rust
// In Service Providers
container.bind::<UserRepository, DatabaseUserRepository>();
container.singleton::<Database, PostgresDatabase>();
container.factory::<HttpClient, || HttpClient::new());

// Resolution
let user_repo = container.resolve::<UserRepository>();
let db = container.resolve::<Database>(); // Same instance every time
```

### Scoped Services
- **Singleton**: Created once, shared across application lifetime
- **Scoped**: Created once per HTTP request
- **Transient**: Created fresh every time resolved

## Module System

### Module Definition
```rust
pub trait Module {
    fn register(&self, container: &mut Container);
    fn boot(&self, app: &Application) {}
    fn routes(&self) -> Vec<Route> { vec![] }
    fn middleware(&self) -> Vec<Box<dyn Middleware>> { vec![] }
    fn providers(&self) -> Vec<Box<dyn ServiceProvider>> { vec![] }
}
```

### Module Registration
```rust
let app = Application::builder()
    .module(AuthModule)
    .module(UserModule)
    .module(BlogModule)
    .build();
```

## Configuration System

### Environment-based Configuration
```rust
#[derive(Config)]
pub struct AppConfig {
    #[config(env = "APP_NAME")]
    pub name: String,
    
    #[config(env = "DATABASE_URL")]
    pub database_url: String,
    
    #[config(default = "info")]
    pub log_level: LogLevel,
}
```

### Configuration Validation
All configuration is validated at application startup, preventing runtime configuration errors.

## Error Handling Strategy

### Layered Error Handling
1. **Domain Errors**: Business logic errors (UserNotFound, InvalidCredentials)
2. **Infrastructure Errors**: Database, network, file system errors
3. **Framework Errors**: Routing, validation, serialization errors
4. **System Errors**: Out of memory, permission denied

### Error Response Structure
```json
{
    "error": {
        "code": "USER_NOT_FOUND",
        "message": "User with ID 123 not found",
        "hint": "Check that the user ID is correct",
        "details": {
            "user_id": 123,
            "attempted_at": "2025-01-13T10:30:00Z"
        }
    }
}
```

## AI Integration Architecture

### MARKER Block System
```rust
// <<<ELIF:BEGIN agent-editable:user_create>>>
pub async fn create_user(&self, data: CreateUserDto) -> Result<User> {
    // AI can safely edit this block
    let user = self.user_repository.create(data).await?;
    self.event_dispatcher.dispatch(UserCreated::new(user.id));
    Ok(user)
}
// <<<ELIF:END agent-editable:user_create>>>
```

### Introspection APIs
- `/_map.json`: Project structure and route mapping
- `/_schema.json`: Database schema and relationships  
- `/_config.json`: Configuration structure and validation rules
- `/_health`: Application and dependency health status

### Context-Aware Generation
The framework provides rich metadata for intelligent code generation:
```json
{
    "models": [
        {
            "name": "User",
            "fields": [
                {"name": "email", "type": "String", "validations": ["email", "unique"]}
            ],
            "relationships": [
                {"type": "HasMany", "target": "Post", "foreign_key": "user_id"}
            ]
        }
    ],
    "routes": [
        {
            "path": "/users/{id}",
            "method": "GET", 
            "controller": "UserController::show",
            "middleware": ["auth", "throttle:60,1"]
        }
    ]
}
```

## Performance Considerations

### Async-First Design
All I/O operations are async, supporting high-concurrency workloads.

### Connection Pooling
Database connections are pooled and managed automatically.

### Lazy Loading
Services are only instantiated when first requested.

### Caching Strategy
- **Query Caching**: Database queries cached automatically
- **Route Caching**: Route resolution cached in production
- **Config Caching**: Configuration parsed once and cached

## Security Architecture

### Defense in Depth
1. **Input Validation**: All inputs validated and sanitized
2. **Authentication**: Multi-provider authentication system
3. **Authorization**: Policy-based access control
4. **Rate Limiting**: Per-route and global rate limiting
5. **HTTPS Only**: TLS required in production
6. **Security Headers**: Comprehensive security headers

### Threat Mitigation
- **SQL Injection**: Parameterized queries only
- **XSS**: Output encoding and CSP headers
- **CSRF**: Token validation for state-changing operations
- **DoS**: Rate limiting and request size limits
- **Data Exposure**: Field-level authorization

## Testing Strategy

### Test Types
1. **Unit Tests**: Individual component testing
2. **Integration Tests**: Module interaction testing
3. **Feature Tests**: End-to-end HTTP testing
4. **Performance Tests**: Load and stress testing

### Test Support
- Database transactions for isolated tests
- Mock services for external dependencies
- Test factories for generating test data
- HTTP test client for API testing

## Deployment Architecture

### Container Support
Framework applications can be containerized with minimal configuration.

### Cloud Native
Support for health checks, metrics, and graceful shutdown.

### Horizontal Scaling
Stateless design enables horizontal scaling without code changes.

---

This architecture provides a solid foundation for building the production-ready framework while maintaining the AI-native design philosophy.