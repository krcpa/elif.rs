# Phase 1: Technical Specifications

## Service Container Specification

### Container Core Implementation

```rust
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct Container {
    // Service definitions
    bindings: HashMap<TypeId, ServiceBinding>,
    // Singleton instances
    singletons: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
    // Scoped instances (per HTTP request)
    scoped: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    // Service metadata for introspection
    metadata: HashMap<TypeId, ServiceMetadata>,
}

pub enum ServiceBinding {
    Transient(Box<dyn Fn(&Container) -> Box<dyn Any + Send + Sync> + Send + Sync>),
    Singleton(Box<dyn Fn(&Container) -> Arc<dyn Any + Send + Sync> + Send + Sync>),
    Scoped(Box<dyn Fn(&Container) -> Box<dyn Any + Send + Sync> + Send + Sync>),
}

pub struct ServiceMetadata {
    pub name: String,
    pub dependencies: Vec<TypeId>,
    pub lifecycle: ServiceLifecycle,
}

pub enum ServiceLifecycle {
    Transient,
    Singleton, 
    Scoped,
}
```

### Container Methods

```rust
impl Container {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            singletons: Arc::new(RwLock::new(HashMap::new())),
            scoped: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    // Bind interface to implementation (transient)
    pub fn bind<I, T>(&mut self) -> &mut Self
    where
        I: 'static,
        T: 'static + Send + Sync + From<I>,
    {
        let type_id = TypeId::of::<I>();
        let factory = Box::new(|container: &Container| {
            Box::new(T::from(container.resolve::<I>().unwrap())) as Box<dyn Any + Send + Sync>
        });
        
        self.bindings.insert(type_id, ServiceBinding::Transient(factory));
        self.metadata.insert(type_id, ServiceMetadata {
            name: std::any::type_name::<I>().to_string(),
            dependencies: vec![], // TODO: Extract from constructor
            lifecycle: ServiceLifecycle::Transient,
        });
        
        self
    }

    // Register singleton
    pub fn singleton<T>(&mut self, instance: T) -> &mut Self
    where
        T: 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let instance = Arc::new(instance);
        
        self.singletons.write().unwrap().insert(type_id, instance);
        self.metadata.insert(type_id, ServiceMetadata {
            name: std::any::type_name::<T>().to_string(),
            dependencies: vec![],
            lifecycle: ServiceLifecycle::Singleton,
        });
        
        self
    }

    // Resolve service
    pub fn resolve<T>(&self) -> Result<T, ContainerError>
    where
        T: 'static + Clone,
    {
        let type_id = TypeId::of::<T>();
        
        // Check singletons first
        if let Some(instance) = self.singletons.read().unwrap().get(&type_id) {
            if let Some(typed_instance) = instance.downcast_ref::<T>() {
                return Ok(typed_instance.clone());
            }
        }
        
        // Check bindings
        if let Some(binding) = self.bindings.get(&type_id) {
            match binding {
                ServiceBinding::Transient(factory) => {
                    let instance = factory(self);
                    if let Some(typed_instance) = instance.downcast_ref::<T>() {
                        return Ok(typed_instance.clone());
                    }
                }
                ServiceBinding::Singleton(factory) => {
                    let instance = factory(self);
                    // Cache for future use
                    self.singletons.write().unwrap().insert(type_id, instance.clone());
                    if let Some(typed_instance) = instance.downcast_ref::<T>() {
                        return Ok(typed_instance.clone());
                    }
                }
                ServiceBinding::Scoped(_) => {
                    // TODO: Implement scoped resolution
                    return Err(ContainerError::ScopedNotImplemented);
                }
            }
        }
        
        Err(ContainerError::ServiceNotFound {
            service: std::any::type_name::<T>().to_string(),
        })
    }

    // Get all registered services (for introspection)
    pub fn services(&self) -> Vec<&ServiceMetadata> {
        self.metadata.values().collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    #[error("Service '{service}' not found in container")]
    ServiceNotFound { service: String },
    
    #[error("Circular dependency detected: {chain}")]
    CircularDependency { chain: String },
    
    #[error("Scoped services not yet implemented")]
    ScopedNotImplemented,
}
```

## Module System Specification

### Module Trait

```rust
pub trait Module: Send + Sync {
    /// Module name for identification
    fn name(&self) -> &'static str;
    
    /// Register services in the container
    fn register(&self, container: &mut Container);
    
    /// Boot the module (called after all modules are registered)
    fn boot(&self, app: &Application) -> Result<(), ModuleError> {
        Ok(())
    }
    
    /// Define routes for this module
    fn routes(&self) -> Vec<RouteDefinition> {
        vec![]
    }
    
    /// Define middleware for this module
    fn middleware(&self) -> Vec<MiddlewareDefinition> {
        vec![]
    }
    
    /// Module dependencies (other modules that must be loaded first)
    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }
}

pub struct RouteDefinition {
    pub method: HttpMethod,
    pub path: String,
    pub handler: String, // "ControllerName::method"
    pub middleware: Vec<String>,
}

pub struct MiddlewareDefinition {
    pub name: String,
    pub priority: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("Module '{name}' failed to boot: {reason}")]
    BootFailed { name: String, reason: String },
    
    #[error("Circular dependency in modules: {chain}")]
    CircularDependency { chain: String },
}
```

