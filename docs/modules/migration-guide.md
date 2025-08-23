# Migration Guide - From Manual IoC to Module System

This guide helps you migrate from manual IoC container registration to the elif.rs module system, providing a clear path to leverage the new declarative module features.

## Overview

The migration involves three main steps:

1. **Identify current IoC patterns** in your codebase
2. **Convert manual registrations** to module definitions
3. **Compose modules** into your application

## Before: Manual IoC Registration

### Typical Manual Registration Pattern

```rust
use elif_core::container::IocContainer;

// Manual service registration (old way)
fn configure_container() -> IocContainer {
    let mut container = IocContainer::new();
    
    // Register concrete services
    container.bind::<DatabaseService, DatabaseService>();
    container.bind::<CacheService, CacheService>();
    
    // Register trait implementations
    container.bind::<dyn UserRepository, SqlUserRepository>();
    container.bind::<dyn EmailService, SmtpEmailService>();
    
    // Named registrations
    container.bind_named::<dyn EmailService, MockEmailService>("test");
    
    // Controllers
    container.bind::<UserController, UserController>();
    container.bind::<PostController, PostController>();
    
    container.build().expect("Failed to build container")
}
```

### Complex Initialization Logic

```rust
fn setup_application() -> Result<Application, Error> {
    let mut container = IocContainer::new();
    
    // Environment-specific configurations
    if cfg!(test) {
        container.bind::<dyn EmailService, MockEmailService>();
        container.bind::<dyn PaymentService, MockPaymentService>();
    } else {
        container.bind::<dyn EmailService, SmtpEmailService>();
        container.bind::<dyn PaymentService, StripePaymentService>();
    }
    
    // Conditional registrations
    if env::var("FEATURE_ANALYTICS").is_ok() {
        container.bind::<dyn AnalyticsService, GoogleAnalyticsService>();
    }
    
    let built_container = container.build()?;
    Ok(Application::new(built_container))
}
```

## After: Module System

### Step 1: Define Domain Modules

Break your services into logical modules by domain:

```rust
use elif_http_derive::module;

// User domain module
#[module(
    providers: [
        UserService,
        dyn UserRepository => SqlUserRepository
    ],
    controllers: [UserController],
    exports: [UserService, dyn UserRepository]
)]
pub struct UserModule;

// Email module  
#[module(
    providers: [
        dyn EmailService => SmtpEmailService,
        dyn EmailService => MockEmailService @ "test"
    ],
    exports: [dyn EmailService]
)]
pub struct EmailModule;

// Infrastructure module
#[module(
    providers: [
        DatabaseService,
        CacheService
    ],
    exports: [DatabaseService, CacheService]
)]
pub struct InfrastructureModule;
```

### Step 2: Create Application Composition

Use `module_composition!` to build your application:

```rust
use elif_http_derive::module_composition;

// Production configuration
fn create_production_app() -> ModuleDescriptor {
    module_composition! {
        modules: [
            InfrastructureModule,
            UserModule, 
            EmailModule,
            PostModule
        ]
    }
}

// Test configuration with overrides
fn create_test_app() -> ModuleDescriptor {
    module_composition! {
        modules: [
            InfrastructureModule,
            UserModule,
            EmailModule
        ],
        overrides: [
            dyn EmailService => MockEmailService,
            dyn PaymentService => MockPaymentService @ "test"
        ]
    }
}
```

### Step 3: Integrate with Application

```rust
use elif_core::modules::ModuleLoader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the application module configuration
    let app_descriptor = if cfg!(test) {
        create_test_app()
    } else {
        create_production_app()  
    };
    
    // Create and configure the IoC container from modules
    let mut container = IocContainer::new();
    let loader = ModuleLoader::new();
    loader.load_module(&app_descriptor, &mut container)?;
    
    let built_container = container.build()?;
    
    // Start your application
    let app = Application::new(built_container);
    app.run().await
}
```

## Migration Patterns

### Pattern 1: Simple Service Registration

**Before:**
```rust
container.bind::<UserService, UserService>();
container.bind::<EmailService, EmailService>();
```

**After:**
```rust
#[module(
    providers: [UserService, EmailService]
)]
pub struct ServicesModule;
```

### Pattern 2: Trait Implementations

**Before:**
```rust
container.bind::<dyn Repository, SqlRepository>();
container.bind::<dyn EmailService, SmtpEmailService>();
```

