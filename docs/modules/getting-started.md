# Getting Started with elif.rs Module System

The elif.rs module system provides a Laravel-inspired dependency injection system with compile-time safety and runtime efficiency. This guide will help you get started with both the full `#[module(...)]` syntax and the simplified demo DSL.

## Overview

The module system consists of three main components:

1. **`#[module(...)]`** - Define modules with providers, controllers, imports, and exports
2. **`module_composition!`** - Compose multiple modules into an application
3. **`demo_module!`** - Simplified Laravel-style sugar syntax for common cases

## Quick Start

### Basic Module Definition

```rust
use elif_http_derive::module;

// Define your services
pub struct UserService {
    // Service implementation
}

pub struct UserController {
    // Controller implementation
}

// Define a module with the #[module(...)] attribute
#[module(
    providers: [UserService],
    controllers: [UserController]
)]
pub struct UserModule;
```

### Demo DSL Sugar Syntax

For simple cases, use the `demo_module!` macro with Laravel-style simplicity:

```rust
use elif_http_derive::demo_module;

// Simplified syntax for quick prototyping
let user_module = demo_module! {
    services: [UserService, EmailService],
    controllers: [UserController, PostController],
    middleware: ["cors", "logging", "auth"]
};
```

### Module Composition

Combine multiple modules into your application:

```rust
use elif_http_derive::module_composition;

// Compose your application from modules
let app = module_composition! {
    modules: [UserModule, AuthModule, PostModule],
    overrides: [
        // Override specific services for testing or configuration
        EmailService => MockEmailService @ "test"
    ]
};
```

## Key Features

### Laravel-Style Simplicity

- **Convention over configuration** - Sensible defaults require minimal setup
- **Zero boilerplate** - Define what you need, framework handles the rest
- **Intuitive APIs** - If you know Laravel, you'll feel at home

### Compile-Time Safety

- **Type-safe dependency injection** - Catch errors at compile time
- **Circular dependency detection** - Prevent runtime issues
- **Provider validation** - Ensure all dependencies are satisfied

### Runtime Efficiency

- **Token-based trait injection** - Fast resolution using compile-time tokens
- **Singleton lifecycle management** - Optimal memory usage
- **Lazy initialization** - Services created only when needed

## Next Steps

- [Module Definition Guide](module-definition.md) - Learn the full `#[module(...)]` syntax
- [Dependency Injection](dependency-injection.md) - Understand provider patterns
- [Application Composition](application-composition.md) - Build complex applications
- [Migration Guide](migration-guide.md) - Migrate from manual IoC registration
- [Best Practices](best-practices.md) - Patterns and conventions

## Examples Repository

Check out the [examples directory](../../examples/) for complete working examples:

- [Basic Module Usage](../../examples/basic-module/)
- [Multi-Module Application](../../examples/multi-module-app/)
- [Testing with Module Overrides](../../examples/testing-modules/)