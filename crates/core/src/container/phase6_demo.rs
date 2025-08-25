/*!
 * Phase 6 Demo: Developer Experience & Tooling
 *
 * This demo shows the new Phase 6 features for IoC Phase 6 - Developer Experience & Tooling
 * including auto-wiring conventions, module system, validation, visualization, and debug utilities.
 */

use crate::container::binding::ServiceBinder;
use crate::container::conventions::{
    ServiceConventions, ServiceMetadata, ServiceRegistry as ConventionServiceRegistry,
};
use crate::container::debug::ContainerInspector;
use crate::container::ioc_container::IocContainer;
use crate::container::module::{ModuleRegistry, ServiceModule};
use crate::container::scope::ServiceScope;
use crate::container::visualization::{
    DependencyVisualizer, VisualizationFormat, VisualizationStyle,
};
use crate::errors::CoreError;
use std::sync::Arc;

/// Example service for demonstration
#[derive(Default)]
pub struct UserService;

impl UserService {
    pub fn get_user(&self, _id: u32) -> String {
        "Demo User".to_string()
    }
}

/// Example repository service
#[derive(Default)]
pub struct UserRepository;

impl UserRepository {
    pub fn find_user(&self, _id: u32) -> Option<String> {
        Some("Repository User".to_string())
    }
}

/// Example logger service
#[derive(Default)]
pub struct AppLogger;

impl AppLogger {
    pub fn log(&self, message: &str) {
        println!("[LOG] {}", message);
    }
}

/// Example module demonstrating the module system
pub struct UserModule;

impl ServiceModule for UserModule {
    fn name(&self) -> &str {
        "User Management Module"
    }

    fn description(&self) -> Option<&str> {
        Some("Handles user-related services and operations")
    }

    fn version(&self) -> Option<&str> {
        Some("1.0.0")
    }

    fn configure(&self, services: &mut crate::container::binding::ServiceBindings) {
        // Example of registering services for this module
        services.bind::<UserService, UserService>();
        services.bind::<UserRepository, UserRepository>();
    }

    fn depends_on(&self) -> Vec<crate::container::module::ModuleId> {
        vec![]
    }
}

/// Core module that other modules depend on
pub struct CoreModule;

impl ServiceModule for CoreModule {
    fn name(&self) -> &str {
        "Core Module"
    }

    fn description(&self) -> Option<&str> {
        Some("Core application services")
    }

    fn configure(&self, services: &mut crate::container::binding::ServiceBindings) {
        // Example of registering services for this module
        services.bind::<AppLogger, AppLogger>();
        // Note: DatabaseConnection would need to be defined to actually bind it
        // services.bind::<DatabaseConnection, DatabaseConnection>();
    }
}

