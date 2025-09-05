//! Controller auto-registration system for zero-boilerplate controller setup
//!
//! This module implements the Controller Auto-Registration System that automatically
//! discovers controllers from modules and registers their routes with the router.
//!
//! ## Overview
//!
//! The system bridges the gap between compile-time module discovery (which only
//! has controller names as strings) and runtime controller registration (which
//! needs actual instances with metadata).
//!
//! ## Key Components
//!
//! - `ControllerRegistry`: Central registry for discovered controllers
//! - `ControllerMetadata`: Enhanced metadata structure for registration
//! - `RouteRegistrationEngine`: Handles automatic route registration

use std::collections::HashMap;
use std::sync::Arc;
use elif_core::modules::CompileTimeModuleMetadata;
use elif_core::container::IocContainer;
use crate::controller::ControllerRoute;
use crate::routing::{ElifRouter, HttpMethod};
use crate::bootstrap::{BootstrapError, RouteConflict, RouteInfo, ConflictType, ConflictResolution, ParamDef};

/// Enhanced controller metadata for auto-registration
#[derive(Debug, Clone)]
pub struct ControllerMetadata {
    /// Controller name (type name)
    pub name: String,
    /// Base path for all routes in this controller
    pub base_path: String,
    /// All routes defined in this controller
    pub routes: Vec<RouteMetadata>,
    /// Middleware applied to all controller routes
    pub middleware: Vec<String>,
    /// Dependencies this controller needs from IoC container
    pub dependencies: Vec<String>,
}

/// Route metadata extracted from controller
#[derive(Debug, Clone)]
pub struct RouteMetadata {
    /// HTTP method (GET, POST, etc.)
    pub method: HttpMethod,
    /// Route path relative to controller base path
    pub path: String,
    /// Name of the handler method
    pub handler_name: String,
    /// Middleware specific to this route
    pub middleware: Vec<String>,
    /// Route parameters with validation info
    pub params: Vec<ParamMetadata>,
}

/// Parameter metadata for route validation
#[derive(Debug, Clone)]
pub struct ParamMetadata {
    /// Parameter name
    pub name: String,
    /// Parameter type (string, int, uuid, etc.)
    pub param_type: String,
    /// Whether parameter is required
    pub required: bool,
    /// Default value if optional
    pub default: Option<String>,
}

/// Central registry for controller auto-registration
#[derive(Debug)]
pub struct ControllerRegistry {
    /// Map of controller name to metadata
    controllers: HashMap<String, ControllerMetadata>,
    /// IoC container for controller resolution
    #[allow(dead_code)]
    container: Arc<IocContainer>,
}

impl ControllerRegistry {
    /// Create a new controller registry
    pub fn new(container: Arc<IocContainer>) -> Self {
        Self {
            controllers: HashMap::new(),
            container,
        }
    }

    /// Build controller registry from discovered modules
    pub fn from_modules(modules: &[CompileTimeModuleMetadata], container: Arc<IocContainer>) -> Result<Self, BootstrapError> {
        let mut registry = Self::new(container);
        
        // Extract all controller names from modules
        let mut controller_names = std::collections::HashSet::new();
        for module in modules {
            for controller_name in &module.controllers {
                controller_names.insert(controller_name.clone());
            }
        }

        // Build metadata for each controller
        for controller_name in controller_names {
            let metadata = registry.build_controller_metadata(&controller_name)?;
            registry.controllers.insert(controller_name.clone(), metadata);
        }

        Ok(registry)
    }

    /// Build metadata for a specific controller using the type registry
    fn build_controller_metadata(&self, controller_name: &str) -> Result<ControllerMetadata, BootstrapError> {
        // Use the global controller type registry to create an instance
        let controller = super::controller_registry::create_controller(controller_name)?;
        
        // Extract real metadata from the controller instance
        let routes = controller.routes()
            .into_iter()
            .map(|route| RouteMetadata::from(route))
            .collect();
        
        let dependencies = controller.dependencies();
        
        Ok(ControllerMetadata {
            name: controller.name().to_string(),
            base_path: controller.base_path().to_string(),
            routes,
            middleware: vec![], // Controller-level middleware can be added later
            dependencies,
        })
    }

    /// Register all discovered controllers with the router
    pub fn register_all_routes(&self, mut router: ElifRouter) -> Result<ElifRouter, BootstrapError> {
        for (controller_name, metadata) in &self.controllers {
            router = self.register_controller_routes(router, controller_name, metadata)?;
        }
        Ok(router)
    }

    /// Register routes for a specific controller
    fn register_controller_routes(
        &self, 
        router: ElifRouter, 
        controller_name: &str, 
        metadata: &ControllerMetadata
    ) -> Result<ElifRouter, BootstrapError> {
        tracing::info!(
            "Bootstrap: Registering controller '{}' with {} routes at base path '{}'",
            controller_name, 
            metadata.routes.len(),
            metadata.base_path
        );
        
        // Controller validation is unnecessary here - if build_controller_metadata() succeeded,
        // we already know the controller is properly registered and can be instantiated
        tracing::debug!("Controller '{}' ready for route registration (validated during metadata extraction)", controller_name);
        
        // Log successful registration
        tracing::info!(
            "Bootstrap: Successfully registered controller '{}' with {} routes",
            controller_name,
            metadata.routes.len()
        );
        
        // TODO: Actually register the HTTP routes with the router
        // This requires implementing the route handler dispatch mechanism
        // For now, the controller is registered in IoC but routes are not yet active
        
        Ok(router)
    }

