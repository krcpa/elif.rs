use crate::container::ContainerBuilder;
use crate::providers::{ProviderError, ProviderMetadata, ServiceProvider};
use std::collections::HashMap;

/// Provider registry manages service providers and their lifecycle
pub struct ProviderRegistry {
    providers: Vec<Box<dyn ServiceProvider>>,
    registration_order: Vec<usize>,
    boot_order: Vec<usize>,
    metadata_cache: HashMap<String, ProviderMetadata>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            registration_order: Vec::new(),
            boot_order: Vec::new(),
            metadata_cache: HashMap::new(),
        }
    }

    /// Register a service provider
    pub fn register<P: ServiceProvider + 'static>(&mut self, provider: P) {
        let metadata = ProviderMetadata::from_provider(&provider);
        let name = metadata.name.clone();

        self.providers.push(Box::new(provider));
        self.metadata_cache.insert(name, metadata);
    }

    /// Get the number of registered providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Get provider metadata by name
    pub fn get_metadata(&self, name: &str) -> Option<&ProviderMetadata> {
        self.metadata_cache.get(name)
    }

    /// Resolve provider dependencies and determine execution order
    pub fn resolve_dependencies(&mut self) -> Result<(), ProviderError> {
        let name_to_index: HashMap<String, usize> = self
            .providers
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name().to_string(), i))
            .collect();

        // Resolve registration order (topological sort)
        self.registration_order = self.topological_sort(&name_to_index, false)?;

        // Resolve boot order (separate sort considering defer_boot)
        self.boot_order = self.topological_sort(&name_to_index, true)?;

        Ok(())
    }

    /// Perform topological sort considering dependencies
    fn topological_sort(
        &self,
        name_to_index: &HashMap<String, usize>,
        consider_defer: bool,
    ) -> Result<Vec<usize>, ProviderError> {
        let provider_count = self.providers.len();
        let mut visited = vec![false; provider_count];
        let mut temp_mark = vec![false; provider_count];
        let mut result = Vec::new();

        // Visit all providers
        for i in 0..provider_count {
            if !visited[i] {
                self.visit_provider(
                    i,
                    name_to_index,
                    &mut visited,
                    &mut temp_mark,
                    &mut result,
                    consider_defer,
                )?;
            }
        }

        Ok(result)
    }

    /// Visit provider for dependency resolution
    #[allow(clippy::only_used_in_recursion)]
    fn visit_provider(
        &self,
        index: usize,
        name_to_index: &HashMap<String, usize>,
        visited: &mut Vec<bool>,
        temp_mark: &mut Vec<bool>,
        result: &mut Vec<usize>,
        consider_defer: bool,
    ) -> Result<(), ProviderError> {
        if temp_mark[index] {
            return Err(ProviderError::CircularDependency {
                provider: self.providers[index].name().to_string(),
            });
        }

        if visited[index] {
            return Ok(());
        }

        temp_mark[index] = true;

        // Visit all dependencies first
        let dependencies = self.providers[index].dependencies();
        for dep_name in dependencies {
            if let Some(&dep_index) = name_to_index.get(dep_name) {
                self.visit_provider(
                    dep_index,
                    name_to_index,
                    visited,
                    temp_mark,
                    result,
                    consider_defer,
                )?;
            } else {
                return Err(ProviderError::MissingDependency {
                    provider: self.providers[index].name().to_string(),
                    dependency: dep_name.to_string(),
                });
            }
        }

        temp_mark[index] = false;
        visited[index] = true;
        result.push(index);

        Ok(())
    }

    /// Register all providers with the container builder
    pub fn register_all(
        &self,
        mut builder: ContainerBuilder,
    ) -> Result<ContainerBuilder, ProviderError> {
        for &index in &self.registration_order {
            let provider = &self.providers[index];
            tracing::info!("Registering provider: {}", provider.name());
            builder = provider.register(builder)?;
        }
        Ok(builder)
    }

    /// Boot all providers after container is built
    pub fn boot_all(&self, container: &crate::container::Container) -> Result<(), ProviderError> {
        for &index in &self.boot_order {
            let provider = &self.providers[index];
            tracing::info!("Booting provider: {}", provider.name());
            provider
                .boot(container)
                .map_err(|e| ProviderError::BootFailed {
                    message: format!("Failed to boot provider '{}': {}", provider.name(), e),
                })?;
        }
        Ok(())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