### Module Registry

```rust
pub struct ModuleRegistry {
    modules: Vec<Box<dyn Module>>,
    boot_order: Vec<usize>, // Indices in dependency order
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: vec![],
            boot_order: vec![],
        }
    }
    
    pub fn register<M: Module + 'static>(&mut self, module: M) {
        self.modules.push(Box::new(module));
    }
    
    pub fn resolve_dependencies(&mut self) -> Result<(), ModuleError> {
        // Topological sort of modules based on dependencies
        let mut visited = vec![false; self.modules.len()];
        let mut temp_mark = vec![false; self.modules.len()];
        let mut result = Vec::new();
        
        for i in 0..self.modules.len() {
            if !visited[i] {
                self.visit(i, &mut visited, &mut temp_mark, &mut result)?;
            }
        }
        
        self.boot_order = result;
        Ok(())
    }
    
    fn visit(
        &self,
        index: usize,
        visited: &mut Vec<bool>,
        temp_mark: &mut Vec<bool>,
        result: &mut Vec<usize>
    ) -> Result<(), ModuleError> {
        if temp_mark[index] {
            return Err(ModuleError::CircularDependency {
                chain: "TODO: Build dependency chain".to_string(),
            });
        }
        
        if visited[index] {
            return Ok(());
        }
        
        temp_mark[index] = true;
        
        // Visit dependencies first
        let deps = self.modules[index].dependencies();
        for dep_name in deps {
            if let Some(dep_index) = self.find_module(dep_name) {
                self.visit(dep_index, visited, temp_mark, result)?;
            }
        }
        
        temp_mark[index] = false;
        visited[index] = true;
        result.push(index);
        
        Ok(())
    }
    
    fn find_module(&self, name: &str) -> Option<usize> {
        self.modules.iter().position(|m| m.name() == name)
    }
    
    pub fn boot_all(&self, container: &mut Container, app: &Application) -> Result<(), ModuleError> {
        // Register all modules first
        for module in &self.modules {
            module.register(container);
        }
        
        // Boot in dependency order
        for &index in &self.boot_order {
            self.modules[index].boot(app).map_err(|_| ModuleError::BootFailed {
                name: self.modules[index].name().to_string(),
                reason: "Boot failed".to_string(),
            })?;
        }
        
        Ok(())
    }
}
```

## Configuration System Specification

### Configuration Trait and Derive

```rust
pub trait Config: Sized {
    fn from_env() -> Result<Self, ConfigError>;
    fn validate(&self) -> Result<(), ConfigError>;
}

// This would be a derive macro
#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    // Implementation would parse struct fields and generate:
    // 1. from_env() method that reads environment variables
    // 2. validate() method that checks constraints
    // 3. default values and type conversion
}

// Usage example:
#[derive(Config, Debug)]
pub struct AppConfig {
    #[config(env = "APP_NAME", default = "elif-app")]
    pub name: String,
    
    #[config(env = "APP_ENV", default = "development")]
    pub environment: Environment,
    
    #[config(env = "DATABASE_URL")]
    pub database_url: String,
    
    #[config(env = "JWT_SECRET")]
    pub jwt_secret: String,
    
    #[config(nested)]
    pub server: ServerConfig,
}

#[derive(Config, Debug)]
pub struct ServerConfig {
    #[config(env = "SERVER_HOST", default = "0.0.0.0")]
    pub host: String,
    
    #[config(env = "SERVER_PORT", default = 3000)]
    pub port: u16,
    
    #[config(env = "SERVER_WORKERS", default = 0)] // 0 = auto-detect
    pub workers: usize,
}

#[derive(Debug)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl FromStr for Environment {
    type Err = ConfigError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "testing" | "test" => Ok(Environment::Testing), 
            "production" | "prod" => Ok(Environment::Production),
            _ => Err(ConfigError::InvalidValue {
                field: "environment".to_string(),
                value: s.to_string(),
                expected: "development, testing, or production".to_string(),
            }),
        }
    }
}
```

### Configuration Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {var}")]
    MissingEnvVar { var: String },
    
    #[error("Invalid value for {field}: '{value}', expected {expected}")]
    InvalidValue { field: String, value: String, expected: String },
    
    #[error("Validation failed for {field}: {reason}")]
    ValidationFailed { field: String, reason: String },
    
    #[error("Configuration parsing error: {message}")]
    ParseError { message: String },
}
```

## Application Lifecycle Specification

### Application Structure

```rust
pub struct Application {
    container: Container,
    modules: ModuleRegistry,
    config: AppConfig,
    state: ApplicationState,
}

