# IoC Container System Overview

elif.rs features a comprehensive IoC (Inversion of Control) container system that provides enterprise-grade dependency injection with a Laravel-like developer experience. The system is designed to handle complex dependency graphs while maintaining simplicity for common use cases.

## Architecture Overview

The IoC system consists of several interconnected components:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ Controllers │  │ Middleware  │  │  Services   │         │  
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                   IoC Container System                      │
│                                                             │
│  ┌──────────────┐    ┌─────────────┐    ┌──────────────┐   │
│  │ Auto-wiring  │    │   Service   │    │  Lifecycle   │   │
│  │  Injectable  │◄──►│  Registry   │◄──►│ Management   │   │
│  │    Trait     │    │             │    │              │   │
│  └──────────────┘    └─────────────┘    └──────────────┘   │
│          │                   │                   │         │
│  ┌──────────────┐    ┌─────────────┐    ┌──────────────┐   │
│  │   Proc Macro │    │ Dependency  │    │   Scoping    │   │
│  │ #[injectable]│    │ Resolution  │    │   System     │   │
│  └──────────────┘    └─────────────┘    └──────────────┘   │
│                              │                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Validation & Debugging                 │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐ │   │
│  │  │ Dependency  │  │    Graph    │  │ Performance  │ │   │
│  │  │ Validation  │  │ Analyzer    │  │  Profiler    │ │   │
│  │  └─────────────┘  └─────────────┘  └──────────────┘ │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. IocContainer & IocContainerBuilder

The heart of the system, providing service registration and resolution:

```rust
// Builder pattern for container construction
let mut builder = IocContainerBuilder::new();
builder
    .bind::<dyn UserRepository, PostgresUserRepository>()
    .bind_singleton::<EmailService, SmtpEmailService>()
    .bind_injectable::<UserService>();

let container = builder.build()?;

// Service resolution
let user_service = container.resolve::<UserService>()?;
```

**Key Features:**
- Type-safe service registration and resolution
- Multiple service lifetimes (Singleton, Scoped, Transient)
- Named service support
- Factory-based registration
- Comprehensive validation

### 2. Injectable Trait & Proc Macro

Automatic dependency injection through code generation:

```rust
// Trait for auto-wirable services
pub trait Injectable: Send + Sync + 'static {
    fn dependencies() -> Vec<ServiceId>;
    fn create<R: DependencyResolver>(resolver: &R) -> Result<Self, CoreError>;
}

// Proc macro for automatic implementation  
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
    email_service: Arc<EmailService>,
    metrics: Option<Arc<MetricsCollector>>, // Optional dependency
}
// Injectable trait automatically implemented!
```

**Benefits:**
- Zero boilerplate - just add `#[injectable]`
- Compile-time dependency analysis
- Clear error messages for invalid configurations
- Support for required and optional dependencies

### 3. Service Lifetimes

Three service lifetimes to handle different use cases:

```rust
// Singleton - shared across application
builder.bind_singleton::<dyn Cache, RedisCache>();

// Scoped - instance per scope (e.g., HTTP request)  
builder.bind_scoped::<DatabaseConnection, DatabaseConnection>();

// Transient - new instance every time
builder.bind_transient::<RequestId, RequestId>();
```

### 4. Scoping System

Hierarchical scopes for managing service instances:

```rust
// Create scopes
let scope_id = container.create_scope()?;
let child_scope = container.create_child_scope(&scope_id)?;

// Resolve services in scope
let scoped_service = container.resolve_scoped::<DatabaseConnection>(&scope_id)?;

// Automatic cleanup
container.dispose_scope(&scope_id).await?;
```

**Use Cases:**
- HTTP request scoping - isolate services per request
- Database transaction scoping - connection per transaction
- User session scoping - services tied to user sessions

### 5. Service Modules

Organize related services into logical modules:

```rust
pub struct DatabaseModule;

impl ServiceModule for DatabaseModule {
    fn configure_services(&self, container: &mut dyn ServiceBinder) -> Result<(), CoreError> {
        container
            .bind::<dyn UserRepository, PostgresUserRepository>()
            .bind::<dyn OrderRepository, PostgresOrderRepository>()
            .bind_singleton::<PgPool, PgPool>();
        Ok(())
    }
    
    fn depends_on(&self) -> Vec<ModuleId> {
        vec![ModuleId::new("config")]
    }
}

// Register modules
let mut registry = ModuleRegistry::new();
registry.register_module(Box::new(DatabaseModule))?;
registry.configure_container(&mut builder)?;
```

