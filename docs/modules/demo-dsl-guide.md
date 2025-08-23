# Demo DSL Guide - Laravel-Style Module Syntax

The `demo_module!` macro provides Laravel-inspired sugar syntax for quickly defining modules without the ceremony of the full `#[module(...)]` attribute syntax.

## Syntax Overview

```rust
use elif_http_derive::demo_module;

let module_descriptor = demo_module! {
    services: [ServiceType1, ServiceType2, ...],
    controllers: [ControllerType1, ControllerType2, ...],
    middleware: ["middleware_name1", "middleware_name2", ...]
};
```

## Basic Usage

### Simple Service Registration

Register concrete services that will be automatically bound in the IoC container:

```rust
pub struct UserService;
pub struct EmailService;

let services_module = demo_module! {
    services: [UserService, EmailService]
};
```

This expands to the equivalent of:

```rust
#[module(
    providers: [UserService, EmailService],
    controllers: [],
    imports: [],
    exports: []
)]
pub struct ServicesModule;
```

### Adding Controllers

Include controllers that will be registered for dependency injection:

```rust
pub struct UserController;
pub struct PostController;

let web_module = demo_module! {
    services: [UserService],
    controllers: [UserController, PostController]
};
```

### Middleware Stack

Apply middleware to the module (simplified demo implementation):

```rust
let auth_module = demo_module! {
    services: [AuthService],
    controllers: [AuthController],
    middleware: ["cors", "logging", "auth", "rate_limiting"]
};
```

## Complete Example

```rust
use elif_http_derive::demo_module;

// Define your types
pub struct DatabaseService;
pub struct CacheService;
pub struct UserService;
pub struct PostService;

pub struct UserController;
pub struct PostController;
pub struct AdminController;

// Create a complete module with all features
let blog_module = demo_module! {
    services: [
        DatabaseService,
        CacheService, 
        UserService,
        PostService
    ],
    controllers: [
        UserController,
        PostController,
        AdminController
    ],
    middleware: [
        "cors",           // Cross-origin resource sharing
        "logging",        // Request/response logging
        "auth",           // Authentication
        "rate_limiting",  // API rate limiting
        "compression"     // Response compression
    ]
};

// The module_descriptor can now be used in your application
println!("Created module with {} middleware layers", 
    blog_module.middleware_stack.len());
```

## When to Use Demo DSL vs Full Syntax

### Use `demo_module!` When:

- **Rapid prototyping** - Quick setup for testing ideas
- **Simple modules** - Only concrete services, no trait mappings
- **Learning** - Getting familiar with the module system
- **Examples/demos** - Clear, concise examples

### Use `#[module(...)]` When:

- **Production code** - Full control and explicit configuration
- **Complex dependencies** - Trait mappings, named services, imports/exports
- **Library development** - Proper module boundaries and interfaces
- **Large applications** - Multiple interconnected modules

## Laravel Inspiration

The demo DSL syntax draws inspiration from Laravel's service container and provider system:

```php
// Laravel-style (PHP)
class BlogServiceProvider extends ServiceProvider {
    public function register() {
        $this->app->bind(UserService::class);
        $this->app->bind(PostService::class);
    }
}
```

```rust
// elif.rs equivalent (Rust)
let blog_module = demo_module! {
    services: [UserService, PostService]
};
```

## Limitations

The demo DSL is intentionally simplified and has some limitations:

- **No trait mappings** - Only concrete service types supported
- **No imports/exports** - Cannot depend on or expose services from other modules  
- **Simple middleware** - Basic string-based middleware names only
- **Demo implementation** - Middleware integration is simplified for demonstration

For production use cases requiring these features, use the full `#[module(...)]` syntax.

## Migration to Full Syntax

When you outgrow the demo DSL, migrating to the full syntax is straightforward:

```rust
// Demo DSL
let module = demo_module! {
    services: [UserService, EmailService],
    controllers: [UserController]
};

// Equivalent full syntax
#[module(
    providers: [UserService, EmailService],
    controllers: [UserController],
    imports: [],
    exports: []
)]
pub struct UserModule;
```

## Next Steps

- [Full Module Syntax Guide](module-definition.md) - Learn the complete `#[module(...)]` system
- [Dependency Injection Patterns](dependency-injection.md) - Advanced provider configurations
- [Migration Guide](migration-guide.md) - Move from demo DSL to production-ready modules