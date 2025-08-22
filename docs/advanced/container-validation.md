# Container Validation

The elif.rs IoC container provides comprehensive validation to catch dependency issues at build time rather than runtime. This helps ensure your application starts correctly and all dependencies are properly configured.

## Overview

Container validation catches:
- **Missing dependencies** - Services that depend on unregistered services
- **Circular dependencies** - Services that depend on each other in a loop
- **Lifetime mismatches** - Invalid lifetime combinations
- **Interface binding issues** - Unbound interfaces or multiple default implementations
- **Performance warnings** - Potential performance issues in dependency graphs

## Automatic Validation

Validation happens automatically when you build the container:

```rust
use elif_core::container::IocContainerBuilder;

let mut builder = IocContainerBuilder::new();

builder
    .bind::<UserService, UserService>()
    .bind::<EmailService, SmtpEmailService>();

// Validation happens here - will fail if dependencies are missing
let container = builder.build()?; // Returns Result<IocContainer, CoreError>
```

## Validation Errors

### Missing Dependencies

When a service depends on an unregistered service:

```rust
use elif_core_derive::injectable;

#[injectable]
pub struct UserService {
    user_repo: Arc<UserRepository>,    // UserRepository not registered!
    email_service: Arc<EmailService>,  // EmailService is registered
}

let mut builder = IocContainerBuilder::new();
builder
    .bind_injectable::<UserService>()
    .bind::<EmailService, SmtpEmailService>();

// This will fail with a detailed error
match builder.build() {
    Err(CoreError::MissingRegistration { service_type, required_by }) => {
        println!("Service '{}' required by '{}' is not registered", service_type, required_by);
        // Output: Service 'UserRepository' required by 'UserService' is not registered
    }
    _ => unreachable!(),
}
```

### Circular Dependencies

When services depend on each other in a cycle:

```rust
#[injectable]
pub struct ServiceA {
    service_b: Arc<ServiceB>,
}

#[injectable] 
pub struct ServiceB {
    service_a: Arc<ServiceA>, // Creates a cycle!
}

let mut builder = IocContainerBuilder::new();
builder
    .bind_injectable::<ServiceA>()
    .bind_injectable::<ServiceB>();

match builder.build() {
    Err(CoreError::CircularDependency { cycle }) => {
        println!("Circular dependency detected: {:?}", cycle);
        // Output: Circular dependency detected: ["ServiceA", "ServiceB", "ServiceA"]
    }
    _ => unreachable!(),
}
```

### Lifetime Mismatches

When service lifetimes are incompatible:

```rust
use elif_core::container::ServiceScope;

let mut builder = IocContainerBuilder::new();

builder
    // Singleton service depending on transient service - potential issue
    .bind_with::<UserService, UserService>()
    .with_lifetime(ServiceScope::Singleton)
    .bind_with::<DatabaseConnection, DatabaseConnection>()  
    .with_lifetime(ServiceScope::Transient); // New connection every time

// This might generate a warning or error depending on configuration
match builder.build() {
    Err(CoreError::LifetimeIncompatibility { parent, child, parent_lifetime, child_lifetime }) => {
        println!("Lifetime mismatch: {} ({:?}) depends on {} ({:?})", 
                parent, parent_lifetime, child, child_lifetime);
    }
    _ => {},
}
```

## Manual Validation

You can also validate the container manually without building:

```rust
use elif_core::container::{DependencyValidator, ValidationReport};

let mut builder = IocContainerBuilder::new();
builder
    .bind::<UserService, UserService>()
    .bind::<EmailService, SmtpEmailService>();

// Get service descriptors without building
let descriptors = builder.get_descriptors();

// Validate manually
let validator = DependencyValidator::new(&descriptors);
let report = validator.validate();

match report.status {
    ValidationStatus::Valid => {
        println!("Container is valid!");
        let container = builder.build()?; // Safe to build
    }
    ValidationStatus::HasWarnings => {
        println!("Container has warnings:");
        for warning in report.warnings {
            println!("  - {}", warning.message);
        }
        let container = builder.build()?; // Still buildable
    }
    ValidationStatus::Invalid => {
        println!("Container is invalid:");
        for error in report.errors {
            println!("  - {}", error.message);
        }
        // Don't build - will fail
    }
}
```

