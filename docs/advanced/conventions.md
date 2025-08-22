# Convention-Based Service Registration

The elif.rs IoC container supports automatic service registration based on naming conventions and patterns. This reduces boilerplate registration code and ensures consistency across your application.

## Overview

Convention-based registration automatically:
- **Determines service lifetimes** based on naming patterns
- **Discovers interface implementations** using naming conventions  
- **Registers services automatically** without explicit binding code
- **Validates conventions** at compile time
- **Provides consistent patterns** across your application

## Basic Convention Setup

### Default Conventions

The container comes with sensible default conventions:

```rust
use elif_core::container::{ServiceConventions, IocContainerBuilder};

let conventions = ServiceConventions::default();
// Default conventions:
// - *Service -> Singleton
// - *Repository -> Scoped  
// - *Factory -> Transient
// - *Handler -> Transient
// - *Manager -> Singleton

let mut builder = IocContainerBuilder::new();
builder.with_conventions(conventions);

// Auto-register services based on conventions
builder.auto_register_assembly()?;
```

### Custom Conventions

Define your own naming patterns:

```rust
let conventions = ServiceConventions::builder()
    // Lifetime patterns
    .with_lifetime_pattern("*Service", ServiceScope::Singleton)
    .with_lifetime_pattern("*Repository", ServiceScope::Scoped)
    .with_lifetime_pattern("*Cache", ServiceScope::Singleton)
    .with_lifetime_pattern("*Connection", ServiceScope::Scoped)
    .with_lifetime_pattern("*Factory", ServiceScope::Transient)
    .with_lifetime_pattern("*Builder", ServiceScope::Transient)
    
    // Interface patterns  
    .with_interface_pattern("I*", "*Impl")     // IUserService -> UserServiceImpl
    .with_interface_pattern("*Trait", "*")     // UserServiceTrait -> UserService
    .with_interface_pattern("dyn *", "*Impl") // dyn Database -> DatabaseImpl
    
    // Exclude patterns
    .exclude_pattern("*Test*")                 // Don't auto-register test types
    .exclude_pattern("Mock*")                  // Don't auto-register mocks
    
    .build();
```

## Lifetime Conventions

### Pattern-Based Lifetime Assignment

Automatically assign lifetimes based on type names:

```rust
// These services will be automatically registered with appropriate lifetimes:

pub struct UserService;        // Matches *Service -> Singleton
pub struct OrderRepository;    // Matches *Repository -> Scoped  
pub struct CacheManager;       // Matches *Manager -> Singleton
pub struct TokenFactory;       // Matches *Factory -> Transient
pub struct RequestHandler;     // Matches *Handler -> Transient

let mut builder = IocContainerBuilder::new();
builder.with_conventions(conventions);

// Auto-register all types in current crate
builder.auto_register_crate()?;

// Manual verification
assert_eq!(builder.get_lifetime::<UserService>(), ServiceScope::Singleton);
assert_eq!(builder.get_lifetime::<OrderRepository>(), ServiceScope::Scoped);
assert_eq!(builder.get_lifetime::<TokenFactory>(), ServiceScope::Transient);
```

### Custom Lifetime Rules

Create complex lifetime determination logic:

```rust
use elif_core::container::{ConventionRule, ServiceScope};

struct DatabaseConventionRule;

impl ConventionRule for DatabaseConventionRule {
    fn name(&self) -> &'static str {
        "database_convention"
    }
    
    fn get_lifetime(&self, type_name: &str) -> Option<ServiceScope> {
        match type_name {
            // Database connections are scoped to requests/transactions
            name if name.contains("Connection") => Some(ServiceScope::Scoped),
            name if name.contains("Pool") => Some(ServiceScope::Singleton),
            name if name.contains("Migration") => Some(ServiceScope::Transient),
            _ => None, // Let other rules handle it
        }
    }
    
    fn should_register(&self, type_name: &str) -> bool {
        // Only register database-related types
        type_name.contains("Db") || 
        type_name.contains("Database") ||
        type_name.contains("Sql")
    }
}

let conventions = ServiceConventions::builder()
    .add_custom_rule(DatabaseConventionRule)
    .build();
```

