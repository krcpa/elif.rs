//! Provider Auto-Configuration System
//!
//! This module implements automatic provider registration from module declarations
//! with proper lifetime management and dependency injection.

use std::collections::HashMap;
use crate::container::{IocContainer, ServiceConventions, ServiceLifetime};
use crate::modules::CompileTimeModuleMetadata;
use crate::errors::CoreError;

/// Configuration error types for provider auto-configuration
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Provider configuration failed: {message}")]
    ConfigurationFailed { message: String },
    
    #[error("Dependency validation failed: {0}")]
    DependencyError(#[from] DependencyError),
    
    #[error("Container error during configuration: {0}")]
    ContainerError(#[from] CoreError),
}

/// Dependency resolution error types
#[derive(Debug, thiserror::Error)]
pub enum DependencyError {
    #[error("Missing dependency '{dependency}' for service '{service}'")]
    MissingDependency {
        service: String,
        dependency: String,
    },
    
    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },
    
    #[error("Service lifetime conflict: {service} ({lifetime:?}) depends on {dependency} ({dependency_lifetime:?})")]
    LifetimeConflict {
        service: String,
        lifetime: ServiceLifetime,
        dependency: String,
        dependency_lifetime: ServiceLifetime,
    },
}

/// Provider type enumeration for different binding strategies
#[derive(Debug, Clone)]
pub enum ProviderType {
    /// Concrete service implementation
    Concrete(String),
    /// Trait/interface implementation
    Trait(String, String), // (trait_name, impl_name)
    /// Named service implementation
    Named(String, String), // (service_name, name)
}

/// Provider metadata extracted from module declarations
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub service_type: ProviderType,
    pub lifetime: Option<ServiceLifetime>,
    pub dependencies: Vec<String>,
}

impl ProviderMetadata {
    /// Create new provider metadata for concrete service
    pub fn concrete(service_name: String) -> Self {
        Self {
            service_type: ProviderType::Concrete(service_name),
            lifetime: None,
            dependencies: Vec::new(),
        }
    }
    
    /// Create new provider metadata for trait implementation
    pub fn trait_impl(trait_name: String, impl_name: String) -> Self {
        Self {
            service_type: ProviderType::Trait(trait_name, impl_name),
            lifetime: None,
            dependencies: Vec::new(),
        }
    }
    
    /// Create new provider metadata for named service
    pub fn named(service_name: String, name: String) -> Self {
        Self {
            service_type: ProviderType::Named(service_name, name),
            lifetime: None,
            dependencies: Vec::new(),
        }
    }
    
    /// Set the service lifetime
    pub fn with_lifetime(mut self, lifetime: ServiceLifetime) -> Self {
        self.lifetime = Some(lifetime);
        self
    }
    
    /// Add a dependency
    pub fn with_dependency(mut self, dependency: String) -> Self {
        self.dependencies.push(dependency);
        self
    }
}

/// Provider configuration engine for automatic DI container setup
pub struct ProviderConfigurator {
    container: IocContainer,
    conventions: ServiceConventions,
    providers: Vec<ProviderMetadata>,
}

impl ProviderConfigurator {
    /// Create a new provider configurator
    pub fn new(container: IocContainer) -> Self {
        Self {
            container,
            conventions: ServiceConventions::new(),
            providers: Vec::new(),
        }
    }
    
    /// Create a new provider configurator with custom conventions
    pub fn with_conventions(container: IocContainer, conventions: ServiceConventions) -> Self {
        Self {
            container,
            conventions,
            providers: Vec::new(),
        }
    }
    
    /// Configure providers from module metadata
    pub fn configure_from_modules(&mut self, modules: &[CompileTimeModuleMetadata]) -> Result<(), ConfigError> {
        // Extract all providers from modules
        self.extract_providers_from_modules(modules)?;
        
        // Apply lifetime conventions
        self.apply_lifetime_conventions()?;
        
        // Validate dependencies
        self.validate_dependencies()?;
        
        // Register providers in container
        self.register_providers()?;
        
        Ok(())
    }
    
    /// Extract providers from module metadata
    fn extract_providers_from_modules(&mut self, modules: &[CompileTimeModuleMetadata]) -> Result<(), ConfigError> {
        for module in modules {
            for provider_str in &module.providers {
                let provider = self.parse_provider_declaration(provider_str)?;
                self.providers.push(provider);
            }
        }
        Ok(())
    }
    
