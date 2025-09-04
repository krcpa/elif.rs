//! Route conflict detection and validation system for bootstrap integration
//!
//! This module provides comprehensive route conflict detection during application startup,
//! building on the core route matching capabilities with enhanced diagnostics and
//! conflict resolution suggestions.

use crate::{
    bootstrap::BootstrapError,
    routing::{HttpMethod, RouteMatchError, RouteMatcher, RouteDefinition},
};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during route validation
#[derive(Error, Debug)]
pub enum RouteValidationError {
    #[error("Route conflict detected")]
    ConflictDetected {
        conflicts: Vec<RouteConflict>,
    },
    #[error("Parameter type conflict in route {route}: {details}")]
    ParameterConflict {
        route: String,
        details: String,
    },
    #[error("Invalid route configuration: {message}")]
    InvalidConfiguration {
        message: String,
    },
    #[error("Route validation failed: {0}")]
    ValidationFailed(#[from] RouteMatchError),
}

/// Detailed information about a route conflict
#[derive(Debug, Clone)]
pub struct RouteConflict {
    pub route1: RouteInfo,
    pub route2: RouteInfo,
    pub conflict_type: ConflictType,
    pub resolution_suggestions: Vec<ConflictResolution>,
}

/// Information about a conflicting route
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub method: HttpMethod,
    pub path: String,
    pub controller: String,
    pub handler: String,
    pub middleware: Vec<String>,
    pub parameters: Vec<ParamDef>,
}

/// Types of route conflicts
#[derive(Debug, Clone)]
pub enum ConflictType {
    /// Exact path conflict (same method + path)
    Exact,
    /// Parameter type mismatch for same path pattern
    ParameterMismatch,
    /// Ambiguous route patterns that could match same request
    Ambiguous,
    /// Middleware incompatibility
    MiddlewareIncompatible,
}

/// Parameter definition for validation
#[derive(Debug, Clone)]
pub struct ParamDef {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub constraints: Vec<String>,
}

/// Suggested resolutions for route conflicts
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    MergePaths { suggestion: String },
    RenameParameter { from: String, to: String },
    DifferentControllerPaths { suggestion: String },
    MiddlewareConsolidation { suggestion: String },
    UseQueryParameters { suggestion: String },
    ReorderRoutes { suggestion: String },
}

/// Route validator for bootstrap integration
#[derive(Debug)]
pub struct RouteValidator {
    /// All registered routes for validation
    routes: HashMap<RouteKey, RouteRegistration>,
    /// Route matcher for conflict detection
    matcher: RouteMatcher,
    /// Enable detailed diagnostics
    enable_diagnostics: bool,
}

/// Key for identifying unique routes
#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct RouteKey {
    pub method: HttpMethod,
    pub path_pattern: String,
}

/// Registration information for a route
#[derive(Debug, Clone)]
pub struct RouteRegistration {
    pub controller: String,
    pub handler: String,
    pub middleware: Vec<String>,
    pub parameters: Vec<ParamDef>,
    pub definition: RouteDefinition,
}

