# Service Modules

Service modules in elif.rs provide a way to organize related services, manage dependencies between modules, and handle complex initialization sequences. This is especially useful for large applications where services can be logically grouped.

## Overview

Service modules allow you to:
- **Group related services** - Organize services by domain, feature, or layer
- **Manage dependencies** - Define module load order and dependencies 
- **Lifecycle management** - Initialize and shutdown modules in correct order
- **Modular architecture** - Enable/disable entire feature sets

## Basic Module Definition

```rust
use elif_core::container::{ServiceModule, ModuleId, IocContainer, ServiceBinder};

pub struct DatabaseModule;

impl ServiceModule for DatabaseModule {
    fn module_id(&self) -> ModuleId {
        ModuleId::new("database")
    }
    
    fn configure_services(&self, container: &mut dyn ServiceBinder) -> Result<(), CoreError> {
        container
            .bind::<dyn UserRepository, PostgresUserRepository>()
            .bind::<dyn OrderRepository, PostgresOrderRepository>()
            .bind_singleton::<DatabaseConfig, DatabaseConfig>()
            .bind_factory::<PgPool, _, _>(|| {
                let config = DatabaseConfig::from_env()?;
                PgPool::connect(&config.url).await
            });
        
        Ok(())
    }
    
    fn depends_on(&self) -> Vec<ModuleId> {
        vec![ModuleId::new("config")] // Must load config module first
    }
}
```

## Module Registry

The `ModuleRegistry` manages module loading, dependency resolution, and initialization:

```rust
use elif_core::container::{ModuleRegistry, IocContainerBuilder};

let mut registry = ModuleRegistry::new();

// Register modules
registry.register_module(Box::new(ConfigModule))?;
registry.register_module(Box::new(DatabaseModule))?;
registry.register_module(Box::new(AuthModule))?;
registry.register_module(Box::new(EmailModule))?;

// Build container with modules
let mut container_builder = IocContainerBuilder::new();
registry.configure_container(&mut container_builder)?;

let container = container_builder.build()?;
```

## Module Dependencies

Modules can declare dependencies on other modules:

```rust
pub struct AuthModule;

impl ServiceModule for AuthModule {
    fn module_id(&self) -> ModuleId {
        ModuleId::new("auth")
    }
    
    fn depends_on(&self) -> Vec<ModuleId> {
        vec![
            ModuleId::new("database"), // Need repositories
            ModuleId::new("config"),   // Need JWT config
            ModuleId::new("cache"),    // Need session cache
        ]
    }
    
    fn configure_services(&self, container: &mut dyn ServiceBinder) -> Result<(), CoreError> {
        container
            .bind::<dyn AuthService, JwtAuthService>()
            .bind::<dyn SessionManager, RedisSessionManager>()
            .bind_factory::<JwtConfig, _, _>(|| {
                JwtConfig::from_env()
            });
        
        Ok(())
    }
}
```

### Dependency Resolution

The registry automatically resolves module dependencies:

```rust
// Modules will be loaded in correct order:
// 1. ConfigModule (no dependencies)
// 2. CacheModule (depends on config)
// 3. DatabaseModule (depends on config)
// 4. AuthModule (depends on database, config, cache)
// 5. EmailModule (depends on config)

registry.configure_container(&mut container_builder)?;
```

### Circular Dependency Detection

The registry detects circular dependencies at registration time:

```rust
// This would cause an error
pub struct ModuleA;
impl ServiceModule for ModuleA {
    fn depends_on(&self) -> Vec<ModuleId> {
        vec![ModuleId::new("module_b")]
    }
}

pub struct ModuleB; 
impl ServiceModule for ModuleB {
    fn depends_on(&self) -> Vec<ModuleId> {
        vec![ModuleId::new("module_a")] // Circular!
    }
}

registry.register_module(Box::new(ModuleA))?;
registry.register_module(Box::new(ModuleB))?; // Error: circular dependency
```

## Advanced Module Features

### Conditional Modules

Load modules based on configuration or environment:

```rust
pub struct EmailModule {
    provider: String,
}

impl EmailModule {
    pub fn new() -> Self {
        Self {
            provider: env::var("EMAIL_PROVIDER").unwrap_or_else(|_| "smtp".to_string()),
        }
    }
}

impl ServiceModule for EmailModule {
    fn configure_services(&self, container: &mut dyn ServiceBinder) -> Result<(), CoreError> {
        match self.provider.as_str() {
            "smtp" => {
                container.bind::<dyn EmailService, SmtpEmailService>();
            }
            "sendgrid" => {
                container.bind::<dyn EmailService, SendGridEmailService>();
            }
            "mock" => {
                container.bind::<dyn EmailService, MockEmailService>();
            }
            _ => return Err(CoreError::Configuration { 
                message: format!("Unknown email provider: {}", self.provider) 
            }),
        }
        
        Ok(())
    }
}

// Register conditionally
let mut registry = ModuleRegistry::new();

if cfg!(feature = "email") {
    registry.register_module(Box::new(EmailModule::new()))?;
}
```

### Module Metadata

Modules can provide metadata for introspection:

```rust
use elif_core::container::{ModuleMetadata, Version};

impl ServiceModule for DatabaseModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: "Database Module".to_string(),
            version: Version::new(1, 2, 0),
            description: "PostgreSQL database access layer".to_string(),
            author: "My Team".to_string(),
            services_count: 4, // Number of services this module provides
        }
    }
}

// Query module information
let modules_info = registry.get_modules_metadata();
for module in modules_info {
    println!("{} v{} - {}", module.name, module.version, module.description);
}
```

## Module Lifecycle Management

### Async Initialization

Modules can perform async initialization after container is built:

```rust
use async_trait::async_trait;

pub struct DatabaseModule {
    pool: Option<Arc<PgPool>>,
}

#[async_trait]
impl ServiceModule for DatabaseModule {
    async fn initialize(&mut self, container: &IocContainer) -> Result<(), CoreError> {
        // Run database migrations
        let migrator = container.resolve::<DatabaseMigrator>()?;
        migrator.run_migrations().await?;
        
        // Warm up connection pool
        let pool = container.resolve::<PgPool>()?;
        pool.warm_up().await?;
        
        self.pool = Some(pool);
        println!("Database module initialized");
        Ok(())
    }
    
    async fn shutdown(&mut self, _container: &IocContainer) -> Result<(), CoreError> {
        if let Some(pool) = &self.pool {
            pool.close().await?;
        }
        println!("Database module shut down");
        Ok(())
    }
}

// Initialize all modules
registry.initialize_all(&container).await?;

// Later, during shutdown
registry.shutdown_all(&container).await?;
```

### Parallel vs Sequential Initialization

Control initialization strategy:

```rust
use elif_core::container::{InitializationStrategy, ModuleConfig};

let mut registry = ModuleRegistry::new();

// Configure initialization strategy
let config = ModuleConfig {
    initialization_strategy: InitializationStrategy::Parallel,
    initialization_timeout: Duration::from_secs(30),
    shutdown_strategy: InitializationStrategy::Sequential, // Reverse order
};

registry.set_config(config);

// Modules with no dependencies initialize in parallel
// Dependent modules wait for their dependencies
registry.initialize_all(&container).await?;
```

## Real-World Example

Here's a complete example of a modular application:

```rust
use elif_core::container::*;

// Config Module - loaded first
pub struct ConfigModule;

impl ServiceModule for ConfigModule {
    fn module_id(&self) -> ModuleId { ModuleId::new("config") }
    
    fn configure_services(&self, container: &mut dyn ServiceBinder) -> Result<(), CoreError> {
        let app_config = AppConfig::from_file("config.toml")?;
        container.bind_instance::<AppConfig, _>(app_config);
        Ok(())
    }
}

// Database Module
pub struct DatabaseModule;

impl ServiceModule for DatabaseModule {
    fn module_id(&self) -> ModuleId { ModuleId::new("database") }
    fn depends_on(&self) -> Vec<ModuleId> { vec![ModuleId::new("config")] }
    
    fn configure_services(&self, container: &mut dyn ServiceBinder) -> Result<(), CoreError> {
        container
            .bind::<dyn UserRepository, PostgresUserRepository>()
            .bind::<dyn OrderRepository, PostgresOrderRepository>()
            .bind_factory::<PgPool, _, _>(|resolver| {
                let config = resolver.resolve::<AppConfig>()?;
                Ok(PgPool::connect(&config.database_url)?)
            });
        Ok(())
    }
}

// Web Module - handles HTTP controllers
pub struct WebModule;

impl ServiceModule for WebModule {
    fn module_id(&self) -> ModuleId { ModuleId::new("web") }
    fn depends_on(&self) -> Vec<ModuleId> { 
        vec![ModuleId::new("database"), ModuleId::new("auth")] 
    }
    
    fn configure_services(&self, container: &mut dyn ServiceBinder) -> Result<(), CoreError> {
        container
            .bind_injectable::<UserController>()
            .bind_injectable::<OrderController>()
            .bind::<dyn MiddlewareChain, DefaultMiddlewareChain>();
        Ok(())
    }
}

// Application setup
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ModuleRegistry::new();
    
    // Register modules in any order - dependencies will be sorted
    registry.register_module(Box::new(WebModule))?;
    registry.register_module(Box::new(DatabaseModule))?;
    registry.register_module(Box::new(ConfigModule))?;
    registry.register_module(Box::new(AuthModule))?;
    
    // Build container with all modules
    let mut container_builder = IocContainerBuilder::new();
    registry.configure_container(&mut container_builder)?;
    
    let container = container_builder.build()?;
    
    // Initialize all modules
    registry.initialize_all(&container).await?;
    
    // Start HTTP server with IoC-powered controllers
    let server = HttpServer::new(&container)?;
    server.listen("0.0.0.0:3000").await?;
    
    // Cleanup
    registry.shutdown_all(&container).await?;
    
    Ok(())
}
```