### 6. Validation System

Comprehensive validation to catch issues early:

```rust
// Automatic validation on container build
let container = builder.build()?; // Validates dependencies

// Manual validation
let validator = DependencyValidator::new(&descriptors);
let report = validator.validate();

match report.status {
    ValidationStatus::Valid => println!("All good!"),
    ValidationStatus::HasWarnings => println!("Warnings: {:?}", report.warnings),
    ValidationStatus::Invalid => println!("Errors: {:?}", report.errors),
}
```

**Validation Types:**
- Missing dependencies
- Circular dependencies  
- Lifetime mismatches
- Performance warnings
- Interface binding issues

## Advanced Features

### Convention-Based Registration

Automatic service registration based on naming conventions:

```rust
use elif_core::container::ServiceConventions;

let conventions = ServiceConventions::builder()
    .with_lifetime_pattern("*Service", ServiceScope::Singleton)
    .with_lifetime_pattern("*Repository", ServiceScope::Scoped)
    .with_interface_pattern("I*", "*Impl")
    .build();

builder.with_conventions(conventions);

// Automatically registers based on conventions
builder.auto_register_assembly()?;
```

### Conditional Registration

Environment or configuration-based service selection:

```rust
builder
    .bind_with::<dyn EmailService, SmtpEmailService>()
    .when_env("EMAIL_PROVIDER", "smtp")
    .in_profile("production");

builder
    .bind_with::<dyn EmailService, MockEmailService>()
    .in_profile("test");
```

### Collection Binding

Register multiple implementations for collection injection:

```rust
builder.bind_collection::<dyn EventHandler, _>(|collection| {
    collection
        .add::<EmailEventHandler>()
        .add::<SmsEventHandler>()
        .add_conditional::<SlackEventHandler>(when_feature_enabled("notifications"));
});

#[injectable]
pub struct EventBus {
    handlers: Vec<Arc<dyn EventHandler>>, // Gets all registered handlers
}
```

### Visualization & Debugging

Built-in tools for understanding your dependency graph:

```rust
use elif_core::container::{DependencyVisualizer, VisualizationFormat};

let visualizer = DependencyVisualizer::new(&container);

// Generate different visualization formats
let dot_graph = visualizer.generate(VisualizationFormat::Dot)?;
let mermaid_diagram = visualizer.generate(VisualizationFormat::Mermaid)?;
let ascii_tree = visualizer.generate(VisualizationFormat::Ascii)?;

// Service exploration
let explorer = ServiceExplorer::new(&container);
let dependencies = explorer.get_dependencies_of::<UserService>()?;
let dependents = explorer.get_dependents_of::<UserRepository>()?;
```

## HTTP Integration

Seamless integration with the HTTP layer:

```rust
use elif_http_derive::{controller, inject};

// Controllers with automatic dependency injection
#[inject(
    user_service: UserService,
    auth_service: AuthService,
    metrics: Option<MetricsCollector>
)]
#[controller("/api/users")]
pub struct UserController;

impl UserController {
    #[get("/{id}")]
    pub async fn get_user(&self, req: ElifRequest) -> HttpResult<ElifResponse> {
        let user_id: u32 = req.path_param("id")?;
        let user = self.user_service.get_user(user_id).await?;
        Ok(ElifResponse::ok().json(&user)?)
    }
}

// Server with IoC container
Server::new()
    .with_container(container)
    .controller::<UserController>() // Automatically resolved from container
    .listen("0.0.0.0:3000")
    .await?;
```

## Performance Characteristics

### Memory Efficiency
- **Service reuse** - Singletons shared across entire application
- **Scope isolation** - Scoped services cleaned up automatically  
- **Lazy resolution** - Services created only when needed
- **Arc-based sharing** - Efficient reference counting for shared services

### Runtime Performance
- **Zero allocation resolution** - Type-safe service lookup
- **Compile-time validation** - Errors caught at build time, not runtime
- **Optimized dependency graphs** - Topological sorting for efficient resolution
- **Minimal overhead** - Generated code has no runtime reflection

