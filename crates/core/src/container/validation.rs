use std::collections::{HashMap, HashSet};

use crate::container::descriptor::{ServiceId, ServiceDescriptor};
use crate::container::scope::ServiceScope;
use crate::errors::CoreError;

/// Compile-time validation error types
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Service registration is missing
    MissingRegistration {
        service_type: String,
        required_by: String,
    },
    /// Circular dependency detected
    CircularDependency {
        cycle: Vec<String>,
    },
    /// Lifetime compatibility issue
    LifetimeIncompatibility {
        service: String,
        service_lifetime: ServiceScope,
        dependency: String,
        dependency_lifetime: ServiceScope,
    },
    /// Interface without implementation
    UnboundInterface {
        interface: String,
    },
    /// Multiple default implementations
    MultipleDefaults {
        interface: String,
        implementations: Vec<String>,
    },
    /// Invalid factory signature
    InvalidFactory {
        service: String,
        error: String,
    },
    /// Missing auto-wire information
    MissingAutoWire {
        service: String,
    },
}

impl ValidationError {
    /// Convert to CoreError for runtime use
    pub fn to_core_error(&self) -> CoreError {
        match self {
            ValidationError::MissingRegistration { service_type, required_by } => {
                CoreError::ServiceNotFound {
                    service_type: format!("{} (required by {})", service_type, required_by),
                }
            },
            ValidationError::CircularDependency { cycle } => {
                CoreError::InvalidServiceDescriptor {
                    message: format!("Circular dependency detected: {}", cycle.join(" -> ")),
                }
            },
            ValidationError::LifetimeIncompatibility { service, service_lifetime, dependency, dependency_lifetime } => {
                CoreError::InvalidServiceDescriptor {
                    message: format!(
                        "Lifetime incompatibility: {} ({:?}) depends on {} ({:?})", 
                        service, service_lifetime, dependency, dependency_lifetime
                    ),
                }
            },
            ValidationError::UnboundInterface { interface } => {
                CoreError::ServiceNotFound {
                    service_type: format!("No implementation bound for interface {}", interface),
                }
            },
            ValidationError::MultipleDefaults { interface, implementations } => {
                CoreError::InvalidServiceDescriptor {
                    message: format!(
                        "Multiple default implementations for {}: {}", 
                        interface, implementations.join(", ")
                    ),
                }
            },
            ValidationError::InvalidFactory { service, error } => {
                CoreError::InvalidServiceDescriptor {
                    message: format!("Invalid factory for {}: {}", service, error),
                }
            },
            ValidationError::MissingAutoWire { service } => {
                CoreError::InvalidServiceDescriptor {
                    message: format!("Service {} is marked for auto-wiring but no constructor info available", service),
                }
            },
        }
    }
}

/// Compile-time dependency validator
#[derive(Debug)]
pub struct DependencyValidator {
    dependency_graph: HashMap<ServiceId, Vec<ServiceId>>,
    interface_bindings: HashMap<String, Vec<ServiceId>>,
    service_ids: Vec<ServiceId>,
}

impl DependencyValidator {
    /// Create a new validator from service descriptors
    pub fn new(descriptors: &[ServiceDescriptor]) -> Self {
        let mut dependency_graph = HashMap::new();
        let mut interface_bindings: HashMap<String, Vec<ServiceId>> = HashMap::new();
        
        // Build dependency graph and interface bindings
        for descriptor in descriptors {
            dependency_graph.insert(
                descriptor.service_id.clone(), 
                descriptor.dependencies.clone()
            );
            
            // Track interface bindings (simplified - would need more metadata in real implementation)
            if let Some(interface_name) = descriptor.service_id.name.as_ref() {
                interface_bindings
                    .entry(interface_name.clone())
                    .or_default()
                    .push(descriptor.service_id.clone());
            }
        }
        
        let service_ids = descriptors.iter().map(|d| d.service_id.clone()).collect();
        
        Self {
            dependency_graph,
            interface_bindings,
            service_ids,
        }
    }
    
    /// Validate all dependencies and return any errors
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        // Check for missing registrations
        errors.extend(self.validate_registrations());
        
        // Check for circular dependencies
        errors.extend(self.validate_circular_dependencies());
        
        // Check lifetime compatibility
        errors.extend(self.validate_lifetime_compatibility());
        
        // Check interface bindings
        errors.extend(self.validate_interface_bindings());
        
        // Check for multiple defaults
        errors.extend(self.validate_default_implementations());
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Validate that all dependencies are registered
    fn validate_registrations(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let registered_services: HashSet<&ServiceId> = self.service_ids.iter().collect();
        
        for (service_id, dependencies) in &self.dependency_graph {
            for dependency in dependencies {
                if !registered_services.contains(dependency) {
                    errors.push(ValidationError::MissingRegistration {
                        service_type: dependency.type_name().to_string(),
                        required_by: service_id.type_name().to_string(),
                    });
                }
            }
        }
        
        errors
    }
    