**After:**
```rust
#[module(
    providers: [
        dyn Repository => SqlRepository,
        dyn EmailService => SmtpEmailService
    ]
)]
pub struct ImplementationModule;
```

### Pattern 3: Named Registrations

**Before:**
```rust
container.bind_named::<dyn EmailService, SmtpEmailService>("smtp");
container.bind_named::<dyn EmailService, MockEmailService>("mock");
```

**After:**
```rust
#[module(
    providers: [
        dyn EmailService => SmtpEmailService @ "smtp",
        dyn EmailService => MockEmailService @ "mock"
    ]
)]
pub struct EmailModule;
```

### Pattern 4: Environment-Specific Configuration

**Before:**
```rust
if cfg!(test) {
    container.bind::<dyn EmailService, MockEmailService>();
} else {
    container.bind::<dyn EmailService, SmtpEmailService>();
}
```

**After:**
```rust
// Define both modules
#[module(providers: [dyn EmailService => SmtpEmailService])]
pub struct ProductionEmailModule;

#[module(providers: [dyn EmailService => MockEmailService])]
pub struct TestEmailModule;

// Compose conditionally
fn email_module() -> ModuleDescriptor {
    if cfg!(test) {
        TestEmailModule::module_descriptor()
    } else {
        ProductionEmailModule::module_descriptor()
    }
}
```

## Demo DSL for Quick Migration

For simple cases, use the demo DSL for rapid migration:

**Before:**
```rust
container.bind::<UserService, UserService>();
container.bind::<PostService, PostService>();
container.bind::<UserController, UserController>();
```

**After (Demo DSL):**
```rust
let module = demo_module! {
    services: [UserService, PostService],
    controllers: [UserController]
};
```

## Benefits of Migration

### Before (Manual IoC)
- ❌ Verbose boilerplate code
- ❌ Runtime-only validation
- ❌ No dependency visualization
- ❌ Difficult to test configurations
- ❌ No module boundaries

### After (Module System)  
- ✅ Declarative, concise definitions
- ✅ Compile-time validation
- ✅ Clear module dependencies
- ✅ Easy configuration overrides
- ✅ Testable module compositions

## Migration Checklist

- [ ] **Inventory current IoC registrations** - List all services and their bindings
- [ ] **Group by domain** - Organize services into logical modules
- [ ] **Define module boundaries** - Decide what each module imports/exports
- [ ] **Convert registrations** - Use `#[module(...)]` or `demo_module!` syntax
- [ ] **Test module composition** - Ensure all dependencies resolve correctly
- [ ] **Update application startup** - Replace manual container setup with module loader
- [ ] **Add environment-specific modules** - Handle test/production differences
- [ ] **Cleanup old code** - Remove manual IoC registration functions

## Common Pitfalls

### Circular Dependencies
```rust
// ❌ Avoid circular imports
#[module(imports: [ModuleB], exports: [ServiceA])]
pub struct ModuleA;

#[module(imports: [ModuleA], exports: [ServiceB])] 
pub struct ModuleB;
```

**Solution:** Create a shared module or restructure dependencies.

### Missing Exports
```rust
// ❌ Service not exported from module
#[module(providers: [UserService])] // Missing exports
pub struct UserModule;

#[module(
    imports: [UserModule],
    providers: [PostService] // Can't access UserService
)]
pub struct PostModule; 
```

**Solution:** Export services that other modules need to import.

## Advanced Migration Scenarios

### Migrating Factory Patterns

**Before:**
```rust
container.bind_factory::<dyn DatabaseConnection, _>(|| {
    create_database_connection(&config)
});
```

**After:**
```rust
// Use provider with custom initialization
#[module(providers: [DatabaseConnectionProvider])]
pub struct DatabaseModule;

impl DatabaseConnectionProvider {
    pub fn provide(&self) -> Box<dyn DatabaseConnection> {
        create_database_connection(&self.config)
    }
}
```

### Migrating Scoped Services

**Before:**
```rust
container.bind_scoped::<RequestContext, RequestContext>();
```

**After:**
```rust
// Module system handles scoping automatically
#[module(providers: [RequestContext])]
pub struct RequestModule;
```

## Next Steps

- [Module Definition Guide](module-definition.md) - Learn the full module syntax
- [Dependency Injection Patterns](dependency-injection.md) - Advanced provider patterns  
- [Testing Modules](testing-guide.md) - Test your module configurations
- [Best Practices](best-practices.md) - Patterns and conventions for module design