### Scalability
- **Thread-safe by design** - All services are `Send + Sync`
- **Concurrent resolution** - Multiple threads can resolve services simultaneously
- **Scope-based isolation** - Request isolation prevents cross-contamination
- **Memory bounded** - Automatic cleanup prevents memory leaks

## Comparison with Other DI Frameworks

| Feature | elif.rs IoC | Spring (Java) | NestJS (Node.js) | ASP.NET Core |
|---------|-------------|---------------|------------------|--------------|
| **Zero Boilerplate** | ✅ `#[injectable]` | ❌ Annotations + XML | ✅ `@Injectable()` | ❌ Constructor patterns |
| **Compile-time Validation** | ✅ Full validation | ❌ Runtime errors | ❌ Runtime errors | ⚠️ Partial |  
| **Type Safety** | ✅ Full type safety | ⚠️ Runtime casting | ❌ Any types | ✅ Generic constraints |
| **Scoping** | ✅ Hierarchical scopes | ✅ Multiple scopes | ✅ Request scoping | ✅ Request scoping |
| **Performance** | ✅ Zero allocation | ⚠️ Reflection overhead | ❌ V8 limitations | ⚠️ Some reflection |
| **Learning Curve** | ✅ Intuitive API | ❌ Complex config | ✅ Decorator pattern | ⚠️ Moderate complexity |

## Best Practices Summary

### 1. Design Principles
- **Depend on abstractions** - Use traits/interfaces instead of concrete types
- **Minimize dependencies** - Keep services focused and loosely coupled  
- **Favor composition** - Build complex behavior from simple services
- **Explicit over implicit** - Make dependencies visible and intentional

### 2. Service Design
```rust
// Good - focused, testable service
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,    // Abstract dependency
    validator: Arc<UserValidator>,          // Single responsibility
}

// Avoid - god service with too many concerns
pub struct UserService {
    db_pool: Arc<PgPool>,                   // Too low-level
    email_service: Arc<EmailService>,       
    sms_service: Arc<SmsService>,
    push_service: Arc<PushService>,         // Too many dependencies
    analytics: Arc<Analytics>,
    cache: Arc<Cache>,
    logger: Arc<Logger>,
    config: Arc<AppConfig>,
}
```

### 3. Lifetime Selection
- **Singleton** - Stateless services, expensive-to-create objects
- **Scoped** - Request/transaction context, database connections
- **Transient** - Lightweight objects, stateful per-use services

### 4. Testing Strategy
```rust
#[cfg(test)]
mod tests {
    // Use real container with mocked dependencies
    let mut container = IocContainerBuilder::new();
    container
        .bind_instance::<dyn UserRepository, _>(MockUserRepository::new())
        .bind_injectable::<UserService>();
    
    let user_service = container.resolve::<UserService>()?;
    // Test with injected mocks
}
```

## Getting Started

1. **Add Dependencies**
```toml
[dependencies]
elif-core = "0.5.0"
elif-core-derive = "0.1.0"  # For #[injectable] macro
```

2. **Define Services**
```rust
#[injectable]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
}
```

3. **Register Services**  
```rust
let mut builder = IocContainerBuilder::new();
builder
    .bind::<dyn UserRepository, PostgresUserRepository>()
    .bind_injectable::<UserService>();

let container = builder.build()?;
```

4. **Resolve Services**
```rust
let user_service = container.resolve::<UserService>()?;
```

The IoC container system in elif.rs provides a powerful, type-safe, and performant foundation for building maintainable applications. Its Laravel-inspired API makes dependency injection approachable while offering enterprise-grade features for complex scenarios.

## Further Reading

### Core Topics
- [Basic Dependency Injection](../basics/dependency-injection.md) - Getting started guide
- [Service Modules](service-modules.md) - Organizing services into modules  
- [Container Validation](container-validation.md) - Validating dependency graphs

### Advanced Topics
- [Performance Optimization](performance-optimization.md) - Optimizing container performance
- [Thread Safety & Concurrency](thread-safety-concurrency.md) - Multi-threaded applications
- [Debugging & Introspection](debugging-introspection.md) - Debugging and monitoring tools
- [Convention-Based Registration](conventions.md) - Automatic service registration
- [Lifecycle Management](lifecycle-management.md) - Service initialization and cleanup
- [Troubleshooting](troubleshooting.md) - Common issues and solutions

### Integration Guides
- [HTTP Integration](../http/ioc-integration.md) - Using IoC with HTTP controllers