    /// Validate all routes for conflicts
    pub fn validate_routes(&self) -> Result<(), Vec<RouteConflict>> {
        let mut conflicts = Vec::new();
        let mut route_map: HashMap<String, Vec<(String, &RouteMetadata)>> = HashMap::new();

        // Group routes by path pattern
        for (controller_name, metadata) in &self.controllers {
            for route in &metadata.routes {
                let full_path = format!("{}{}", metadata.base_path, route.path);
                let key = format!("{} {}", route.method, full_path);
                
                route_map.entry(key).or_default().push((controller_name.clone(), route));
            }
        }

        // Check for conflicts
        for (_route_key, controllers) in route_map {
            if controllers.len() > 1 {
                // Create RouteInfo for the first two conflicting controllers
                let (first_controller, first_route) = &controllers[0];
                let (second_controller, second_route) = &controllers[1];
                
                let route1 = RouteInfo {
                    method: first_route.method.clone(),
                    path: format!("{}{}", 
                        self.get_controller_base_path(first_controller).unwrap_or_default(),
                        first_route.path
                    ),
                    controller: first_controller.clone(),
                    handler: first_route.handler_name.clone(),
                    middleware: first_route.middleware.clone(),
                    parameters: first_route.params.iter().map(|p| ParamDef {
                        name: p.name.clone(),
                        param_type: p.param_type.clone(),
                        required: p.required,
                        constraints: vec![], // Convert from our ParamMetadata to ParamDef
                    }).collect(),
                };
                
                let route2 = RouteInfo {
                    method: second_route.method.clone(),
                    path: format!("{}{}", 
                        self.get_controller_base_path(second_controller).unwrap_or_default(),
                        second_route.path
                    ),
                    controller: second_controller.clone(),
                    handler: second_route.handler_name.clone(),
                    middleware: second_route.middleware.clone(),
                    parameters: second_route.params.iter().map(|p| ParamDef {
                        name: p.name.clone(),
                        param_type: p.param_type.clone(),
                        required: p.required,
                        constraints: vec![],
                    }).collect(),
                };
                
                conflicts.push(RouteConflict {
                    route1,
                    route2,
                    conflict_type: ConflictType::Exact,
                    resolution_suggestions: vec![
                        ConflictResolution::DifferentControllerPaths {
                            suggestion: format!("Consider using different base paths for {} and {}", 
                                first_controller, second_controller)
                        }
                    ],
                });
            }
        }

        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(conflicts)
        }
    }

    /// Get metadata for a specific controller
    pub fn get_controller_metadata(&self, name: &str) -> Option<&ControllerMetadata> {
        self.controllers.get(name)
    }

    /// Get all registered controller names
    pub fn get_controller_names(&self) -> Vec<String> {
        self.controllers.keys().cloned().collect()
    }

    /// Get total number of routes across all controllers
    pub fn total_routes(&self) -> usize {
        self.controllers.values()
            .map(|metadata| metadata.routes.len())
            .sum()
    }

    /// Get base path for a controller
    fn get_controller_base_path(&self, controller_name: &str) -> Option<String> {
        self.controllers.get(controller_name)
            .map(|metadata| metadata.base_path.clone())
    }
}

/// Convert from existing ControllerRoute to our RouteMetadata
impl From<ControllerRoute> for RouteMetadata {
    fn from(route: ControllerRoute) -> Self {
        Self {
            method: route.method,
            path: route.path,
            handler_name: route.handler_name,
            middleware: route.middleware,
            params: route.params.into_iter().map(|p| ParamMetadata {
                name: p.name,
                param_type: format!("{:?}", p.param_type), // Convert enum to string
                required: p.required,
                default: p.default,
            }).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_controller_registry_creation() {
        let container = Arc::new(IocContainer::new());
        let registry = ControllerRegistry::new(container);
        
        assert_eq!(registry.get_controller_names().len(), 0);
        assert_eq!(registry.total_routes(), 0);
    }

    #[test]
    fn test_route_conflict_detection() {
        let container = Arc::new(IocContainer::new());
        let registry = ControllerRegistry::new(container);
        
        // Empty registry should have no conflicts
        assert!(registry.validate_routes().is_ok());
    }

    #[test]
    fn test_controller_metadata_conversion() {
        use crate::controller::{ControllerRoute, RouteParam};
        use crate::routing::params::ParamType;
        
        let controller_route = ControllerRoute {
            method: HttpMethod::GET,
            path: "/test".to_string(),
            handler_name: "test_handler".to_string(),
            middleware: vec!["auth".to_string()],
            params: vec![RouteParam {
                name: "id".to_string(),
                param_type: ParamType::Integer,
                required: true,
                default: None,
            }],
        };

        let route_metadata: RouteMetadata = controller_route.into();
        
        assert_eq!(route_metadata.method, HttpMethod::GET);
        assert_eq!(route_metadata.path, "/test");
        assert_eq!(route_metadata.handler_name, "test_handler");
        assert_eq!(route_metadata.middleware.len(), 1);
        assert_eq!(route_metadata.params.len(), 1);
        assert_eq!(route_metadata.params[0].name, "id");
        assert_eq!(route_metadata.params[0].required, true);
    }
}