/*!
 * ServiceModule Integration Example
 *
 * This example demonstrates the complete ServiceModule system functionality,
 * showing how modules can organize and register related services with dependency
 * management and proper initialization order.
 */

use crate::container::binding::{ServiceBinder, ServiceBindings};
use crate::container::ioc_builder::IocContainerBuilder;
use crate::container::module::{ModuleId, ModuleRegistry, ServiceModule};
use crate::errors::CoreError;

// ===== Example Services =====

/// User repository interface - would typically be a trait in a real application
#[derive(Default, Debug)]
pub struct UserRepository {
    pub connection_string: String,
}

impl UserRepository {
    pub fn find_user(&self, id: u32) -> Option<String> {
        println!("UserRepository: Finding user {}", id);
        Some(format!("User-{}", id))
    }
}

/// User service that depends on UserRepository
#[derive(Default, Debug)]
pub struct UserService {
    pub cache_enabled: bool,
}

impl UserService {
    pub fn get_user(&self, id: u32) -> String {
        println!("UserService: Getting user {}", id);
        format!("UserService-{}", id)
    }
}

/// Logging service for core infrastructure
#[derive(Default, Debug)]
pub struct LoggingService {
    pub level: String,
}

impl LoggingService {
    pub fn log(&self, message: &str) {
        println!("[{}] {}", self.level, message);
    }
}

/// Configuration service for core infrastructure
#[derive(Default, Debug)]
pub struct ConfigService {
    pub environment: String,
}

// ===== Service Modules =====

/// Core infrastructure module - provides logging, config, etc.
pub struct CoreInfraModule;

impl ServiceModule for CoreInfraModule {
    fn name(&self) -> &str {
        "Core Infrastructure Module"
    }

    fn description(&self) -> Option<&str> {
        Some("Provides core infrastructure services like logging and configuration")
    }

    fn version(&self) -> Option<&str> {
        Some("1.0.0")
    }

    fn configure(&self, services: &mut ServiceBindings) {
        // Register core infrastructure services as singletons
        services.bind_singleton::<LoggingService, LoggingService>();
        services.bind_singleton::<ConfigService, ConfigService>();
    }
}

/// Data access module - provides repositories and data services
pub struct DataAccessModule;

impl ServiceModule for DataAccessModule {
    fn name(&self) -> &str {
        "Data Access Module"
    }

    fn description(&self) -> Option<&str> {
        Some("Provides data access repositories and database services")
    }

    fn version(&self) -> Option<&str> {
        Some("2.1.0")
    }

    fn depends_on(&self) -> Vec<ModuleId> {
        // Data access depends on core infrastructure
        vec![ModuleId::of::<CoreInfraModule>()]
    }

    fn configure(&self, services: &mut ServiceBindings) {
        // Register data access services
        services.bind_singleton::<UserRepository, UserRepository>();
    }
}

/// Business logic module - provides business services
pub struct BusinessLogicModule;

impl ServiceModule for BusinessLogicModule {
    fn name(&self) -> &str {
        "Business Logic Module"
    }

    fn description(&self) -> Option<&str> {
        Some("Provides business logic and application services")
    }

    fn version(&self) -> Option<&str> {
        Some("1.5.2")
    }

    fn depends_on(&self) -> Vec<ModuleId> {
        // Business logic depends on both data access and core infrastructure
        vec![
            ModuleId::of::<DataAccessModule>(),
            ModuleId::of::<CoreInfraModule>(),
        ]
    }

    fn configure(&self, services: &mut ServiceBindings) {
        // Register business services as transient (new instance per request)
        services.bind::<UserService, UserService>();
    }
}

// ===== Example Usage =====