    /// Validate there are no circular dependencies
    fn validate_circular_dependencies(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();
        
        for service_id in self.dependency_graph.keys() {
            if !visited.contains(service_id) {
                if let Some(cycle) = self.detect_cycle(
                    service_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path
                ) {
                    errors.push(ValidationError::CircularDependency { cycle });
                }
            }
        }
        
        errors
    }
    
    /// Detect cycles in dependency graph using DFS
    fn detect_cycle(
        &self,
        service_id: &ServiceId,
        visited: &mut HashSet<ServiceId>,
        rec_stack: &mut HashSet<ServiceId>,
        path: &mut Vec<String>
    ) -> Option<Vec<String>> {
        visited.insert(service_id.clone());
        rec_stack.insert(service_id.clone());
        path.push(service_id.type_name().to_string());
        
        if let Some(dependencies) = self.dependency_graph.get(service_id) {
            for dependency in dependencies {
                if !visited.contains(dependency) {
                    if let Some(cycle) = self.detect_cycle(dependency, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dependency) {
                    // Found a back edge - cycle detected
                    let cycle_start = path.iter()
                        .position(|name| name == &dependency.type_name())
                        .unwrap_or(0);
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dependency.type_name().to_string());
                    return Some(cycle);
                }
            }
        }
        
        rec_stack.remove(service_id);
        path.pop();
        None
    }
    
    /// Validate lifetime compatibility
    fn validate_lifetime_compatibility(&self) -> Vec<ValidationError> {
        let errors = Vec::new();
        
        // For now, skip lifetime validation as we don't store lifetime info
        // In a full implementation, we'd need to pass lifetime information
        // or store it separately in the validator
        
        errors
    }
    
    /// Check if service lifetime is compatible with dependency lifetime
    #[allow(dead_code)]
    fn is_lifetime_compatible(&self, service: ServiceScope, dependency: ServiceScope) -> bool {
        match (service, dependency) {
            // Singleton can depend on anything
            (ServiceScope::Singleton, _) => true,
            
            // Scoped can depend on Singleton or Scoped, but not Transient
            (ServiceScope::Scoped, ServiceScope::Transient) => false,
            (ServiceScope::Scoped, _) => true,
            
            // Transient can depend on anything
            (ServiceScope::Transient, _) => true,
        }
    }
    
    /// Validate interface bindings
    fn validate_interface_bindings(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        
        // This would be more sophisticated in a real implementation
        // For now, just check if we have interfaces without implementations
        for (interface, implementations) in &self.interface_bindings {
            if implementations.is_empty() {
                errors.push(ValidationError::UnboundInterface {
                    interface: interface.clone(),
                });
            }
        }
        
        errors
    }
    
    /// Validate default implementations
    fn validate_default_implementations(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let defaults_per_interface: HashMap<String, Vec<String>> = HashMap::new();
        
        // This would need additional metadata to track which services are marked as default
        // For now, this is a placeholder
        
        for (interface, implementations) in defaults_per_interface {
            if implementations.len() > 1 {
                errors.push(ValidationError::MultipleDefaults {
                    interface,
                    implementations,
                });
            }
        }
        
        errors
    }
    
    /// Get dependency graph for visualization
    pub fn dependency_graph(&self) -> &HashMap<ServiceId, Vec<ServiceId>> {
        &self.dependency_graph
    }
    
    /// Get topologically sorted services
    pub fn topological_sort(&self) -> Result<Vec<ServiceId>, ValidationError> {
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        let mut result = Vec::new();
        
        for service_id in self.dependency_graph.keys() {
            if !visited.contains(service_id) {
                if let Err(cycle) = self.visit_for_topo_sort(
                    service_id,
                    &mut visited,
                    &mut temp_visited,
                    &mut result
                ) {
                    return Err(cycle);
                }
            }
        }
        
        result.reverse(); // Reverse for correct dependency order
        Ok(result)
    }
    
    /// Visit node for topological sorting
    fn visit_for_topo_sort(
        &self,
        service_id: &ServiceId,
        visited: &mut HashSet<ServiceId>,
        temp_visited: &mut HashSet<ServiceId>,
        result: &mut Vec<ServiceId>
    ) -> Result<(), ValidationError> {
        if temp_visited.contains(service_id) {
            return Err(ValidationError::CircularDependency {
                cycle: vec![service_id.type_name().to_string()], // Simplified
            });
        }
        
        if visited.contains(service_id) {
            return Ok(());
        }
        
        temp_visited.insert(service_id.clone());
        
        if let Some(dependencies) = self.dependency_graph.get(service_id) {
            for dependency in dependencies {
                self.visit_for_topo_sort(dependency, visited, temp_visited, result)?;
            }
        }
        
        temp_visited.remove(service_id);
        visited.insert(service_id.clone());
        result.push(service_id.clone());
        
        Ok(())
    }
}