#[derive(Debug, Clone)]
pub enum ApplicationState {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String),
}

impl Application {
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder::new()
    }
    
    pub async fn start(&mut self) -> Result<(), ApplicationError> {
        self.state = ApplicationState::Starting;
        
        // Load configuration
        self.config = AppConfig::from_env()
            .map_err(|e| ApplicationError::ConfigError(e))?;
        
        // Validate configuration
        self.config.validate()
            .map_err(|e| ApplicationError::ConfigError(e))?;
        
        // Register configuration as singleton
        self.container.singleton(self.config.clone());
        
        // Resolve module dependencies
        self.modules.resolve_dependencies()
            .map_err(|e| ApplicationError::ModuleError(e))?;
        
        // Boot all modules
        self.modules.boot_all(&mut self.container, self)
            .map_err(|e| ApplicationError::ModuleError(e))?;
        
        self.state = ApplicationState::Running;
        Ok(())
    }
    
    pub async fn shutdown(&mut self) -> Result<(), ApplicationError> {
        self.state = ApplicationState::Stopping;
        
        // TODO: Shutdown modules in reverse boot order
        // TODO: Close database connections
        // TODO: Finish processing queued jobs
        
        self.state = ApplicationState::Stopped;
        Ok(())
    }
    
    pub fn state(&self) -> ApplicationState {
        self.state.clone()
    }
}

pub struct ApplicationBuilder {
    modules: Vec<Box<dyn Module>>,
}

impl ApplicationBuilder {
    pub fn new() -> Self {
        Self {
            modules: vec![],
        }
    }
    
    pub fn module<M: Module + 'static>(mut self, module: M) -> Self {
        self.modules.push(Box::new(module));
        self
    }
    
    pub fn build(self) -> Application {
        let mut registry = ModuleRegistry::new();
        for module in self.modules {
            registry.register_boxed(module);
        }
        
        Application {
            container: Container::new(),
            modules: registry,
            config: AppConfig::default(), // Will be loaded on start
            state: ApplicationState::Created,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
    
    #[error("Module error: {0}")]
    ModuleError(#[from] ModuleError),
    
    #[error("Container error: {0}")]
    ContainerError(#[from] ContainerError),
    
    #[error("Application failed to start: {reason}")]
    StartupFailed { reason: String },
}
```

## HTTP Layer Specification

### Basic HTTP Server

```rust
use axum::{Router, Server};
use std::net::SocketAddr;

pub struct HttpServer {
    app: Router,
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new(container: &Container, config: &ServerConfig) -> Self {
        let app = Router::new();
        let addr = format!("{}:{}", config.host, config.port)
            .parse()
            .expect("Invalid server address");
            
        Self { app, addr }
    }
    
    pub async fn serve(self) -> Result<(), std::io::Error> {
        println!("🚀 Server starting on {}", self.addr);
        
        Server::bind(&self.addr)
            .serve(self.app.into_make_service())
            .await
    }
}
```

### Route Registration

```rust
pub struct RouteRegistry {
    routes: Vec<RouteDefinition>,
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self { routes: vec![] }
    }
    
    pub fn add(&mut self, route: RouteDefinition) {
        self.routes.push(route);
    }
    
    pub fn build_router(&self, container: &Container) -> Router {
        let mut router = Router::new();
        
        for route in &self.routes {
            // TODO: Convert route definitions to Axum routes
            // This requires more complex handler resolution
        }
        
        router
    }
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    OPTIONS,
    HEAD,
}
```

## Testing Requirements

### Unit Tests Required

1. **Container Tests**:
   - Service registration and resolution
   - Singleton behavior
   - Circular dependency detection
   - Thread safety

2. **Module Tests**:
   - Module registration
   - Dependency resolution
   - Boot order verification

3. **Configuration Tests**:
   - Environment variable parsing
   - Default value handling
   - Validation logic
   - Nested configuration

4. **Application Tests**:
   - Startup/shutdown lifecycle
   - Error handling during startup
   - State transitions

### Integration Tests Required

1. **End-to-End Application**:
   - Full application startup with multiple modules
   - HTTP request handling
   - Configuration loading from environment

2. **Module Integration**:
   - Cross-module service resolution
   - Module boot order with dependencies

### Performance Benchmarks Required

1. **Service Resolution**: <1μs per resolution
2. **Application Startup**: <500ms for basic app  
3. **Memory Usage**: <10MB base overhead
4. **HTTP Throughput**: 1000+ requests/second

This specification provides the technical detail needed to implement Phase 1. Each component is designed to be independently testable while working together as a cohesive foundation.