    /// Parse provider declaration string into ProviderMetadata
    fn parse_provider_declaration(&self, provider_str: &str) -> Result<ProviderMetadata, ConfigError> {
        // Handle different provider declaration patterns:
        // - "UserService" -> Concrete service
        // - "dyn EmailService => SmtpEmailService" -> Trait mapping
        // - "dyn EmailService => SmtpEmailService @ smtp" -> Named service
        
        if let Some(arrow_pos) = provider_str.find(" => ") {
            // Trait mapping or named service
            let trait_part = provider_str[..arrow_pos].trim();
            let impl_part = provider_str[arrow_pos + 4..].trim();
            
            // Remove "dyn " prefix if present
            let trait_name = if let Some(stripped) = trait_part.strip_prefix("dyn ") {
                stripped.trim().to_string()
            } else {
                trait_part.to_string()
            };
            
            // Check for named service (@ symbol)
            if let Some(at_pos) = impl_part.find(" @ ") {
                let impl_name = impl_part[..at_pos].trim().to_string();
                let service_name = impl_part[at_pos + 3..].trim().to_string();
                Ok(ProviderMetadata::named(impl_name, service_name))
            } else {
                Ok(ProviderMetadata::trait_impl(trait_name, impl_part.to_string()))
            }
        } else {
            // Concrete service
            Ok(ProviderMetadata::concrete(provider_str.trim().to_string()))
        }
    }
    
    /// Apply lifetime conventions to providers
    pub fn apply_lifetime_conventions(&mut self) -> Result<(), ConfigError> {
        for provider in &mut self.providers {
            if provider.lifetime.is_none() {
                let service_name = match &provider.service_type {
                    ProviderType::Concrete(name) => name,
                    ProviderType::Trait(_, impl_name) => impl_name,
                    ProviderType::Named(service_name, _) => service_name,
                };
                
                let lifetime = self.conventions.get_lifetime_for_type(service_name);
                provider.lifetime = Some(lifetime);
            }
        }
        Ok(())
    }
    
    /// Validate all dependencies can be resolved
    pub fn validate_dependencies(&self) -> Result<(), DependencyError> {
        // Build service registry
        let mut services: HashMap<String, &ProviderMetadata> = HashMap::new();
        
        for provider in &self.providers {
            let service_name = match &provider.service_type {
                ProviderType::Concrete(name) => name.clone(),
                ProviderType::Trait(trait_name, _) => trait_name.clone(),
                ProviderType::Named(_, name) => name.clone(),
            };
            services.insert(service_name, provider);
        }
        
        // Check each service's dependencies
        for provider in &self.providers {
            let service_name = match &provider.service_type {
                ProviderType::Concrete(name) => name.clone(),
                ProviderType::Trait(trait_name, _) => trait_name.clone(),
                ProviderType::Named(_, name) => name.clone(),
            };
            
            for dependency in &provider.dependencies {
                if !services.contains_key(dependency) {
                    return Err(DependencyError::MissingDependency {
                        service: service_name.clone(),
                        dependency: dependency.clone(),
                    });
                }
                
                // Check lifetime compatibility
                if let Some(dep_provider) = services.get(dependency) {
                    self.validate_lifetime_compatibility(provider, dep_provider, &service_name, dependency)?;
                }
            }
        }
        
        // Check for circular dependencies
        self.detect_circular_dependencies(&services)?;
        
        Ok(())
    }
    
    /// Validate service lifetime compatibility
    fn validate_lifetime_compatibility(
        &self,
        service: &ProviderMetadata,
        dependency: &ProviderMetadata,
        service_name: &str,
        dependency_name: &str,
    ) -> Result<(), DependencyError> {
        let service_lifetime = service.lifetime.unwrap_or(ServiceLifetime::Transient);
        let dependency_lifetime = dependency.lifetime.unwrap_or(ServiceLifetime::Transient);
        
        // Check lifetime rules:
        // - Singleton can depend on any lifetime
        // - Scoped cannot depend on Transient
        // - Transient can depend on any lifetime
        match (service_lifetime, dependency_lifetime) {
            (ServiceLifetime::Scoped, ServiceLifetime::Transient) => {
                Err(DependencyError::LifetimeConflict {
                    service: service_name.to_string(),
                    lifetime: service_lifetime,
                    dependency: dependency_name.to_string(),
                    dependency_lifetime,
                })
            }
            _ => Ok(()),
        }
    }
    