## Validation Configuration

Customize validation behavior:

```rust
use elif_core::container::{ValidationConfig, ValidationLevel};

let config = ValidationConfig {
    // Treat lifetime mismatches as errors instead of warnings
    lifetime_validation: ValidationLevel::Error,
    
    // Allow circular dependencies (not recommended)
    circular_dependency_validation: ValidationLevel::Disabled,
    
    // Warn about performance issues
    performance_validation: ValidationLevel::Warning,
    
    // Maximum allowed dependency chain depth
    max_dependency_depth: 10,
    
    // Warn if a service has too many dependencies
    max_dependencies_per_service: 8,
};

let mut builder = IocContainerBuilder::new();
builder.set_validation_config(config);

// Validation will use the custom configuration
let container = builder.build()?;
```

## Validation Report Details

The validation report provides comprehensive information:

```rust
let report = validator.validate();

println!("Validation Report for Container");
println!("Status: {:?}", report.status);
println!("Services validated: {}", report.services_count);
println!("Errors: {}", report.errors.len());
println!("Warnings: {}", report.warnings.len());

// Print all errors
for error in &report.errors {
    println!("ERROR: {} ({})", error.message, error.error_code);
    if let Some(suggestion) = &error.suggestion {
        println!("  Suggestion: {}", suggestion);
    }
}

// Print all warnings  
for warning in &report.warnings {
    println!("WARNING: {} ({})", warning.message, warning.warning_code);
    if let Some(suggestion) = &warning.suggestion {
        println!("  Suggestion: {}", suggestion);
    }
}

// Print dependency graph statistics
if let Some(stats) = &report.graph_stats {
    println!("Graph Statistics:");
    println!("  - Max depth: {}", stats.max_depth);
    println!("  - Total edges: {}", stats.total_edges);
    println!("  - Strongly connected components: {}", stats.scc_count);
}
```

## Common Validation Issues

### 1. Forgotten Interface Registration

```rust
pub trait UserRepository: Send + Sync {
    fn find_by_id(&self, id: u32) -> Result<User, DbError>;
}

pub struct PostgresUserRepository;
impl UserRepository for PostgresUserRepository { /* ... */ }

#[injectable]
pub struct UserService {
    repo: Arc<dyn UserRepository>, // Interface dependency
}

let mut builder = IocContainerBuilder::new();
builder
    .bind_injectable::<UserService>()
    .bind::<PostgresUserRepository, PostgresUserRepository>(); // Wrong! Binds concrete type

// Error: dyn UserRepository not registered
// Fix: bind the interface
builder.bind::<dyn UserRepository, PostgresUserRepository>();
```

### 2. Transitive Dependency Issues

```rust
#[injectable]
pub struct ControllerA {
    service_a: Arc<ServiceA>,
}

#[injectable]
pub struct ServiceA {
    service_b: Arc<ServiceB>, // ServiceB not registered
}

let mut builder = IocContainerBuilder::new();
builder
    .bind_injectable::<ControllerA>()
    .bind_injectable::<ServiceA>(); 
    // Missing: ServiceB registration

// Error: ServiceB required by ServiceA is not registered
// Fix: Register all transitive dependencies
builder.bind_injectable::<ServiceB>();
```

### 3. Complex Circular Dependencies

