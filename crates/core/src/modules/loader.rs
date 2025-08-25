use crate::container::{Container, ContainerBuilder};
use crate::modules::{Module, ModuleError, ModuleRegistry};
use std::time::Instant;

/// Module loader for orchestrating module loading process
pub struct ModuleLoader {
    registry: ModuleRegistry,
    loading_stats: LoadingStats,
}

impl ModuleLoader {
    /// Create a new module loader
    pub fn new() -> Self {
        Self {
            registry: ModuleRegistry::new(),
            loading_stats: LoadingStats::new(),
        }
    }

    /// Create a new module loader with existing registry
    pub fn with_registry(registry: ModuleRegistry) -> Self {
        Self {
            registry,
            loading_stats: LoadingStats::new(),
        }
    }

    /// Register a module
    pub fn register<M: Module + 'static>(&mut self, module: M) {
        self.registry.register(module);
    }

    /// Register multiple modules at once
    pub fn register_modules<M: Module + 'static>(&mut self, modules: Vec<M>) {
        for module in modules {
            self.registry.register(module);
        }
    }

    /// Load all modules and create container
    pub async fn load(&mut self) -> Result<Container, ModuleError> {
        let start_time = Instant::now();

        tracing::info!("Starting module loading process...");

        // Validate modules
        self.registry.validate()?;
        self.loading_stats.validation_time = start_time.elapsed();

        // Resolve dependencies
        let dep_start = Instant::now();
        self.registry.resolve_dependencies()?;
        self.loading_stats.dependency_resolution_time = dep_start.elapsed();

        // Configure container
        let config_start = Instant::now();
        let builder = ContainerBuilder::new();
        let builder = self.registry.configure_all(builder)?;
        let mut container = builder.build()?;
        self.loading_stats.configuration_time = config_start.elapsed();

        // Initialize container
        let init_start = Instant::now();
        container.initialize().await?;
        self.loading_stats.initialization_time = init_start.elapsed();

        // Boot modules
        let boot_start = Instant::now();
        self.registry.boot_all(&container)?;
        self.loading_stats.boot_time = boot_start.elapsed();

        self.loading_stats.total_time = start_time.elapsed();
        self.loading_stats.module_count = self.registry.module_count();

        tracing::info!(
            "Module loading completed successfully in {:?} with {} modules",
            self.loading_stats.total_time,
            self.loading_stats.module_count
        );

        Ok(container)
    }

    /// Load modules without creating container (for testing)
    pub fn load_modules_only(&mut self) -> Result<(), ModuleError> {
        self.registry.validate()?;
        self.registry.resolve_dependencies()?;
        Ok(())
    }

    /// Get loading statistics
    pub fn loading_stats(&self) -> &LoadingStats {
        &self.loading_stats
    }

    /// Get module registry
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    /// Get mutable module registry
    pub fn registry_mut(&mut self) -> &mut ModuleRegistry {
        &mut self.registry
    }

    /// Get all routes from loaded modules
    pub fn routes(&mut self) -> Vec<crate::modules::RouteDefinition> {
        self.registry.collect_routes()
    }

    /// Get all middleware from loaded modules
    pub fn middleware(&mut self) -> Vec<crate::modules::MiddlewareDefinition> {
        self.registry.collect_middleware()
    }

    /// Print loading summary
    pub fn print_summary(&self) {
        let stats = &self.loading_stats;

        println!("\n=== Module Loading Summary ===");
        println!("Modules loaded: {}", stats.module_count);
        println!("Total time: {:?}", stats.total_time);
        println!("  - Validation: {:?}", stats.validation_time);
        println!(
            "  - Dependency resolution: {:?}",
            stats.dependency_resolution_time
        );
        println!("  - Configuration: {:?}", stats.configuration_time);
        println!("  - Initialization: {:?}", stats.initialization_time);
        println!("  - Boot: {:?}", stats.boot_time);
        println!("===============================\n");
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for module loading process
#[derive(Debug, Clone)]
pub struct LoadingStats {
    pub module_count: usize,
    pub total_time: std::time::Duration,
    pub validation_time: std::time::Duration,
    pub dependency_resolution_time: std::time::Duration,
    pub configuration_time: std::time::Duration,
    pub initialization_time: std::time::Duration,
    pub boot_time: std::time::Duration,
}

impl LoadingStats {
    /// Create new loading stats
    pub fn new() -> Self {
        Self {
            module_count: 0,
            total_time: std::time::Duration::ZERO,
            validation_time: std::time::Duration::ZERO,
            dependency_resolution_time: std::time::Duration::ZERO,
            configuration_time: std::time::Duration::ZERO,
            initialization_time: std::time::Duration::ZERO,
            boot_time: std::time::Duration::ZERO,
        }
    }

    /// Get the percentage of time spent on each phase
    pub fn phase_percentages(&self) -> PhasePercentages {
        let total_micros = self.total_time.as_micros() as f64;

        if total_micros == 0.0 {
            return PhasePercentages::default();
        }

        PhasePercentages {
            validation: (self.validation_time.as_micros() as f64 / total_micros) * 100.0,
            dependency_resolution: (self.dependency_resolution_time.as_micros() as f64
                / total_micros)
                * 100.0,
            configuration: (self.configuration_time.as_micros() as f64 / total_micros) * 100.0,
            initialization: (self.initialization_time.as_micros() as f64 / total_micros) * 100.0,
            boot: (self.boot_time.as_micros() as f64 / total_micros) * 100.0,
        }
    }
}

impl Default for LoadingStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Phase percentages for loading stats
#[derive(Debug, Clone, Default)]
pub struct PhasePercentages {
    pub validation: f64,
    pub dependency_resolution: f64,
    pub configuration: f64,
    pub initialization: f64,
    pub boot: f64,
}

/// Builder for module loader
pub struct ModuleLoaderBuilder {
    loader: ModuleLoader,
}

impl ModuleLoaderBuilder {
    /// Create a new module loader builder
    pub fn new() -> Self {
        Self {
            loader: ModuleLoader::new(),
        }
    }

    /// Add a module
    pub fn add_module<M: Module + 'static>(mut self, module: M) -> Self {
        self.loader.register(module);
        self
    }

    /// Add multiple modules
    pub fn add_modules<M: Module + 'static>(mut self, modules: Vec<M>) -> Self {
        self.loader.register_modules(modules);
        self
    }

    /// Build and return the module loader
    pub fn build(self) -> ModuleLoader {
        self.loader
    }

    /// Build, load modules, and return container
    pub async fn load(mut self) -> Result<Container, ModuleError> {
        self.loader.load().await
    }
}

impl Default for ModuleLoaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::BaseModule;

    #[tokio::test]
    async fn test_module_loader() -> Result<(), ModuleError> {
        let mut loader = ModuleLoader::new();

        loader.register(BaseModule::new("test_module_1"));
        loader.register(BaseModule::new("test_module_2"));

        let container = loader.load().await?;

        assert!(container.is_initialized());
        assert_eq!(loader.loading_stats().module_count, 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_module_loader_builder() -> Result<(), ModuleError> {
        let container = ModuleLoaderBuilder::new()
            .add_module(BaseModule::new("test_module_1"))
            .add_module(BaseModule::new("test_module_2"))
            .load()
            .await?;

        assert!(container.is_initialized());

        Ok(())
    }
}
