use crate::container::{Container, ContainerBuilder};
use crate::providers::{ProviderError, ProviderRegistry, ServiceProvider};
use std::time::Instant;

/// Provider lifecycle manager
pub struct ProviderLifecycleManager {
    registry: ProviderRegistry,
    lifecycle_stats: ProviderLifecycleStats,
}

impl ProviderLifecycleManager {
    /// Create a new provider lifecycle manager
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
            lifecycle_stats: ProviderLifecycleStats::new(),
        }
    }

    /// Register a provider
    pub fn register<P: ServiceProvider + 'static>(&mut self, provider: P) {
        self.registry.register(provider);
    }

    /// Execute full provider lifecycle and return built container
    pub async fn execute_lifecycle(
        &mut self,
        builder: ContainerBuilder,
    ) -> Result<Container, ProviderError> {
        let start_time = Instant::now();

        tracing::info!("Starting provider lifecycle execution...");

        // Resolve dependencies
        let dep_start = Instant::now();
        self.registry.resolve_dependencies()?;
        self.lifecycle_stats.dependency_resolution_time = dep_start.elapsed();

        // Register all providers
        let reg_start = Instant::now();
        let builder = self.registry.register_all(builder)?;
        self.lifecycle_stats.registration_time = reg_start.elapsed();

        // Build container
        let build_start = Instant::now();
        let mut container = builder.build()?;
        container.initialize().await?;
        self.lifecycle_stats.container_build_time = build_start.elapsed();

        // Boot all providers
        let boot_start = Instant::now();
        self.registry.boot_all(&container)?;
        self.lifecycle_stats.boot_time = boot_start.elapsed();

        self.lifecycle_stats.total_time = start_time.elapsed();
        self.lifecycle_stats.provider_count = self.registry.provider_count();

        tracing::info!(
            "Provider lifecycle completed successfully in {:?} with {} providers",
            self.lifecycle_stats.total_time,
            self.lifecycle_stats.provider_count
        );

        Ok(container)
    }

    /// Get lifecycle statistics
    pub fn lifecycle_stats(&self) -> &ProviderLifecycleStats {
        &self.lifecycle_stats
    }

    /// Get provider registry
    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }
}

impl Default for ProviderLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for provider lifecycle execution
#[derive(Debug, Clone)]
pub struct ProviderLifecycleStats {
    pub provider_count: usize,
    pub total_time: std::time::Duration,
    pub dependency_resolution_time: std::time::Duration,
    pub registration_time: std::time::Duration,
    pub container_build_time: std::time::Duration,
    pub boot_time: std::time::Duration,
}

impl ProviderLifecycleStats {
    /// Create new lifecycle stats
    pub fn new() -> Self {
        Self {
            provider_count: 0,
            total_time: std::time::Duration::ZERO,
            dependency_resolution_time: std::time::Duration::ZERO,
            registration_time: std::time::Duration::ZERO,
            container_build_time: std::time::Duration::ZERO,
            boot_time: std::time::Duration::ZERO,
        }
    }
}

impl Default for ProviderLifecycleStats {
    fn default() -> Self {
        Self::new()
    }
}