impl RouteValidator {
    /// Create a new route validator
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            matcher: RouteMatcher::new(),
            enable_diagnostics: true,
        }
    }

    /// Enable or disable detailed diagnostics
    pub fn with_diagnostics(mut self, enable: bool) -> Self {
        self.enable_diagnostics = enable;
        self
    }

    /// Register a route for validation
    pub fn register_route(&mut self, registration: RouteRegistration) -> Result<(), RouteValidationError> {
        let route_key = RouteKey {
            method: registration.definition.method.clone(),
            path_pattern: registration.definition.path.clone(),
        };

        // Check for conflicts before adding
        if let Some(existing) = self.routes.get(&route_key) {
            let conflict = self.analyze_conflict(&registration, existing)?;
            return Err(RouteValidationError::ConflictDetected {
                conflicts: vec![conflict],
            });
        }

        // Add to route matcher for pattern conflict detection
        self.matcher.add_route(registration.definition.clone())
            .map_err(RouteValidationError::ValidationFailed)?;

        // Store registration
        self.routes.insert(route_key, registration);
        
        Ok(())
    }

    /// Validate all registered routes for conflicts
    pub fn validate_all_routes(&self) -> Result<ValidationReport, RouteValidationError> {
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();

        // Check for parameter type conflicts
        self.check_parameter_conflicts(&mut conflicts);

        // Check for middleware incompatibilities
        self.check_middleware_conflicts(&mut warnings);

        // Check for performance issues
        self.check_performance_issues(&mut warnings);

        if !conflicts.is_empty() {
            return Err(RouteValidationError::ConflictDetected { conflicts });
        }

        Ok(ValidationReport {
            total_routes: self.routes.len(),
            conflicts: conflicts.len(),
            warnings: warnings.len(),
            performance_score: self.calculate_performance_score(),
            suggestions: self.generate_optimization_suggestions(),
        })
    }

    /// Generate detailed conflict report for diagnostics
    pub fn generate_conflict_report(&self, conflicts: &[RouteConflict]) -> String {
        let mut report = String::new();
        
        for (i, conflict) in conflicts.iter().enumerate() {
            if i > 0 {
                report.push_str("\n\n");
            }
            
            match conflict.conflict_type {
                ConflictType::Exact => {
                    report.push_str(&format!(
                        "Error: Duplicate route definition detected\n\n\
                         Route: {} {}\n\
                         Defined in:\n\
                         1. {}::{}\n\
                         2. {}::{}\n\n\
                         Resolution suggestions:",
                        conflict.route1.method.as_str(),
                        conflict.route1.path,
                        conflict.route1.controller,
                        conflict.route1.handler,
                        conflict.route2.controller,
                        conflict.route2.handler
                    ));
                }
                ConflictType::ParameterMismatch => {
                    report.push_str(&format!(
                        "Error: Route parameter type conflict\n\n\
                         Route pattern: {} {}\n\
                         Parameter conflicts:\n\
                         • {} expects different types\n\
                         • {} expects different types\n\n\
                         Resolution: Ensure all controllers use the same parameter types",
                        conflict.route1.method.as_str(),
                        conflict.route1.path,
                        conflict.route1.controller,
                        conflict.route2.controller
                    ));
                }
                ConflictType::Ambiguous => {
                    report.push_str(&format!(
                        "Error: Ambiguous route patterns detected\n\n\
                         Routes that could match the same request:\n\
                         1. {} {} ({})\n\
                         2. {} {} ({})\n\n\
                         Problem: These patterns could match the same request\n\n\
                         Resolution: Reorder routes or use more specific patterns",
                        conflict.route1.method.as_str(),
                        conflict.route1.path,
                        conflict.route1.controller,
                        conflict.route2.method.as_str(),
                        conflict.route2.path,
                        conflict.route2.controller
                    ));
                }
                ConflictType::MiddlewareIncompatible => {
                    report.push_str(&format!(
                        "Warning: Middleware incompatibility detected\n\n\
                         Route: {} {}\n\
                         Controllers with different middleware:\n\
                         • {}: {:?}\n\
                         • {}: {:?}\n\n\
                         Resolution: Consider consolidating middleware requirements",
                        conflict.route1.method.as_str(),
                        conflict.route1.path,
                        conflict.route1.controller,
                        conflict.route1.middleware,
                        conflict.route2.controller,
                        conflict.route2.middleware
                    ));
                }
            }

            // Add resolution suggestions
            for (j, suggestion) in conflict.resolution_suggestions.iter().enumerate() {
                report.push_str(&format!("\n  {}. {}", j + 1, self.format_suggestion(suggestion)));
            }
        }

        report
    }

    /// Analyze conflict between two route registrations
    fn analyze_conflict(&self, route1: &RouteRegistration, route2: &RouteRegistration) -> Result<RouteConflict, RouteValidationError> {
        let route_info1 = RouteInfo {
            method: route1.definition.method.clone(),
            path: route1.definition.path.clone(),
            controller: route1.controller.clone(),
            handler: route1.handler.clone(),
            middleware: route1.middleware.clone(),
            parameters: route1.parameters.clone(),
        };

        let route_info2 = RouteInfo {
            method: route2.definition.method.clone(),
            path: route2.definition.path.clone(),
            controller: route2.controller.clone(),
            handler: route2.handler.clone(),
            middleware: route2.middleware.clone(),
            parameters: route2.parameters.clone(),
        };

        let conflict_type = if route1.definition.path == route2.definition.path {
            if self.parameters_conflict(&route1.parameters, &route2.parameters) {
                ConflictType::ParameterMismatch
            } else {
                ConflictType::Exact
            }
        } else {
            ConflictType::Ambiguous
        };

        let resolution_suggestions = self.generate_resolution_suggestions(&route_info1, &route_info2, &conflict_type);

        Ok(RouteConflict {
            route1: route_info1,
            route2: route_info2,
            conflict_type,
            resolution_suggestions,
        })
    }

    /// Check if parameters conflict between routes
    fn parameters_conflict(&self, params1: &[ParamDef], params2: &[ParamDef]) -> bool {
        for param1 in params1 {
            for param2 in params2 {
                if param1.name == param2.name && param1.param_type != param2.param_type {
                    return true;
                }
            }
        }
        false
    }

    /// Check for parameter conflicts across all routes
    fn check_parameter_conflicts(&self, _conflicts: &mut Vec<RouteConflict>) {
        let mut param_types: HashMap<String, (String, String)> = HashMap::new();
        
        for registration in self.routes.values() {
            for param in &registration.parameters {
                let key = format!("{}:{}", registration.definition.path, param.name);
                if let Some((existing_type, _existing_controller)) = param_types.get(&key) {
                    if existing_type != &param.param_type {
                        // Found parameter conflict - would need to create RouteConflict
                        // This is simplified for now
                    }
                } else {
                    param_types.insert(key, (param.param_type.clone(), registration.controller.clone()));
                }
            }
        }
    }

    /// Check for middleware conflicts
    fn check_middleware_conflicts(&self, warnings: &mut Vec<String>) {
        // Group routes by path pattern to check middleware consistency
        let mut path_middleware: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();
        
        for registration in self.routes.values() {
            let path = &registration.definition.path;
            path_middleware
                .entry(path.clone())
                .or_default()
                .push((registration.controller.clone(), registration.middleware.clone()));
        }

        for (path, controllers) in path_middleware {
            if controllers.len() > 1 {
                let middleware_sets: HashSet<Vec<String>> = controllers.iter().map(|(_, mw)| mw.clone()).collect();
                if middleware_sets.len() > 1 {
                    warnings.push(format!(
                        "Inconsistent middleware for path {}: controllers have different middleware requirements",
                        path
                    ));
                }
            }
        }
    }

    /// Check for performance issues
    fn check_performance_issues(&self, warnings: &mut Vec<String>) {
        if self.routes.len() > 1000 {
            warnings.push("Large number of routes (>1000) may impact performance".to_string());
        }

        // Check for overly complex patterns
        for registration in self.routes.values() {
            let param_count = registration.parameters.len();
            if param_count > 5 {
                warnings.push(format!(
                    "Route {} has {} parameters, consider simplifying",
                    registration.definition.path,
                    param_count
                ));
            }
        }
    }

    /// Calculate overall performance score
    fn calculate_performance_score(&self) -> u32 {
        let base_score: u32 = 100;
        let route_penalty = (self.routes.len() / 100) as u32; // 1 point per 100 routes
        
        let complex_routes = self.routes.values()
            .filter(|r| r.parameters.len() > 3)
            .count() as u32;
        
        base_score.saturating_sub(route_penalty + complex_routes)
    }

    /// Generate optimization suggestions
    fn generate_optimization_suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        if self.routes.len() > 500 {
            suggestions.push("Consider grouping routes by modules for better organization".to_string());
        }

        suggestions
    }

    /// Generate resolution suggestions for conflicts
    fn generate_resolution_suggestions(&self, _route1: &RouteInfo, _route2: &RouteInfo, conflict_type: &ConflictType) -> Vec<ConflictResolution> {
        match conflict_type {
            ConflictType::Exact => vec![
                ConflictResolution::DifferentControllerPaths { 
                    suggestion: "Use different base paths like /api/users vs /api/admin/users".to_string() 
                },
                ConflictResolution::MergePaths { 
                    suggestion: "Merge functionality into a single controller".to_string() 
                },
                ConflictResolution::UseQueryParameters { 
                    suggestion: "Use query parameters instead: GET /api/users/{id}?admin=true".to_string() 
                },
            ],
            ConflictType::ParameterMismatch => vec![
                ConflictResolution::RenameParameter { 
                    from: "id".to_string(), 
                    to: "user_id".to_string() 
                },
            ],
            ConflictType::Ambiguous => vec![
                ConflictResolution::ReorderRoutes { 
                    suggestion: "Reorder routes to put more specific patterns first".to_string() 
                },
            ],
            ConflictType::MiddlewareIncompatible => vec![
                ConflictResolution::MiddlewareConsolidation { 
                    suggestion: "Consolidate middleware requirements across controllers".to_string() 
                },
            ],
        }
    }

    /// Format a resolution suggestion for display
    fn format_suggestion(&self, suggestion: &ConflictResolution) -> String {
        match suggestion {
            ConflictResolution::MergePaths { suggestion } => suggestion.clone(),
            ConflictResolution::RenameParameter { from, to } => {
                format!("Rename parameter '{}' to '{}'", from, to)
            },
            ConflictResolution::DifferentControllerPaths { suggestion } => suggestion.clone(),
            ConflictResolution::MiddlewareConsolidation { suggestion } => suggestion.clone(),
            ConflictResolution::UseQueryParameters { suggestion } => suggestion.clone(),
            ConflictResolution::ReorderRoutes { suggestion } => suggestion.clone(),
        }
    }
}