/// Container validator for runtime validation
#[derive(Debug)]
pub struct ContainerValidator;

impl ContainerValidator {
    /// Create a new container validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate container configuration
    pub fn validate_container(
        &self,
        descriptors: &[ServiceDescriptor]
    ) -> Result<ValidationReport, Vec<ValidationError>> {
        let validator = DependencyValidator::new(&descriptors);
        
        match validator.validate() {
            Ok(()) => {
                let topo_order = validator.topological_sort()
                    .map_err(|e| vec![e])?;
                
                Ok(ValidationReport {
                    is_valid: true,
                    service_count: descriptors.len(),
                    dependency_count: validator.dependency_graph.values()
                        .map(|deps| deps.len())
                        .sum(),
                    resolution_order: topo_order,
                    errors: vec![],
                    warnings: vec![],
                })
            },
            Err(errors) => {
                Err(errors)
            }
        }
    }
    
    /// Validate and provide warnings for potential issues
    pub fn validate_with_warnings(
        &self,
        descriptors: &[ServiceDescriptor]
    ) -> ValidationReport {
        let validator = DependencyValidator::new(&descriptors);
        let mut warnings = Vec::new();
        
        match validator.validate() {
            Ok(()) => {
                // Check for potential warnings
                warnings.extend(self.check_performance_warnings(descriptors));
                warnings.extend(self.check_design_warnings(descriptors));
                
                let topo_order = validator.topological_sort()
                    .unwrap_or_else(|_| vec![]);
                
                ValidationReport {
                    is_valid: true,
                    service_count: descriptors.len(),
                    dependency_count: validator.dependency_graph.values()
                        .map(|deps| deps.len())
                        .sum(),
                    resolution_order: topo_order,
                    errors: vec![],
                    warnings,
                }
            },
            Err(errors) => {
                ValidationReport {
                    is_valid: false,
                    service_count: descriptors.len(),
                    dependency_count: 0,
                    resolution_order: vec![],
                    errors,
                    warnings,
                }
            }
        }
    }
    
    /// Check for performance-related warnings
    fn check_performance_warnings(&self, descriptors: &[ServiceDescriptor]) -> Vec<ValidationWarning> {
        let mut warnings = Vec::new();
        
        // Check for too many transient services
        let transient_count = descriptors.iter()
            .filter(|d| d.lifetime == ServiceScope::Transient)
            .count();
        
        if transient_count > descriptors.len() / 2 {
            warnings.push(ValidationWarning::PerformanceConcern {
                issue: format!("High number of transient services ({}/{}). Consider using singleton or scoped lifetimes.", transient_count, descriptors.len()),
            });
        }
        
        // Check for deeply nested dependencies
        // TODO: Implement depth checking
        
        warnings
    }
    
    /// Check for design-related warnings
    fn check_design_warnings(&self, descriptors: &[ServiceDescriptor]) -> Vec<ValidationWarning> {
        let mut warnings = Vec::new();
        
        // Check for services with too many dependencies
        for descriptor in descriptors {
            if descriptor.dependencies.len() > 5 {
                warnings.push(ValidationWarning::DesignConcern {
                    service: descriptor.service_id.type_name().to_string(),
                    issue: format!("Service has {} dependencies. Consider breaking it down.", descriptor.dependencies.len()),
                });
            }
        }
        
        warnings
    }
}

impl Default for ContainerValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation warning types
#[derive(Debug, Clone)]
pub enum ValidationWarning {
    /// Performance-related concern
    PerformanceConcern {
        issue: String,
    },
    /// Design-related concern
    DesignConcern {
        service: String,
        issue: String,
    },
    /// Best practice violation
    BestPracticeViolation {
        service: String,
        violation: String,
        suggestion: String,
    },
}

