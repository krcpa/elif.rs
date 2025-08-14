# Phase 1: Architecture Foundation

**Duration**: Months 1-3 (12 weeks)  
**Team**: 2-3 developers  
**Goal**: Build solid architectural foundation with dependency injection and modular design

## Overview

Phase 1 establishes the core architectural patterns that all other phases will build upon. This includes the dependency injection container, module system, configuration management, and basic HTTP handling.

## Key Components

### 1. Service Container & Dependency Injection
**File**: `crates/elif-core/src/container.rs`

The heart of the framework - enables testable, loosely-coupled components.

**Requirements**:
- Service registration (bind, singleton, factory)
- Service resolution with automatic dependency injection
- Circular dependency detection
- Scoped services (per-request lifecycle)
- Thread-safe operation

**API Design**:
```rust
pub struct Container {
    services: HashMap<TypeId, ServiceDefinition>,
    singletons: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    scoped: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Container {
    pub fn bind<Interface, Implementation>(&mut self) 
    where Implementation: Interface + 'static;
    
    pub fn singleton<T>(&mut self, instance: T) 
    where T: 'static + Send + Sync;
    
    pub fn resolve<T>(&self) -> Result<T, ContainerError>;
}
```

### 2. Module System
**File**: `crates/elif-core/src/module.rs`

Organizes application into feature-based modules (similar to NestJS).

**Requirements**:
- Module trait for registration and bootstrapping
- Service provider pattern
- Route registration per module
- Middleware registration per module
- Module dependency ordering

**API Design**:
```rust
pub trait Module: Send + Sync {
    fn name(&self) -> &'static str;
    fn register(&self, container: &mut Container);
    fn boot(&self, app: &Application) -> Result<(), ModuleError>;
    fn routes(&self) -> Vec<RouteDefinition>;
    fn middleware(&self) -> Vec<MiddlewareDefinition>;
}
```

### 3. Configuration Management
**File**: `crates/elif-core/src/config.rs`

Environment-based configuration with validation.

**Requirements**:
- Environment variable binding with defaults
- Configuration validation at startup
- Nested configuration structures
- Type-safe configuration access
- Configuration reloading (for development)

**API Design**:
```rust
#[derive(Config, Debug)]
pub struct AppConfig {
    #[config(env = "APP_NAME", default = "elif-app")]
    pub name: String,
    
    #[config(env = "DATABASE_URL")]
    pub database_url: String,
    
    #[config(nested)]
    pub auth: AuthConfig,
}
```

### 4. Application Lifecycle
**File**: `crates/elif-core/src/application.rs`

Application bootstrapping, startup, and shutdown.

**Requirements**:
- Builder pattern for application construction
- Service provider registration and booting
- Graceful shutdown handling
- Lifecycle hooks (starting, started, stopping, stopped)
- Error handling during startup

**API Design**:
```rust
pub struct Application {
    container: Container,
    modules: Vec<Box<dyn Module>>,
    config: AppConfig,
}

impl Application {
    pub fn builder() -> ApplicationBuilder;
    pub async fn start(&mut self) -> Result<(), ApplicationError>;
    pub async fn shutdown(&mut self) -> Result<(), ApplicationError>;
}
```

### 5. Basic HTTP Layer
**File**: `crates/elif-http/src/server.rs`

Basic HTTP server with routing (detailed HTTP features in Phase 4).

**Requirements**:
- Route registration and matching
- Request/Response abstractions
- Basic middleware support
- Async request handling
- Error handling and conversion

## Implementation Plan

### Week 1-2: Service Container
- [ ] Design container architecture
- [ ] Implement service registration methods
- [ ] Add service resolution with dependency injection
- [ ] Build circular dependency detection
- [ ] Add comprehensive tests

### Week 3-4: Module System  
- [ ] Define Module trait
- [ ] Implement module registration and booting
- [ ] Add service provider pattern
- [ ] Build module dependency resolution
- [ ] Create example modules for testing

### Week 5-6: Configuration Management
- [ ] Create Config derive macro
- [ ] Implement environment variable binding
- [ ] Add configuration validation
- [ ] Support nested configurations
- [ ] Add hot reloading for development

### Week 7-8: Application Lifecycle
- [ ] Implement Application builder
- [ ] Add startup/shutdown lifecycle
- [ ] Integrate container and modules
- [ ] Add error handling and recovery
- [ ] Build graceful shutdown

### Week 9-10: Basic HTTP Integration
- [ ] Create basic HTTP server
- [ ] Implement route registration
- [ ] Add request/response handling
- [ ] Integrate with DI container
- [ ] Basic middleware support

### Week 11-12: Testing & Documentation
- [ ] Comprehensive unit tests (>90% coverage)
- [ ] Integration tests with example app
- [ ] Performance benchmarks
- [ ] API documentation
- [ ] Usage examples and tutorials

## Testing Strategy

### Unit Tests
- Container service resolution under various scenarios
- Module registration and booting
- Configuration parsing and validation
- Application lifecycle management

### Integration Tests
- End-to-end application startup and shutdown
- Module interaction and dependency injection
- Configuration loading from various sources
- HTTP request handling through full stack

### Performance Tests
- Service resolution performance (target: <1μs per resolution)
- Module loading time (target: <100ms for 100 modules)
- Memory usage (target: <10MB base overhead)

## Success Criteria

### Functional Requirements
- [ ] Can create services and resolve them through DI container
- [ ] Modules can register services, routes, and middleware
- [ ] Configuration loads from environment with proper validation
- [ ] Application starts up cleanly and handles basic HTTP requests
- [ ] Graceful shutdown works correctly

### Performance Requirements
- [ ] Service resolution: <1 microsecond average
- [ ] Application startup: <500ms for basic app
- [ ] Memory usage: <10MB base overhead
- [ ] Can handle 1000+ concurrent connections

### Quality Requirements
- [ ] >90% test coverage
- [ ] Zero memory leaks in long-running processes
- [ ] All public APIs documented
- [ ] Comprehensive error messages

## Deliverables

1. **Core Crates**:
   - `elif-core` - Container, modules, config, application
   - `elif-http` - Basic HTTP server and routing

2. **Documentation**:
   - API documentation for all public interfaces
   - Tutorial for creating basic applications
   - Architecture documentation

3. **Examples**:
   - Basic "Hello World" application
   - Multi-module application example
   - Configuration examples

4. **Tests**:
   - Unit test suite with >90% coverage
   - Integration test suite
   - Performance benchmarks

## Files Structure
```
crates/elif-core/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── container.rs        # DI container implementation
│   ├── module.rs          # Module system
│   ├── config.rs          # Configuration management
│   ├── application.rs     # Application lifecycle
│   └── error.rs          # Error types
├── tests/
│   ├── container_tests.rs
│   ├── module_tests.rs
│   ├── config_tests.rs
│   └── integration_tests.rs
└── Cargo.toml

crates/elif-http/
├── src/
│   ├── lib.rs
│   ├── server.rs          # HTTP server
│   ├── router.rs          # Route matching
│   ├── request.rs         # Request abstraction
│   └── response.rs        # Response abstraction
└── Cargo.toml
```

This phase provides the foundation that all subsequent phases will build upon. The architecture must be solid, performant, and extensible to support the complex features planned for later phases.