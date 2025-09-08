//! Automatic Container Configuration
//!
//! This module provides automatic container configuration based on conventions
//! and metadata, integrating with the provider auto-configuration system.

use std::collections::HashMap;
use crate::container::{IocContainer, IocContainerBuilder, ServiceConventions, ServiceLifetime};
use crate::bootstrap::providers::{ProviderConfigurator, ConfigError};
use crate::modules::CompileTimeModuleMetadata;

/// Automatic configuration builder for IoC container
pub struct AutoConfigBuilder {
    builder: IocContainerBuilder,
    conventions: ServiceConventions,
    modules: Vec<CompileTimeModuleMetadata>,
    custom_configurations: Vec<Box<dyn ConfigurationRule>>,
}

impl AutoConfigBuilder {
    /// Create a new auto-configuration builder
    pub fn new() -> Self {
        Self {
            builder: IocContainerBuilder::new(),
            conventions: ServiceConventions::new(),
            modules: Vec::new(),
            custom_configurations: Vec::new(),
        }
    }
    
    /// Create auto-configuration builder with existing IoC builder
    pub fn with_builder(builder: IocContainerBuilder) -> Self {
        Self {
            builder,
            conventions: ServiceConventions::new(),
            modules: Vec::new(),
            custom_configurations: Vec::new(),
        }
    }
    
    /// Set custom service conventions
    pub fn with_conventions(mut self, conventions: ServiceConventions) -> Self {
        self.conventions = conventions;
        self
    }
    
    /// Add modules to configure
    pub fn with_modules(mut self, modules: Vec<CompileTimeModuleMetadata>) -> Self {
        self.modules = modules;
        self
    }
    
    /// Add a single module to configure
    pub fn add_module(mut self, module: CompileTimeModuleMetadata) -> Self {
        self.modules.push(module);
        self
    }
    
    /// Add a custom configuration rule
    pub fn add_configuration_rule<R: ConfigurationRule + 'static>(mut self, rule: R) -> Self {
        self.custom_configurations.push(Box::new(rule));
        self
    }
    
    /// Build the container with automatic configuration
    pub fn build(self) -> Result<IocContainer, ConfigError> {
        // Build basic container
        let container = self.builder.build()
            .map_err(|e| ConfigError::ContainerError(e))?;
        
        // Create provider configurator
        let mut configurator = ProviderConfigurator::with_conventions(container, self.conventions);
        
        // Configure from modules
        configurator.configure_from_modules(&self.modules)?;
        
        // Apply custom configuration rules
        for rule in &self.custom_configurations {
            rule.apply(&mut configurator)?;
        }
        
        Ok(configurator.into_container())
    }
}

impl Default for AutoConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for custom configuration rules
pub trait ConfigurationRule: Send + Sync {
    /// Apply the configuration rule to the provider configurator
    fn apply(&self, configurator: &mut ProviderConfigurator) -> Result<(), ConfigError>;
    
    /// Get a description of what this rule does
    fn description(&self) -> &'static str {
        "Custom configuration rule"
    }
    
    /// Get the priority of this rule (higher numbers run first)
    fn priority(&self) -> u32 {
        100
    }
}

/// Container configuration utilities
pub struct ContainerAutoConfig;

impl ContainerAutoConfig {
    /// Create a fully auto-configured container from modules
    pub fn from_modules(modules: Vec<CompileTimeModuleMetadata>) -> Result<IocContainer, ConfigError> {
        AutoConfigBuilder::new()
            .with_modules(modules)
            .build()
    }
    
    /// Create a fully auto-configured container from modules with custom conventions
    pub fn from_modules_with_conventions(
        modules: Vec<CompileTimeModuleMetadata>,
        conventions: ServiceConventions,
    ) -> Result<IocContainer, ConfigError> {
        AutoConfigBuilder::new()
            .with_modules(modules)
            .with_conventions(conventions)
            .build()
    }
    