/// Validation report containing results and recommendations
#[derive(Debug)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub service_count: usize,
    pub dependency_count: usize,
    pub resolution_order: Vec<ServiceId>,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationReport {
    /// Generate a human-readable report
    pub fn to_string(&self) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("Container Validation Report\n"));
        report.push_str(&format!("==========================\n\n"));
        
        report.push_str(&format!("Status: {}\n", if self.is_valid { "VALID" } else { "INVALID" }));
        report.push_str(&format!("Services: {}\n", self.service_count));
        report.push_str(&format!("Dependencies: {}\n", self.dependency_count));
        report.push_str(&format!("Resolution Order: {} services\n\n", self.resolution_order.len()));
        
        if !self.errors.is_empty() {
            report.push_str("ERRORS:\n");
            for (i, error) in self.errors.iter().enumerate() {
                report.push_str(&format!("  {}. {:?}\n", i + 1, error));
            }
            report.push('\n');
        }
        
        if !self.warnings.is_empty() {
            report.push_str("WARNINGS:\n");
            for (i, warning) in self.warnings.iter().enumerate() {
                report.push_str(&format!("  {}. {:?}\n", i + 1, warning));
            }
            report.push('\n');
        }
        
        if self.is_valid {
            report.push_str("✅ Container configuration is valid and ready for use.\n");
        } else {
            report.push_str("❌ Container configuration has errors that must be fixed.\n");
        }
        
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::descriptor::{ServiceDescriptor, ServiceActivationStrategy};
    use std::any::{Any, TypeId};

    fn create_test_descriptor(
        type_name: &str, 
        lifetime: ServiceScope, 
        deps: Vec<&str>
    ) -> ServiceDescriptor {
        let service_id = ServiceId {
            type_id: TypeId::of::<()>(),
            type_name: "test_service",
            name: Some(type_name.to_string()),
        };
        
        let dependencies: Vec<ServiceId> = deps.iter().map(|dep| ServiceId {
            type_id: TypeId::of::<()>(),
            type_name: "test_dependency",
            name: Some(dep.to_string()),
        }).collect();
        
        ServiceDescriptor {
            service_id,
            implementation_id: TypeId::of::<()>(),
            lifetime,
            dependencies,
            activation_strategy: ServiceActivationStrategy::Factory(
                Box::new(|| Ok(Box::new(()) as Box<dyn Any + Send + Sync>))
            ),
        }
    }

    #[test]
    fn test_valid_dependencies() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec![]),
            create_test_descriptor("ServiceB", ServiceScope::Singleton, vec!["ServiceA"]),
            create_test_descriptor("ServiceC", ServiceScope::Transient, vec!["ServiceA", "ServiceB"]),
        ];
        
        let validator = DependencyValidator::new(&descriptors);
        let result = validator.validate();
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_dependency() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec!["MissingService"]),
        ];
        
        let validator = DependencyValidator::new(&descriptors);
        let result = validator.validate();
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::MissingRegistration { .. }));
    }

    #[test]
    fn test_circular_dependency() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec!["ServiceB"]),
            create_test_descriptor("ServiceB", ServiceScope::Singleton, vec!["ServiceA"]),
        ];
        
        let validator = DependencyValidator::new(&descriptors);
        let result = validator.validate();
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::CircularDependency { .. })));
    }

    #[test]
    fn test_lifetime_incompatibility() {
        let descriptors = vec![
            create_test_descriptor("SingletonService", ServiceScope::Singleton, vec!["TransientService"]),
            create_test_descriptor("TransientService", ServiceScope::Transient, vec![]),
            create_test_descriptor("ScopedService", ServiceScope::Scoped, vec!["TransientService"]),
        ];
        
        let validator = DependencyValidator::new(&descriptors);
        let result = validator.validate();
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Scoped depending on Transient should be an error
        assert!(errors.iter().any(|e| matches!(e, ValidationError::LifetimeIncompatibility { .. })));
    }

    #[test]
    fn test_topological_sort() {
        let descriptors = vec![
            create_test_descriptor("ServiceC", ServiceScope::Singleton, vec!["ServiceA", "ServiceB"]),
            create_test_descriptor("ServiceB", ServiceScope::Singleton, vec!["ServiceA"]),
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec![]),
        ];
        
        let validator = DependencyValidator::new(&descriptors);
        let topo_order = validator.topological_sort().unwrap();
        
        // ServiceA should come first (no dependencies)
        // ServiceB should come second (depends on A)
        // ServiceC should come last (depends on A and B)
        let names: Vec<String> = topo_order.iter()
            .map(|id| id.name.as_ref().unwrap().clone())
            .collect();
        
        assert_eq!(names[0], "ServiceA");
        assert_eq!(names[1], "ServiceB");
        assert_eq!(names[2], "ServiceC");
    }

    #[test]
    fn test_container_validator() {
        let descriptors = vec![
            create_test_descriptor("ServiceA", ServiceScope::Singleton, vec![]),
            create_test_descriptor("ServiceB", ServiceScope::Transient, vec!["ServiceA"]),
        ];
        
        let validator = ContainerValidator::new();
        let report = validator.validate_with_warnings(&descriptors);
        
        assert!(report.is_valid);
        assert_eq!(report.service_count, 2);
        assert_eq!(report.resolution_order.len(), 2);
    }
}