## Interface Discovery

### Automatic Interface Binding

Automatically bind interfaces to implementations:

```rust
// Define interfaces and implementations following conventions
pub trait UserRepository: Send + Sync {
    async fn find_user(&self, id: u32) -> Result<User, DbError>;
}

// Convention: I* -> *Impl pattern
pub struct UserRepositoryImpl {
    pool: Arc<PgPool>,
}

impl UserRepository for UserRepositoryImpl {
    async fn find_user(&self, id: u32) -> Result<User, DbError> {
        // Implementation
    }
}

// Auto-registration will create this binding:
// builder.bind::<dyn UserRepository, UserRepositoryImpl>();

let conventions = ServiceConventions::builder()
    .with_interface_pattern("*Repository", "*RepositoryImpl")
    .auto_bind_interfaces(true)
    .build();

builder.with_conventions(conventions);
builder.auto_register_crate()?;

// Interface is automatically available
let repo = container.resolve::<dyn UserRepository>()?;
```

### Complex Interface Patterns

Handle multiple interface patterns:

```rust
struct InterfaceDiscoveryRule;

impl ConventionRule for InterfaceDiscoveryRule {
    fn name(&self) -> &'static str {
        "interface_discovery"
    }
    
    fn discover_interface_bindings(&self, type_name: &str) -> Vec<(String, String)> {
        let mut bindings = Vec::new();
        
        // Pattern 1: *Service -> I*Service interface
        if type_name.ends_with("Service") && !type_name.starts_with('I') {
            let interface = format!("I{}", type_name);
            bindings.push((interface, type_name.to_string()));
        }
        
        // Pattern 2: *Impl -> * interface
        if type_name.ends_with("Impl") {
            let interface = type_name.strip_suffix("Impl").unwrap().to_string();
            bindings.push((interface, type_name.to_string()));
        }
        
        // Pattern 3: Postgres* -> dyn * interface  
        if type_name.starts_with("Postgres") {
            let interface = format!("dyn {}", &type_name[8..]); // Remove "Postgres" prefix
            bindings.push((interface, type_name.to_string()));
        }
        
        bindings
    }
}
```

## Named Service Conventions

### Automatic Named Service Registration

Generate named services based on patterns:

```rust
struct NamedServiceRule;

impl ConventionRule for NamedServiceRule {
    fn name(&self) -> &'static str {
        "named_service_convention"
    }
    
    fn get_service_names(&self, type_name: &str) -> Vec<String> {
        let mut names = Vec::new();
        
        // Pattern: *Cache services get named by type
        if type_name.contains("Cache") {
            if type_name.contains("Redis") {
                names.push("redis".to_string());
            }
            if type_name.contains("Memory") {
                names.push("memory".to_string());
            }
            names.push("default".to_string());
        }
        
        // Pattern: *Repository services get named by domain
        if type_name.ends_with("Repository") {
            let domain = type_name.strip_suffix("Repository").unwrap().to_lowercase();
            names.push(domain);
        }
        
        names
    }
}

// This creates multiple named registrations:
// builder.bind_named::<dyn Cache, RedisCache>("redis");
// builder.bind_named::<dyn Cache, RedisCache>("default");
// builder.bind_named::<dyn UserRepository, UserRepository>("user");
```

## Auto-Discovery Scopes

### Crate-Level Discovery

Register all types in your crate:

```rust
// Register all public types in current crate
builder.auto_register_crate()?;

// Register types from specific modules
builder.auto_register_module("myapp::services")?;
builder.auto_register_module("myapp::repositories")?;

// Register with filtering
builder.auto_register_crate_with_filter(|type_name| {
    // Only register services and repositories
    type_name.ends_with("Service") || type_name.ends_with("Repository")
})?;
```

### Assembly-Level Discovery

Register types across multiple crates:

```rust
// Register from multiple crates
builder.auto_register_assemblies(&[
    "myapp_core",
    "myapp_services", 
    "myapp_repositories"
])?;

// With conventions applied across all assemblies
let conventions = ServiceConventions::builder()
    .with_lifetime_pattern("*Service", ServiceScope::Singleton)
    .build();

builder
    .with_conventions(conventions)
    .auto_register_assemblies(&["myapp_core", "myapp_web"])?;
```