    /// Validate module configuration without building container
    pub fn validate_modules(modules: &[CompileTimeModuleMetadata]) -> Result<ValidationReport, ConfigError> {
        let container = IocContainer::new();
        let mut configurator = ProviderConfigurator::new(container);
        
        // Extract providers and validate
        configurator.configure_from_modules(modules)?;
        
        Ok(ValidationReport::from_configurator(&configurator))
    }
}

/// Validation report for container configuration
#[derive(Debug)]
pub struct ValidationReport {
    pub total_providers: usize,
    pub providers_by_lifetime: HashMap<ServiceLifetime, usize>,
    pub dependency_graph_depth: usize,
    pub potential_issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Create validation report from provider configurator
    pub fn from_configurator(configurator: &ProviderConfigurator) -> Self {
        let mut providers_by_lifetime = HashMap::new();

        // This assumes a `providers()` method is added to `ProviderConfigurator`
        for provider in configurator.providers() {
            let lifetime = provider.lifetime.unwrap_or(ServiceLifetime::Transient);
            *providers_by_lifetime.entry(lifetime).or_default() += 1;
        }

        // For now, create a partial report
        // TODO: Calculate dependency_graph_depth and potential_issues
        Self {
            total_providers: configurator.providers().len(),
            providers_by_lifetime,
            dependency_graph_depth: 0,
            potential_issues: Vec::new(),
        }
    }
    
    /// Check if the configuration has any issues
    pub fn has_issues(&self) -> bool {
        !self.potential_issues.is_empty()
    }
    
    /// Get summary statistics
    pub fn summary(&self) -> String {
        format!(
            "Configuration Summary:\n\
            - Total Providers: {}\n\
            - Singleton Services: {}\n\
            - Scoped Services: {}\n\
            - Transient Services: {}\n\
            - Dependency Depth: {}\n\
            - Issues Found: {}",
            self.total_providers,
            self.providers_by_lifetime.get(&ServiceLifetime::Singleton).unwrap_or(&0),
            self.providers_by_lifetime.get(&ServiceLifetime::Scoped).unwrap_or(&0),
            self.providers_by_lifetime.get(&ServiceLifetime::Transient).unwrap_or(&0),
            self.dependency_graph_depth,
            self.potential_issues.len()
        )
    }
}

/// Validation issue types
#[derive(Debug, Clone)]
pub enum ValidationIssue {
    /// Service has too many dependencies
    TooManyDependencies {
        service: String,
        count: usize,
        recommended_max: usize,
    },
    /// Deep dependency chain
    DeepDependencyChain {
        service: String,
        depth: usize,
        recommended_max: usize,
    },
    /// Potential performance concern
    PerformanceConcern {
        service: String,
        issue: String,
        suggestion: String,
    },
    /// Convention violation
    ConventionViolation {
        service: String,
        expected: String,
        actual: String,
    },
}

/// Pre-built configuration rules
#[allow(dead_code)]
pub mod rules {
    use super::*;
    
    /// Rule that validates service dependency counts
    pub struct DependencyCountRule {
        max_dependencies: usize,
    }
    
    impl DependencyCountRule {
        pub fn new(max_dependencies: usize) -> Self {
            Self { max_dependencies }
        }
    }
    
    impl ConfigurationRule for DependencyCountRule {
        fn apply(&self, _configurator: &mut ProviderConfigurator) -> Result<(), ConfigError> {
            // This would validate dependency counts and add warnings
            // For now, just a placeholder
            Ok(())
        }
        
        fn description(&self) -> &'static str {
            "Validates that services don't have too many dependencies"
        }
        