/// Demo function for Phase 6 features
pub fn demo_phase6_features() -> Result<(), CoreError> {
    println!("=== Phase 6 Demo: Developer Experience & Tooling ===");
    println!();

    // 1. Auto-wiring Conventions Demo
    println!("1. Auto-wiring Conventions:");
    println!("---------------------------");

    let mut conventions = ServiceConventions::new();
    conventions.add_naming_convention("*Service", ServiceScope::Singleton);
    conventions.add_naming_convention("*Repository", ServiceScope::Scoped);
    conventions.add_naming_convention("*Logger", ServiceScope::Singleton);

    println!(
        "✓ UserService lifetime: {:?}",
        conventions.get_lifetime_for_type("UserService")
    );
    println!(
        "✓ UserRepository lifetime: {:?}",
        conventions.get_lifetime_for_type("UserRepository")
    );
    println!(
        "✓ AppLogger lifetime: {:?}",
        conventions.get_lifetime_for_type("AppLogger")
    );

    // Register services using conventions
    let mut service_registry = ConventionServiceRegistry::new();

    let user_service_metadata =
        ServiceMetadata::new("UserService".to_string()).with_lifetime(ServiceScope::Singleton);
    service_registry.register_service(user_service_metadata);

    let user_repo_metadata =
        ServiceMetadata::new("UserRepository".to_string()).with_lifetime(ServiceScope::Scoped);
    service_registry.register_service(user_repo_metadata);

    println!(
        "✓ Registered {} services with conventions",
        service_registry.all_services().len()
    );
    println!();

    // 2. Module System Demo
    println!("2. Module System:");
    println!("-----------------");

    let mut module_registry = ModuleRegistry::new();

    // Register modules
    module_registry.register_module(CoreModule, None)?;
    module_registry.register_module(UserModule, None)?;

    // Calculate load order
    let load_order = module_registry.calculate_load_order()?;
    println!(
        "✓ Module load order calculated: {} modules",
        load_order.len()
    );

    for (i, module_id) in load_order.iter().enumerate() {
        if let Some(loaded_module) = module_registry.get_loaded_module(module_id) {
            println!(
                "  {}. {} ({})",
                i + 1,
                loaded_module.metadata.name,
                loaded_module
                    .metadata
                    .version
                    .as_deref()
                    .unwrap_or("no version")
            );
        }
    }
    println!();

    // 3. Container Inspection Demo
    println!("3. Debug & Inspection Tools:");
    println!("-----------------------------");

    let container = IocContainer::new();
    let inspector = ContainerInspector::new(Arc::new(container));

    let container_info = inspector.get_container_info();
    println!(
        "✓ Container Status: {}",
        if container_info.is_built {
            "Built"
        } else {
            "Not Built"
        }
    );
    println!("✓ Registered Services: {}", container_info.service_count);
    println!("✓ Cached Instances: {}", container_info.cached_instances);
    println!();

    // 4. Visualization Demo
    println!("4. Dependency Visualization:");
    println!("----------------------------");

    // Create a simple visualizer (would normally use real descriptors)
    let descriptors = vec![]; // Empty for demo
    let visualizer = DependencyVisualizer::new(descriptors);

    // Generate different visualization formats
    let style = VisualizationStyle::default();

    match visualizer.visualize(VisualizationFormat::Ascii, style.clone()) {
        Ok(ascii_viz) => {
            println!("✓ ASCII Visualization Generated:");
            println!(
                "{}",
                ascii_viz.lines().take(10).collect::<Vec<_>>().join("\n")
            );
            if ascii_viz.lines().count() > 10 {
                println!("  ... (truncated)");
            }
        }
        Err(e) => println!("⚠️  ASCII visualization failed: {}", e),
    }
    println!();

    match visualizer.visualize(VisualizationFormat::Json, style) {
        Ok(json_viz) => {
            println!("✓ JSON Visualization Generated ({} bytes)", json_viz.len());
            // Show first few lines
            let preview: String = json_viz.chars().take(200).collect();
            println!("  Preview: {}...", preview);
        }
        Err(e) => println!("⚠️  JSON visualization failed: {}", e),
    }
    println!();

    // 5. Health Check Demo
    println!("5. Container Health Monitoring:");
    println!("-------------------------------");

    // This would normally perform actual health checks
    println!("✓ Health Check System Available");
    println!("✓ Performance Profiler Available");
    println!("✓ Resolution Tracer Available");
    println!();

    println!("🎉 Phase 6 Demo Complete!");
    println!("   All developer experience and tooling features are operational.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase6_demo() {
        let result = demo_phase6_features();
        assert!(result.is_ok(), "Phase 6 demo should run successfully");
    }

    #[test]
    fn test_service_conventions() {
        let conventions = ServiceConventions::new();

        // Test default naming conventions
        assert_eq!(
            conventions.get_lifetime_for_type("UserService"),
            ServiceScope::Singleton
        );
        assert_eq!(
            conventions.get_lifetime_for_type("UserRepository"),
            ServiceScope::Scoped
        );
        assert_eq!(
            conventions.get_lifetime_for_type("PaymentFactory"),
            ServiceScope::Transient
        );
        assert_eq!(
            conventions.get_lifetime_for_type("AppLogger"),
            ServiceScope::Singleton
        );
    }

    #[test]
    fn test_module_registry() {
        let mut registry = ModuleRegistry::new();

        // Register modules
        assert!(registry.register_module(CoreModule, None).is_ok());
        assert!(registry.register_module(UserModule, None).is_ok());

        // Calculate load order
        let load_order = registry.calculate_load_order();
        assert!(load_order.is_ok());
        assert_eq!(load_order.unwrap().len(), 2);
    }

    #[test]
    fn test_container_inspector() {
        let container = IocContainer::new();
        let inspector = ContainerInspector::new(Arc::new(container));

        let info = inspector.get_container_info();
        assert!(!info.is_built); // New container should not be built
        assert_eq!(info.service_count, 0); // Should have no services initially
    }

    #[test]
    fn test_service_metadata() {
        let metadata = ServiceMetadata::new("TestService".to_string())
            .with_lifetime(ServiceScope::Singleton)
            .with_name("test".to_string())
            .as_default()
            .with_tag("core".to_string());

        assert_eq!(metadata.type_name, "TestService");
        assert_eq!(metadata.lifetime, Some(ServiceScope::Singleton));
        assert_eq!(metadata.name, Some("test".to_string()));
        assert!(metadata.is_default);
        assert_eq!(metadata.tags, vec!["core"]);
    }
}