    /// Detect circular dependencies using depth-first search
    fn detect_circular_dependencies(
        &self,
        services: &HashMap<String, &ProviderMetadata>,
    ) -> Result<(), DependencyError> {
        let mut visited = HashMap::new();
        let mut rec_stack = HashMap::new();
        
        for service_name in services.keys() {
            if !visited.get(service_name).unwrap_or(&false) {
                if let Some(cycle) = self.dfs_cycle_detection(
                    service_name,
                    services,
                    &mut visited,
                    &mut rec_stack,
                    Vec::new(),
                )? {
                    return Err(DependencyError::CircularDependency {
                        cycle: cycle.join(" → "),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Depth-first search for cycle detection
    fn dfs_cycle_detection(
        &self,
        service: &str,
        services: &HashMap<String, &ProviderMetadata>,
        visited: &mut HashMap<String, bool>,
        rec_stack: &mut HashMap<String, bool>,
        mut path: Vec<String>,
    ) -> Result<Option<Vec<String>>, DependencyError> {
        visited.insert(service.to_string(), true);
        rec_stack.insert(service.to_string(), true);
        path.push(service.to_string());
        
        if let Some(provider) = services.get(service) {
            for dependency in &provider.dependencies {
                if !visited.get(dependency).unwrap_or(&false) {
                    if let Some(cycle) = self.dfs_cycle_detection(
                        dependency,
                        services,
                        visited,
                        rec_stack,
                        path.clone(),
                    )? {
                        return Ok(Some(cycle));
                    }
                } else if *rec_stack.get(dependency).unwrap_or(&false) {
                    // Found cycle
                    let cycle_start = path.iter().position(|s| s == dependency).unwrap_or(0);
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dependency.to_string());
                    return Ok(Some(cycle));
                }
            }
        }
        
        rec_stack.insert(service.to_string(), false);
        Ok(None)
    }
    
    /// Register all providers in the DI container
    fn register_providers(&mut self) -> Result<(), ConfigError> {
        // Clone providers to avoid borrow checker issues
        let providers = self.providers.clone();
        
        for provider in &providers {
            let lifetime = provider.lifetime.unwrap_or(ServiceLifetime::Transient);
            
            match &provider.service_type {
                ProviderType::Concrete(service_type) => {
                    // For now, we'll use placeholder registration
                    // In a real implementation, this would use reflection or codegen
                    // to actually register the concrete types
                    self.register_concrete_service(service_type, lifetime)?;
                }
                ProviderType::Trait(trait_type, impl_type) => {
                    self.register_trait_service(trait_type, impl_type, lifetime)?;
                }
                ProviderType::Named(service_type, name) => {
                    self.register_named_service(service_type, name, lifetime)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Register a concrete service (placeholder implementation)
    fn register_concrete_service(&mut self, _service_type: &str, _lifetime: ServiceLifetime) -> Result<(), ConfigError> {
        // TODO: Implement actual service registration using codegen or reflection
        // For now, this is a placeholder that would be filled in with actual
        // container.bind::<ServiceType, ServiceType>() calls
        Ok(())
    }
    
    /// Register a trait service (placeholder implementation)
    fn register_trait_service(
        &mut self,
        _trait_type: &str,
        _impl_type: &str,
        _lifetime: ServiceLifetime,
    ) -> Result<(), ConfigError> {
        // TODO: Implement actual trait registration using codegen or reflection
        // For now, this is a placeholder that would be filled in with actual
        // container.bind_trait::<TraitType, ImplType>() calls
        Ok(())
    }
    
    /// Register a named service (placeholder implementation)
    fn register_named_service(
        &mut self,
        _service_type: &str,
        _name: &str,
        _lifetime: ServiceLifetime,
    ) -> Result<(), ConfigError> {
        // TODO: Implement actual named service registration using codegen or reflection
        // For now, this is a placeholder that would be filled in with actual
        // container.bind_named::<ServiceType>(name) calls
        Ok(())
    }
    
    /// Get the configured container
    pub fn into_container(self) -> IocContainer {
        self.container
    }
    
    /// Get reference to the container
    pub fn container(&self) -> &IocContainer {
        &self.container
    }
    
    /// Get mutable reference to the container
    pub fn container_mut(&mut self) -> &mut IocContainer {
        &mut self.container
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::CompileTimeModuleMetadata;
    
    #[test]
    fn test_parse_concrete_provider() {
        let configurator = ProviderConfigurator::new(IocContainer::new());
        let provider = configurator.parse_provider_declaration("UserService").unwrap();
        
        match provider.service_type {
            ProviderType::Concrete(name) => assert_eq!(name, "UserService"),
            _ => panic!("Expected concrete provider"),
        }
    }
    
    #[test]
    fn test_parse_trait_provider() {
        let configurator = ProviderConfigurator::new(IocContainer::new());
        let provider = configurator.parse_provider_declaration("dyn EmailService => SmtpEmailService").unwrap();
        
        match provider.service_type {
            ProviderType::Trait(trait_name, impl_name) => {
                assert_eq!(trait_name, "EmailService");
                assert_eq!(impl_name, "SmtpEmailService");
            }
            _ => panic!("Expected trait provider"),
        }
    }
    
    #[test]
    fn test_parse_named_provider() {
        let configurator = ProviderConfigurator::new(IocContainer::new());
        let provider = configurator.parse_provider_declaration("dyn EmailService => SmtpEmailService @ smtp").unwrap();
        
        match provider.service_type {
            ProviderType::Named(service_name, name) => {
                assert_eq!(service_name, "SmtpEmailService");
                assert_eq!(name, "smtp");
            }
            _ => panic!("Expected named provider"),
        }
    }
    
    #[test]
    fn test_lifetime_conventions_applied() {
        let mut configurator = ProviderConfigurator::new(IocContainer::new());
        
        // Create test module with providers
        let module = CompileTimeModuleMetadata::new("TestModule".to_string())
            .with_providers(vec![
                "UserService".to_string(),
                "UserRepository".to_string(),
                "PaymentFactory".to_string(),
            ]);
        
        configurator.configure_from_modules(&[module]).unwrap();
        
        // Check that lifetime conventions were applied
        assert_eq!(configurator.providers.len(), 3);
        
        for provider in &configurator.providers {
            match &provider.service_type {
                ProviderType::Concrete(name) => {
                    let expected_lifetime = if name.ends_with("Service") {
                        ServiceLifetime::Singleton
                    } else if name.ends_with("Repository") {
                        ServiceLifetime::Scoped
                    } else if name.ends_with("Factory") {
                        ServiceLifetime::Transient
                    } else {
                        ServiceLifetime::Transient
                    };
                    assert_eq!(provider.lifetime, Some(expected_lifetime));
                }
                _ => {}
            }
        }
    }
    
    #[test]
    fn test_missing_dependency_detection() {
        let mut configurator = ProviderConfigurator::new(IocContainer::new());
        
        // Add provider with dependencies
        configurator.providers.push(
            ProviderMetadata::concrete("UserController".to_string())
                .with_dependency("UserService".to_string())
                .with_dependency("MissingService".to_string()),
        );
        
        configurator.providers.push(
            ProviderMetadata::concrete("UserService".to_string()),
        );
        
        let result = configurator.validate_dependencies();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            DependencyError::MissingDependency { service, dependency } => {
                assert_eq!(service, "UserController");
                assert_eq!(dependency, "MissingService");
            }
            _ => panic!("Expected MissingDependency error"),
        }
    }
    
    #[test]
    fn test_lifetime_conflict_detection() {
        let mut configurator = ProviderConfigurator::new(IocContainer::new());
        
        // Scoped service depending on Transient service (should fail)
        configurator.providers.push(
            ProviderMetadata::concrete("ScopedService".to_string())
                .with_lifetime(ServiceLifetime::Scoped)
                .with_dependency("TransientService".to_string()),
        );
        
        configurator.providers.push(
            ProviderMetadata::concrete("TransientService".to_string())
                .with_lifetime(ServiceLifetime::Transient),
        );
        
        let result = configurator.validate_dependencies();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            DependencyError::LifetimeConflict { service, dependency, .. } => {
                assert_eq!(service, "ScopedService");
                assert_eq!(dependency, "TransientService");
            }
            _ => panic!("Expected LifetimeConflict error"),
        }
    }
    
    #[test]
    fn test_circular_dependency_detection() {
        let mut configurator = ProviderConfigurator::new(IocContainer::new());
        
        // Create circular dependency: A -> B -> C -> A
        configurator.providers.push(
            ProviderMetadata::concrete("ServiceA".to_string())
                .with_dependency("ServiceB".to_string()),
        );
        
        configurator.providers.push(
            ProviderMetadata::concrete("ServiceB".to_string())
                .with_dependency("ServiceC".to_string()),
        );
        
        configurator.providers.push(
            ProviderMetadata::concrete("ServiceC".to_string())
                .with_dependency("ServiceA".to_string()),
        );
        
        let result = configurator.validate_dependencies();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            DependencyError::CircularDependency { cycle } => {
                assert!(cycle.contains("ServiceA"));
                assert!(cycle.contains("ServiceB"));
                assert!(cycle.contains("ServiceC"));
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }
}