/// Demonstrates the complete ServiceModule integration
pub fn demonstrate_module_system() -> Result<(), CoreError> {
    println!("=== ServiceModule Integration Demo ===\n");

    // 1. Create and configure the module registry
    println!("1. Creating module registry...");
    let mut registry = ModuleRegistry::new();

    // Register modules in any order - dependency resolution will sort them
    registry.register_module(BusinessLogicModule, None)?;
    registry.register_module(CoreInfraModule, None)?;
    registry.register_module(DataAccessModule, None)?;

    println!(
        "   ✓ Registered {} modules",
        registry.get_all_loaded_modules().len()
    );

    // 2. Calculate load order based on dependencies
    println!("\n2. Calculating dependency load order...");
    let load_order = registry.calculate_load_order()?;

    for (i, module_id) in load_order.iter().enumerate() {
        if let Some(loaded_module) = registry.get_loaded_module(module_id) {
            println!(
                "   {}. {} (v{})",
                i + 1,
                loaded_module.metadata.name,
                loaded_module
                    .metadata
                    .version
                    .as_deref()
                    .unwrap_or("unknown")
            );
        }
    }

    // 3. Configure all modules with a container builder
    println!("\n3. Configuring services from modules...");
    let mut container_builder = IocContainerBuilder::new();

    registry.configure_all(&mut container_builder)?;
    println!("   ✓ All modules configured");

    // 4. Build the IoC container
    println!("\n4. Building IoC container...");
    let container = container_builder.build()?;
    println!("   ✓ Container built successfully");

    // 5. Initialize all modules (async lifecycle)
    println!("\n5. Initializing modules...");
    // Note: In a real application, you would await this
    // registry.initialize_all(&container).await?;
    println!("   ✓ All modules initialized (demo - not actually awaited)");

    // 6. Demonstrate service resolution
    println!("\n6. Demonstrating service resolution:");

    // Resolve services registered by different modules
    match container.resolve::<LoggingService>() {
        Ok(logger) => {
            logger.log("Logger service resolved from CoreInfraModule");
        }
        Err(e) => println!("   ❌ Failed to resolve LoggingService: {}", e),
    }

    match container.resolve::<ConfigService>() {
        Ok(_config) => {
            println!("   ✓ ConfigService resolved from CoreInfraModule");
        }
        Err(e) => println!("   ❌ Failed to resolve ConfigService: {}", e),
    }

    match container.resolve::<UserRepository>() {
        Ok(repo) => {
            let user = repo.find_user(123);
            println!(
                "   ✓ UserRepository resolved from DataAccessModule: {:?}",
                user
            );
        }
        Err(e) => println!("   ❌ Failed to resolve UserRepository: {}", e),
    }

    match container.resolve::<UserService>() {
        Ok(service) => {
            let user = service.get_user(456);
            println!(
                "   ✓ UserService resolved from BusinessLogicModule: {}",
                user
            );
        }
        Err(e) => println!("   ❌ Failed to resolve UserService: {}", e),
    }

    // 7. Show module information
    println!("\n7. Module Status Summary:");
    for loaded_module in registry.get_all_loaded_modules() {
        println!(
            "   • {} - {:?}",
            loaded_module.metadata.name, loaded_module.state
        );
    }

    println!("\n🎉 ServiceModule integration demo completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::module::ModuleState;

    #[test]
    fn test_module_integration_demo() {
        let result = demonstrate_module_system();
        assert!(
            result.is_ok(),
            "Module integration demo should complete successfully: {:?}",
            result
        );
    }

    #[test]
    fn test_module_dependency_resolution() {
        let mut registry = ModuleRegistry::new();

        // Register modules in reverse dependency order
        registry.register_module(BusinessLogicModule, None).unwrap();
        registry.register_module(DataAccessModule, None).unwrap();
        registry.register_module(CoreInfraModule, None).unwrap();

        let load_order = registry.calculate_load_order().unwrap();

        // Should be ordered: CoreInfra -> DataAccess -> BusinessLogic
        assert_eq!(load_order.len(), 3);

        let order_names: Vec<String> = load_order
            .iter()
            .map(|id| {
                registry
                    .get_loaded_module(id)
                    .unwrap()
                    .metadata
                    .name
                    .clone()
            })
            .collect();

        assert_eq!(order_names[0], "Core Infrastructure Module");
        assert_eq!(order_names[1], "Data Access Module");
        assert_eq!(order_names[2], "Business Logic Module");
    }

    #[test]
    fn test_service_registration_through_modules() {
        let mut registry = ModuleRegistry::new();
        registry.register_module(CoreInfraModule, None).unwrap();
        registry.register_module(DataAccessModule, None).unwrap();

        let mut container_builder = IocContainerBuilder::new();
        registry.configure_all(&mut container_builder).unwrap();

        let container = container_builder.build().unwrap();

        // Services from CoreInfraModule should be available
        assert!(container.resolve::<LoggingService>().is_ok());
        assert!(container.resolve::<ConfigService>().is_ok());

        // Services from DataAccessModule should be available
        assert!(container.resolve::<UserRepository>().is_ok());
    }

    #[test]
    fn test_module_state_tracking() {
        let mut registry = ModuleRegistry::new();
        registry.register_module(CoreInfraModule, None).unwrap();

        // Initially should be registered
        let module_id = ModuleId::of::<CoreInfraModule>();
        let loaded_module = registry.get_loaded_module(&module_id).unwrap();
        assert_eq!(loaded_module.state, ModuleState::Registered);

        // After configuration should be configured
        let mut container_builder = IocContainerBuilder::new();
        registry.configure_all(&mut container_builder).unwrap();

        let loaded_module = registry.get_loaded_module(&module_id).unwrap();
        assert_eq!(loaded_module.state, ModuleState::Configured);
    }
}
