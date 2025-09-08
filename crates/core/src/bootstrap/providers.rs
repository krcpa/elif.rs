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
    Trait {
        trait_name: String,
        impl_name: String,
    },
    /// Named trait implementation with explicit naming
    NamedTrait {
        trait_name: String,
        impl_name: String,
        name: String,
    },
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
            service_type: ProviderType::Trait {
                trait_name,
                impl_name,
            },
            lifetime: None,
            dependencies: Vec::new(),
        }
    }
    
    /// Create new provider metadata for named trait implementation
    pub fn named_trait(trait_name: String, impl_name: String, name: String) -> Self {
        Self {
            service_type: ProviderType::NamedTrait {
                trait_name,
                impl_name,
                name,
            },
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
        // - "dyn EmailService => SmtpEmailService @ smtp" -> Named trait service
        
        if let Some(arrow_pos) = provider_str.find(" => ") {
            // Trait mapping or named trait service
            let trait_part = provider_str[..arrow_pos].trim();
            let impl_part = provider_str[arrow_pos + 4..].trim();
            
            // Remove "dyn " prefix if present
            let trait_name = if let Some(stripped) = trait_part.strip_prefix("dyn ") {
                stripped.trim().to_string()
            } else {
                trait_part.to_string()
            };
            
            // Check for named trait service (@ symbol)
            if let Some(at_pos) = impl_part.find(" @ ") {
                let impl_name = impl_part[..at_pos].trim().to_string();
                let name = impl_part[at_pos + 3..].trim().to_string();
                Ok(ProviderMetadata::named_trait(trait_name, impl_name, name))
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
                    ProviderType::Trait { impl_name, .. } => impl_name,
                    ProviderType::NamedTrait { impl_name, .. } => impl_name,
                };
                
                let lifetime = self.conventions.get_lifetime_for_type(service_name);
                provider.lifetime = Some(lifetime);
            }
        }
        Ok(())
    }
    
    /// Validate all dependencies can be resolved
    pub fn validate_dependencies(&self) -> Result<(), DependencyError> {
        // Build service registry with proper keys for different provider types
        let mut services: HashMap<String, &ProviderMetadata> = HashMap::new();
        let mut named_services: HashMap<String, &ProviderMetadata> = HashMap::new();
        
        // Register services by their resolution keys
        for provider in &self.providers {
            match &provider.service_type {
                ProviderType::Concrete(name) => {
                    services.insert(name.clone(), provider);
                }
                ProviderType::Trait { trait_name, .. } => {
                    services.insert(trait_name.clone(), provider);
                }
                ProviderType::NamedTrait { trait_name, name, .. } => {
                    // Named services can be resolved both by trait name and by qualified name
                    let qualified_name = format!("{}@{}", trait_name, name);
                    services.insert(qualified_name.clone(), provider);
                    named_services.insert(name.clone(), provider);
                    
                    // Also make available by trait name if no other implementation exists
                    if !services.contains_key(trait_name) {
                        services.insert(trait_name.clone(), provider);
                    }
                }
            }
        }
        
        // Check each service's dependencies
        for provider in &self.providers {
            let service_key = self.get_service_key(&provider.service_type);
            
            for dependency in &provider.dependencies {
                // Try to resolve dependency in multiple ways:
                // 1. Direct service name
                // 2. Named service lookup
                // 3. Qualified name lookup
                let dep_provider = services.get(dependency)
                    .or_else(|| named_services.get(dependency))
                    .or_else(|| {
                        // Try to find by qualified name pattern (trait@name)
                        if dependency.contains('@') {
                            services.get(dependency)
                        } else {
                            None
                        }
                    });
                
                if let Some(dep_provider) = dep_provider {
                    // Check lifetime compatibility
                    self.validate_lifetime_compatibility(provider, dep_provider, &service_key, dependency)?;
                } else {
                    return Err(DependencyError::MissingDependency {
                        service: service_key,
                        dependency: dependency.clone(),
                    });
                }
            }
        }
        
        // Check for circular dependencies
        self.detect_circular_dependencies(&services)?;
        
        Ok(())
    }
    
    /// Get the service key for dependency resolution
    fn get_service_key(&self, provider_type: &ProviderType) -> String {
        match provider_type {
            ProviderType::Concrete(name) => name.clone(),
            ProviderType::Trait { trait_name, .. } => trait_name.clone(),
            ProviderType::NamedTrait { trait_name, name, .. } => {
                format!("{}@{}", trait_name, name)
            }
        }
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
                ProviderType::Trait { trait_name, impl_name } => {
                    self.register_trait_service(trait_name, impl_name, lifetime)?;
                }
                ProviderType::NamedTrait { trait_name, impl_name, name } => {
                    self.register_named_trait_service(trait_name, impl_name, name, lifetime)?;
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
    
    /// Register a named trait service (placeholder implementation)
    fn register_named_trait_service(
        &mut self,
        _trait_name: &str,
        _impl_name: &str,
        _name: &str,
        _lifetime: ServiceLifetime,
    ) -> Result<(), ConfigError> {
        // TODO: Implement actual named trait service registration using codegen or reflection
        // For now, this is a placeholder that would be filled in with actual
        // container.bind_named_trait::<TraitType, ImplType>(name) calls
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
            ProviderType::Trait { trait_name, impl_name } => {
                assert_eq!(trait_name, "EmailService");
                assert_eq!(impl_name, "SmtpEmailService");
            }
            _ => panic!("Expected trait provider"),
        }
    }
    
    #[test]
    fn test_parse_named_trait_provider() {
        let configurator = ProviderConfigurator::new(IocContainer::new());
        let provider = configurator.parse_provider_declaration("dyn EmailService => SmtpEmailService @ smtp").unwrap();
        
        match provider.service_type {
            ProviderType::NamedTrait { trait_name, impl_name, name } => {
                assert_eq!(trait_name, "EmailService");
                assert_eq!(impl_name, "SmtpEmailService");
                assert_eq!(name, "smtp");
            }
            _ => panic!("Expected named trait provider"),
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
                ProviderType::Trait { impl_name, .. } => {
                    let expected_lifetime = if impl_name.ends_with("Service") {
                        ServiceLifetime::Singleton
                    } else if impl_name.ends_with("Repository") {
                        ServiceLifetime::Scoped
                    } else if impl_name.ends_with("Factory") {
                        ServiceLifetime::Transient
                    } else {
                        ServiceLifetime::Transient
                    };
                    assert_eq!(provider.lifetime, Some(expected_lifetime));
                }
                ProviderType::NamedTrait { impl_name, .. } => {
                    let expected_lifetime = if impl_name.ends_with("Service") {
                        ServiceLifetime::Singleton
                    } else if impl_name.ends_with("Repository") {
                        ServiceLifetime::Scoped
                    } else if impl_name.ends_with("Factory") {
                        ServiceLifetime::Transient
                    } else {
                        ServiceLifetime::Transient
                    };
                    assert_eq!(provider.lifetime, Some(expected_lifetime));
                }
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
    
    #[test]
    fn test_named_trait_dependency_resolution() {
        let mut configurator = ProviderConfigurator::new(IocContainer::new());
        
        // Create a named trait provider and a service that depends on it
        configurator.providers.push(
            ProviderMetadata::named_trait(
                "EmailService".to_string(),
                "SmtpEmailService".to_string(),
                "smtp".to_string()
            )
        );
        
        // UserController depends on EmailService@smtp (qualified name)
        configurator.providers.push(
            ProviderMetadata::concrete("UserController".to_string())
                .with_dependency("EmailService@smtp".to_string()),
        );
        
        // NotificationService depends on EmailService (should resolve to the named trait)
        configurator.providers.push(
            ProviderMetadata::concrete("NotificationService".to_string())
                .with_dependency("EmailService".to_string()),
        );
        
        let result = configurator.validate_dependencies();
        assert!(result.is_ok(), "Dependency validation should succeed for named traits");
    }
    
    #[test]
    fn test_multiple_named_trait_implementations() {
        let mut configurator = ProviderConfigurator::new(IocContainer::new());
        
        // Create multiple named implementations of the same trait
        configurator.providers.push(
            ProviderMetadata::named_trait(
                "EmailService".to_string(),
                "SmtpEmailService".to_string(),
                "smtp".to_string()
            )
        );
        
        configurator.providers.push(
            ProviderMetadata::named_trait(
                "EmailService".to_string(),
                "SendGridEmailService".to_string(),
                "sendgrid".to_string()
            )
        );
        
        // Controller depending on specific named implementation
        configurator.providers.push(
            ProviderMetadata::concrete("UserController".to_string())
                .with_dependency("EmailService@smtp".to_string()),
        );
        
        // Another controller depending on different implementation
        configurator.providers.push(
            ProviderMetadata::concrete("AdminController".to_string())
                .with_dependency("EmailService@sendgrid".to_string()),
        );
        
        let result = configurator.validate_dependencies();
        assert!(result.is_ok(), "Multiple named trait implementations should resolve correctly");
    }
    
    #[test]
    fn test_service_key_generation() {
        let configurator = ProviderConfigurator::new(IocContainer::new());
        
        // Test concrete service key
        let concrete_key = configurator.get_service_key(&ProviderType::Concrete("UserService".to_string()));
        assert_eq!(concrete_key, "UserService");
        
        // Test trait service key
        let trait_key = configurator.get_service_key(&ProviderType::Trait {
            trait_name: "EmailService".to_string(),
            impl_name: "SmtpEmailService".to_string(),
        });
        assert_eq!(trait_key, "EmailService");
        
        // Test named trait service key
        let named_key = configurator.get_service_key(&ProviderType::NamedTrait {
            trait_name: "EmailService".to_string(),
            impl_name: "SmtpEmailService".to_string(),
            name: "smtp".to_string(),
        });
        assert_eq!(named_key, "EmailService@smtp");
    }
}