## Testing Modules

Test modules in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_database_module_configuration() {
        let mut registry = ModuleRegistry::new();
        registry.register_module(Box::new(ConfigModule))?;
        registry.register_module(Box::new(DatabaseModule))?;
        
        let mut builder = IocContainerBuilder::new();
        registry.configure_container(&mut builder)?;
        
        let container = builder.build()?;
        
        // Verify services are registered
        assert!(container.contains::<dyn UserRepository>());
        assert!(container.contains::<dyn OrderRepository>());
        assert!(container.contains::<PgPool>());
        
        // Test resolution
        let user_repo = container.resolve::<dyn UserRepository>()?;
        assert!(user_repo.is_ok());
    }
    
    #[tokio::test] 
    async fn test_module_initialization_order() {
        let mut registry = ModuleRegistry::new();
        
        // Register in reverse dependency order
        registry.register_module(Box::new(WebModule))?;
        registry.register_module(Box::new(DatabaseModule))?;
        registry.register_module(Box::new(ConfigModule))?;
        
        // Should resolve to correct load order
        let load_order = registry.get_load_order()?;
        
        assert_eq!(load_order[0].as_str(), "config");
        assert_eq!(load_order[1].as_str(), "database");
        assert_eq!(load_order[2].as_str(), "web");
    }
}
```

## Best Practices

### 1. Single Responsibility
Each module should have a clear, focused purpose:

```rust
// Good - focused on authentication
pub struct AuthModule;

// Avoid - mixing concerns
pub struct AuthAndEmailAndLoggingModule; 
```

### 2. Explicit Dependencies
Always declare module dependencies explicitly:

```rust
impl ServiceModule for AuthModule {
    fn depends_on(&self) -> Vec<ModuleId> {
        vec![
            ModuleId::new("database"), // Explicit dependency
            ModuleId::new("config"),   // Don't rely on implicit loading
        ]
    }
}
```

### 3. Environment-Specific Modules
Use different modules for different environments:

```rust
#[cfg(not(test))]
pub fn create_email_module() -> Box<dyn ServiceModule> {
    Box::new(SmtpEmailModule)
}

#[cfg(test)]
pub fn create_email_module() -> Box<dyn ServiceModule> {
    Box::new(MockEmailModule)
}
```

### 4. Module Versioning
Version your modules for backward compatibility:

```rust
impl ServiceModule for DatabaseModuleV2 {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            version: Version::new(2, 0, 0),
            // ... other metadata
        }
    }
    
    fn configure_services(&self, container: &mut dyn ServiceBinder) -> Result<(), CoreError> {
        // V2 services with backward compatibility
        container
            .bind::<dyn UserRepository, PostgresUserRepositoryV2>()
            .bind::<dyn UserRepository, PostgresUserRepository>() // Legacy fallback
            .bind_named::<dyn UserRepository, PostgresUserRepositoryV2>("v2");
        
        Ok(())
    }
}
```

Service modules provide a clean way to organize complex applications, manage dependencies, and handle initialization sequences. They're particularly useful for microservice architectures, plugin systems, and large monolithic applications that need better separation of concerns.