/// Report generated after route validation
#[derive(Debug)]
pub struct ValidationReport {
    pub total_routes: usize,
    pub conflicts: usize,
    pub warnings: usize,
    pub performance_score: u32,
    pub suggestions: Vec<String>,
}

impl Default for RouteValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert RouteValidationError to BootstrapError for integration
impl From<RouteValidationError> for BootstrapError {
    fn from(err: RouteValidationError) -> Self {
        BootstrapError::RouteRegistrationFailed {
            message: format!("Route validation failed: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create test route registration
    fn create_test_route(
        controller: &str,
        handler: &str,
        method: HttpMethod,
        path: &str,
        params: Vec<ParamDef>,
    ) -> RouteRegistration {
        RouteRegistration {
            controller: controller.to_string(),
            handler: handler.to_string(),
            middleware: Vec::new(),
            parameters: params,
            definition: RouteDefinition {
                id: format!("{}::{}", controller, handler),
                method,
                path: path.to_string(),
            },
        }
    }

    #[test]
    fn test_successful_route_registration() {
        let mut validator = RouteValidator::new();
        
        let route = create_test_route(
            "UserController",
            "get_user",
            HttpMethod::GET,
            "/api/users/{id}",
            vec![ParamDef {
                name: "id".to_string(),
                param_type: "u32".to_string(),
                required: true,
                constraints: vec!["int".to_string()],
            }]
        );

        let result = validator.register_route(route);
        assert!(result.is_ok(), "Route registration should succeed");
        
        let report = validator.validate_all_routes().unwrap();
        assert_eq!(report.total_routes, 1);
        assert_eq!(report.conflicts, 0);
    }

    #[test]
    fn test_exact_route_conflict_detection() {
        let mut validator = RouteValidator::new();
        
        // Register first route
        let route1 = create_test_route(
            "UserController",
            "get_user",
            HttpMethod::GET,
            "/api/users/{id}",
            vec![ParamDef {
                name: "id".to_string(),
                param_type: "u32".to_string(),
                required: true,
                constraints: vec!["int".to_string()],
            }]
        );
        validator.register_route(route1).unwrap();

        // Try to register conflicting route
        let route2 = create_test_route(
            "AdminController",
            "get_admin_user",
            HttpMethod::GET,
            "/api/users/{id}",  // Same path!
            vec![ParamDef {
                name: "id".to_string(),
                param_type: "u32".to_string(),
                required: true,
                constraints: vec!["int".to_string()],
            }]
        );

        let result = validator.register_route(route2);
        assert!(result.is_err(), "Conflicting route should be rejected");
        
        match result.unwrap_err() {
            RouteValidationError::ConflictDetected { conflicts } => {
                assert_eq!(conflicts.len(), 1);
                assert!(matches!(conflicts[0].conflict_type, ConflictType::Exact));
            },
            _ => panic!("Expected ConflictDetected error"),
        }
    }

    #[test]
    fn test_parameter_conflict_detection() {
        let validator = RouteValidator::new();
        
        let route1 = create_test_route(
            "UserController",
            "get_user",
            HttpMethod::GET,
            "/api/users/{id}",
            vec![ParamDef {
                name: "id".to_string(),
                param_type: "u32".to_string(),
                required: true,
                constraints: vec!["int".to_string()],
            }]
        );

        let route2 = create_test_route(
            "AdminController", 
            "get_admin_user",
            HttpMethod::GET,
            "/api/users/{id}",
            vec![ParamDef {
                name: "id".to_string(),
                param_type: "String".to_string(), // Different type!
                required: true,
                constraints: vec!["string".to_string()],
            }]
        );

        // Check if parameters conflict
        let conflicts = validator.parameters_conflict(&route1.parameters, &route2.parameters);
        assert!(conflicts, "Parameters with same name but different types should conflict");
    }

    #[test]
    fn test_conflict_report_generation() {
        let validator = RouteValidator::new();
        
        let route_info1 = RouteInfo {
            method: HttpMethod::GET,
            path: "/api/users/{id}".to_string(),
            controller: "UserController".to_string(),
            handler: "get_user".to_string(),
            middleware: Vec::new(),
            parameters: Vec::new(),
        };

        let route_info2 = RouteInfo {
            method: HttpMethod::GET,
            path: "/api/users/{id}".to_string(),
            controller: "AdminController".to_string(),
            handler: "get_admin_user".to_string(),
            middleware: Vec::new(),
            parameters: Vec::new(),
        };

        let conflict = RouteConflict {
            route1: route_info1,
            route2: route_info2,
            conflict_type: ConflictType::Exact,
            resolution_suggestions: vec![
                ConflictResolution::DifferentControllerPaths {
                    suggestion: "Use different paths".to_string()
                }
            ],
        };

        let report = validator.generate_conflict_report(&[conflict]);
        
        assert!(report.contains("Duplicate route definition detected"));
        assert!(report.contains("UserController::get_user"));
        assert!(report.contains("AdminController::get_admin_user"));
        assert!(report.contains("Resolution suggestions"));
    }

    #[test]
    fn test_validation_report_generation() {
        let mut validator = RouteValidator::new();
        
        // Register multiple routes
        for i in 0..5 {
            let route = create_test_route(
                &format!("Controller{}", i),
                "handler",
                HttpMethod::GET,
                &format!("/api/resource{}/{}", i, "{id}"),
                vec![ParamDef {
                    name: "id".to_string(),
                    param_type: "u32".to_string(),
                    required: true,
                    constraints: vec!["int".to_string()],
                }]
            );
            validator.register_route(route).unwrap();
        }

        let report = validator.validate_all_routes().unwrap();
        
        assert_eq!(report.total_routes, 5);
        assert_eq!(report.conflicts, 0);
        assert!(report.performance_score > 0);
    }

    #[test]
    fn test_performance_scoring() {
        let validator = RouteValidator::new();
        
        // Empty validator should have perfect score
        let score = validator.calculate_performance_score();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_resolution_suggestions() {
        let validator = RouteValidator::new();
        
        let route1 = RouteInfo {
            method: HttpMethod::GET,
            path: "/api/users".to_string(),
            controller: "UserController".to_string(),
            handler: "list".to_string(),
            middleware: Vec::new(),
            parameters: Vec::new(),
        };

        let route2 = RouteInfo {
            method: HttpMethod::GET,
            path: "/api/users".to_string(),
            controller: "AdminController".to_string(),
            handler: "list_admin".to_string(),
            middleware: Vec::new(),
            parameters: Vec::new(),
        };

        let suggestions = validator.generate_resolution_suggestions(&route1, &route2, &ConflictType::Exact);
        
        assert!(!suggestions.is_empty());
        assert!(matches!(suggestions[0], ConflictResolution::DifferentControllerPaths { .. }));
    }
}