```rust
// A → B → C → A (indirect circular dependency)
#[injectable] pub struct ServiceA { b: Arc<ServiceB> }
#[injectable] pub struct ServiceB { c: Arc<ServiceC> }  
#[injectable] pub struct ServiceC { a: Arc<ServiceA> } // Circular!

// Solution: Break the cycle with an interface
pub trait ServiceAInterface: Send + Sync {
    fn do_something(&self);
}

impl ServiceAInterface for ServiceA { /* ... */ }

#[injectable] 
pub struct ServiceC {
    a: Arc<dyn ServiceAInterface>, // Use interface to break cycle
}

let mut builder = IocContainerBuilder::new();
builder
    .bind_injectable::<ServiceA>()
    .bind_injectable::<ServiceB>()
    .bind_injectable::<ServiceC>()
    .bind::<dyn ServiceAInterface, ServiceA>(); // Bind interface separately
```

## Integration with CI/CD

Use validation in your build pipeline:

```rust
// In your build script or test
#[test]
fn validate_container_configuration() {
    let mut builder = IocContainerBuilder::new();
    
    // Register all your production services
    configure_production_services(&mut builder);
    
    // Validate without building
    let descriptors = builder.get_descriptors();
    let validator = DependencyValidator::new(&descriptors);
    let report = validator.validate();
    
    // Fail CI if there are errors
    assert_eq!(report.status, ValidationStatus::Valid, 
               "Container validation failed: {:#?}", report.errors);
    
    // Optional: Fail on too many warnings
    assert!(report.warnings.len() <= 5, 
            "Too many validation warnings: {}", report.warnings.len());
}

fn configure_production_services(builder: &mut IocContainerBuilder) {
    // Your actual service registration
    builder
        .bind_injectable::<UserController>()
        .bind_injectable::<UserService>()
        .bind::<dyn UserRepository, PostgresUserRepository>()
        // ... all other services
        ;
}
```

## Performance Validation

The validator can identify potential performance issues:

```rust
let report = validator.validate();

for warning in &report.warnings {
    match warning.warning_code.as_str() {
        "HIGH_TRANSIENT_RATIO" => {
            println!("Warning: High ratio of transient services detected");
            println!("Consider using singleton or scoped lifetimes for expensive objects");
        }
        "DEEP_DEPENDENCY_CHAIN" => {
            println!("Warning: Deep dependency chain detected (depth > 8)");
            println!("Consider refactoring to reduce coupling");
        }
        "TOO_MANY_DEPENDENCIES" => {
            println!("Warning: Service has too many dependencies (>8)");  
            println!("Consider splitting into smaller services");
        }
        "EXPENSIVE_FACTORY" => {
            println!("Warning: Factory function appears computationally expensive");
            println!("Consider caching or optimizing factory logic");
        }
        _ => {}
    }
}
```

## Best Practices

### 1. Validate Early and Often
Run validation in development and CI/CD:

```rust
// During development
if cfg!(debug_assertions) {
    let report = validator.validate();
    if report.status != ValidationStatus::Valid {
        panic!("Container validation failed in development: {:#?}", report);
    }
}
```

### 2. Use Descriptive Service Names
This helps with error messages:

```rust
// Good - clear service names
pub struct PostgresUserRepository;
pub struct SmtpEmailService;

// Avoid - generic names that don't help with debugging
pub struct Repository;
pub struct Service;
```

### 3. Validate Module Boundaries
When using service modules, validate each module separately:

```rust
#[test]
fn validate_database_module() {
    let mut builder = IocContainerBuilder::new();
    let database_module = DatabaseModule;
    database_module.configure_services(&mut builder)?;
    
    let report = validator.validate();
    assert_eq!(report.status, ValidationStatus::Valid);
}
```

### 4. Document Validation Exceptions
When you need to suppress certain validations:

```rust
let config = ValidationConfig {
    // We allow this circular dependency because ServiceA and ServiceB
    // use lazy initialization to break the actual runtime cycle
    circular_dependency_validation: ValidationLevel::Warning,
    ..Default::default()
};

builder.set_validation_config(config);
```

Container validation is essential for maintaining reliable dependency injection in complex applications. It catches issues early in the development process and provides clear guidance for fixing problems.