        fn priority(&self) -> u32 {
            200
        }
    }
    
    /// Rule that enforces naming conventions more strictly
    pub struct NamingConventionRule {
        strict_mode: bool,
    }
    
    impl NamingConventionRule {
        pub fn new(strict_mode: bool) -> Self {
            Self { strict_mode }
        }
    }
    
    impl ConfigurationRule for NamingConventionRule {
        fn apply(&self, _configurator: &mut ProviderConfigurator) -> Result<(), ConfigError> {
            // This would enforce naming conventions more strictly
            // For now, just a placeholder
            Ok(())
        }
        
        fn description(&self) -> &'static str {
            "Enforces strict naming conventions for services"
        }
        
        fn priority(&self) -> u32 {
            150
        }
    }
    
    /// Rule that optimizes service lifetimes based on usage patterns
    pub struct LifetimeOptimizationRule;
    
    impl ConfigurationRule for LifetimeOptimizationRule {
        fn apply(&self, _configurator: &mut ProviderConfigurator) -> Result<(), ConfigError> {
            // This would analyze usage patterns and suggest lifetime optimizations
            // For now, just a placeholder
            Ok(())
        }
        
        fn description(&self) -> &'static str {
            "Optimizes service lifetimes based on dependency patterns"
        }
        
        fn priority(&self) -> u32 {
            50
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::CompileTimeModuleMetadata;
    
    #[test]
    fn test_auto_config_builder() {
        let module = CompileTimeModuleMetadata::new("TestModule".to_string())
            .with_providers(vec!["UserService".to_string(), "UserRepository".to_string()]);
        
        let result = AutoConfigBuilder::new()
            .add_module(module)
            .build();
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_container_auto_config_from_modules() {
        let modules = vec![
            CompileTimeModuleMetadata::new("UserModule".to_string())
                .with_providers(vec!["UserService".to_string()]),
            CompileTimeModuleMetadata::new("AuthModule".to_string())
                .with_providers(vec!["AuthService".to_string()]),
        ];
        
        let result = ContainerAutoConfig::from_modules(modules);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validation_report() {
        let modules = vec![
            CompileTimeModuleMetadata::new("TestModule".to_string())
                .with_providers(vec!["TestService".to_string()]),
        ];
        
        let result = ContainerAutoConfig::validate_modules(&modules);
        assert!(result.is_ok());
        
        let report = result.unwrap();
        assert!(!report.has_issues()); // Should be valid
    }
    
    #[test]
    fn test_custom_configuration_rule() {
        struct TestRule;
        
        impl ConfigurationRule for TestRule {
            fn apply(&self, _configurator: &mut ProviderConfigurator) -> Result<(), ConfigError> {
                Ok(())
            }
        }
        
        let module = CompileTimeModuleMetadata::new("TestModule".to_string())
            .with_providers(vec!["TestService".to_string()]);
        
        let result = AutoConfigBuilder::new()
            .add_module(module)
            .add_configuration_rule(TestRule)
            .build();
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validation_report_summary() {
        let report = ValidationReport {
            total_providers: 5,
            providers_by_lifetime: [
                (ServiceLifetime::Singleton, 2),
                (ServiceLifetime::Scoped, 2),
                (ServiceLifetime::Transient, 1),
            ].iter().cloned().collect(),
            dependency_graph_depth: 3,
            potential_issues: vec![],
        };
        
        let summary = report.summary();
        assert!(summary.contains("Total Providers: 5"));
        assert!(summary.contains("Singleton Services: 2"));
        assert!(summary.contains("Scoped Services: 2"));
        assert!(summary.contains("Transient Services: 1"));
    }
    
    #[test]
    fn test_configuration_rules() {
        use rules::*;
        
        // Test that rules can be created and have correct properties
        let dep_rule = DependencyCountRule::new(10);
        assert_eq!(dep_rule.priority(), 200);
        assert!(dep_rule.description().contains("dependencies"));
        
        let naming_rule = NamingConventionRule::new(true);
        assert_eq!(naming_rule.priority(), 150);
        assert!(naming_rule.description().contains("naming"));
        
        let lifetime_rule = LifetimeOptimizationRule;
        assert_eq!(lifetime_rule.priority(), 50);
        assert!(lifetime_rule.description().contains("lifetime"));
    }
}