## Validation and Diagnostics

### Convention Validation

Validate that your conventions are working correctly:

```rust
use elif_core::container::{ConventionValidator, ConventionReport};

let validator = ConventionValidator::new(&conventions);
let report = validator.validate_crate()?;

match report.status {
    ConventionStatus::Valid => {
        println!("✅ All conventions applied successfully");
        println!("Registered {} services", report.registered_services);
        println!("Created {} interface bindings", report.interface_bindings);
    }
    ConventionStatus::HasWarnings => {
        println!("⚠️  Conventions applied with warnings:");
        for warning in &report.warnings {
            println!("  {}", warning.message);
        }
    }
    ConventionStatus::HasErrors => {
        println!("❌ Convention errors:");
        for error in &report.errors {
            println!("  {}", error.message);
        }
    }
}
```

### Convention Diagnostics

Get detailed information about convention application:

```rust
let diagnostics = validator.get_diagnostics();

println!("Convention Diagnostics:");
println!("  Types scanned: {}", diagnostics.types_scanned);
println!("  Services registered: {}", diagnostics.services_registered);
println!("  Interfaces bound: {}", diagnostics.interfaces_bound);
println!("  Named services created: {}", diagnostics.named_services);

// Show which conventions matched which types
for (type_name, matched_rules) in &diagnostics.rule_matches {
    println!("  {}: matched rules {:?}", type_name, matched_rules);
}

// Show types that didn't match any conventions
if !diagnostics.unmatched_types.is_empty() {
    println!("  Unmatched types: {:?}", diagnostics.unmatched_types);
}
```

## Advanced Convention Patterns

### Environment-Specific Conventions

Use different conventions based on environment:

```rust
fn create_conventions() -> ServiceConventions {
    let mut builder = ServiceConventions::builder();
    
    // Base conventions
    builder
        .with_lifetime_pattern("*Service", ServiceScope::Singleton)
        .with_lifetime_pattern("*Repository", ServiceScope::Scoped);
    
    // Environment-specific conventions
    match env::var("ELIF_ENV").as_deref() {
        Ok("production") => {
            builder
                .with_interface_pattern("*", "*Impl")  // Use real implementations
                .exclude_pattern("Mock*")              // Exclude mocks
                .exclude_pattern("*Test*");            // Exclude test types
        }
        Ok("test") => {
            builder
                .with_interface_pattern("*", "Mock*")  // Prefer mocks
                .include_pattern("Mock*")              // Include mocks
                .include_pattern("*Test*");            // Include test types
        }
        _ => {
            // Development defaults
            builder
                .with_interface_pattern("*", "*Impl")
                .include_pattern("Mock*");             // Allow both real and mock
        }
    }
    
    builder.build()
}
```

### Modular Conventions

Apply different conventions to different modules:

```rust
let conventions = ServiceConventions::builder()
    // Web layer conventions
    .with_module_pattern("myapp::controllers::*")
    .with_lifetime_pattern("*Controller", ServiceScope::Scoped)
    .with_interface_pattern("*Controller", "*Controller") // Self-registration
    
    // Service layer conventions  
    .with_module_pattern("myapp::services::*")
    .with_lifetime_pattern("*Service", ServiceScope::Singleton)
    .with_interface_pattern("I*Service", "*Service")
    
    // Data layer conventions
    .with_module_pattern("myapp::repositories::*")
    .with_lifetime_pattern("*Repository", ServiceScope::Scoped)
    .with_interface_pattern("*Repository", "*RepositoryImpl")
    
    .build();
```

### Custom Registration Logic

Implement complex registration logic:

```rust
struct EventHandlerConvention;

impl ConventionRule for EventHandlerConvention {
    fn name(&self) -> &'static str {
        "event_handler_convention"
    }
    
    fn custom_registration(&self, type_name: &str, builder: &mut dyn ServiceBinder) -> bool {
        if type_name.ends_with("EventHandler") {
            // Register as collection item
            if type_name.contains("Email") {
                builder.bind_to_collection::<dyn EventHandler, EmailEventHandler>();
            } else if type_name.contains("Sms") {
                builder.bind_to_collection::<dyn EventHandler, SmsEventHandler>();
            }
            
            // Also register individually
            builder.bind_transient_by_name(type_name);
            
            true // Handled by this convention
        } else {
            false // Let other conventions handle it
        }
    }
}
```

## Best Practices

### 1. Start with Simple Conventions

Begin with basic patterns and add complexity as needed:

```rust
// Start simple
let conventions = ServiceConventions::builder()
    .with_lifetime_pattern("*Service", ServiceScope::Singleton)
    .with_lifetime_pattern("*Repository", ServiceScope::Scoped)
    .build();

// Add complexity gradually
let conventions = ServiceConventions::builder()
    .with_lifetime_pattern("*Service", ServiceScope::Singleton)
    .with_lifetime_pattern("*Repository", ServiceScope::Scoped)
    .with_interface_pattern("I*", "*Impl")     // Add interface patterns
    .exclude_pattern("*Test*")                 // Add exclusions
    .build();
```

### 2. Document Your Conventions

Make conventions explicit and well-documented:

```rust
/// Application Service Registration Conventions
/// 
/// Lifetime Patterns:
/// - *Service -> Singleton (stateless business logic)
/// - *Repository -> Scoped (database connections)  
/// - *Cache -> Singleton (shared cache instances)
/// - *Factory -> Transient (object creation)
/// 
/// Interface Patterns:
/// - I*Service -> *ServiceImpl
/// - *Repository -> *RepositoryImpl
/// 
/// Exclusions:
/// - Mock* types (test-only)
/// - *Test* types (test utilities)
pub fn create_app_conventions() -> ServiceConventions {
    ServiceConventions::builder()
        .with_lifetime_pattern("*Service", ServiceScope::Singleton)
        .with_lifetime_pattern("*Repository", ServiceScope::Scoped)
        .with_lifetime_pattern("*Cache", ServiceScope::Singleton) 
        .with_lifetime_pattern("*Factory", ServiceScope::Transient)
        .with_interface_pattern("I*Service", "*ServiceImpl")
        .with_interface_pattern("*Repository", "*RepositoryImpl")
        .exclude_pattern("Mock*")
        .exclude_pattern("*Test*")
        .build()
}
```

### 3. Validate Conventions in Tests

Ensure conventions work as expected:

```rust
#[test]
fn test_convention_application() {
    let conventions = create_app_conventions();
    let mut builder = IocContainerBuilder::new();
    
    builder.with_conventions(conventions);
    builder.auto_register_crate()?;
    
    let container = builder.build()?;
    
    // Verify expected services are registered
    assert!(container.contains::<UserService>());
    assert!(container.contains::<dyn UserRepository>());
    assert!(container.contains::<CacheManager>());
    
    // Verify lifetimes are correct
    assert_eq!(container.get_service_lifetime::<UserService>(), ServiceScope::Singleton);
    assert_eq!(container.get_service_lifetime::<dyn UserRepository>(), ServiceScope::Scoped);
}
```

### 4. Use Convention Diagnostics

Regularly check convention effectiveness:

```rust
#[test]
fn analyze_convention_coverage() {
    let conventions = create_app_conventions();
    let validator = ConventionValidator::new(&conventions);
    
    let diagnostics = validator.get_diagnostics();
    
    // Ensure good coverage
    let coverage = diagnostics.services_registered as f64 / diagnostics.types_scanned as f64;
    assert!(coverage > 0.8, "Convention coverage too low: {:.2}%", coverage * 100.0);
    
    // Check for unmatched types that should be matched
    for unmatched in &diagnostics.unmatched_types {
        if unmatched.ends_with("Service") || unmatched.ends_with("Repository") {
            panic!("Important type not matched by conventions: {}", unmatched);
        }
    }
}
```

Convention-based registration in elif.rs eliminates boilerplate code while ensuring consistent service registration patterns across your application. By following these patterns and best practices, you can build maintainable, scalable applications with minimal configuration overhead.