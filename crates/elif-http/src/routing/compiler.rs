//! Route compilation and optimization for elif.rs
//!
//! This module provides compilation and optimization of route definitions
//! into efficient runtime structures for high-performance route matching.

use super::{HttpMethod, RouteInfo};
use super::pattern::{RoutePattern, RoutePatternError};
use super::matcher::{RouteMatcher, RouteDefinition, RouteMatchError};
use super::extraction::ParameterExtractor;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during route compilation
#[derive(Error, Debug)]
pub enum CompilationError {
    #[error("Route pattern error: {0}")]
    PatternError(#[from] RoutePatternError),
    #[error("Route matching error: {0}")]
    MatcherError(#[from] RouteMatchError),
    #[error("Duplicate route ID: {0}")]
    DuplicateRouteId(String),
    #[error("Route conflict detected: {0} conflicts with {1}")]
    RouteConflict(String, String),
    #[error("Invalid route configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
}

/// Configuration for route compilation
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Enable conflict detection
    pub detect_conflicts: bool,
    /// Enable route optimization
    pub enable_optimization: bool,
    /// Maximum number of routes before warning
    pub max_routes_warning: usize,
    /// Enable performance analysis
    pub performance_analysis: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            detect_conflicts: true,
            enable_optimization: true,
            max_routes_warning: 1000,
            performance_analysis: true,
        }
    }
}

/// Statistics about compiled routes
#[derive(Debug, Clone)]
pub struct CompilationStats {
    pub total_routes: usize,
    pub static_routes: usize,
    pub dynamic_routes: usize,
    pub parameter_routes: usize,
    pub catch_all_routes: usize,
    pub conflicts_detected: usize,
    pub optimizations_applied: usize,
    pub compilation_time_ms: u128,
}

/// A single route definition for compilation
#[derive(Debug, Clone)]
pub struct CompilableRoute {
    pub id: String,
    pub method: HttpMethod,
    pub path: String,
    pub name: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl CompilableRoute {
    pub fn new(id: String, method: HttpMethod, path: String) -> Self {
        Self {
            id,
            method,
            path,
            name: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Result of route compilation
#[derive(Debug)]
pub struct CompilationResult {
    pub matcher: RouteMatcher,
    pub extractors: HashMap<String, ParameterExtractor>,
    pub route_registry: HashMap<String, RouteInfo>,
    pub stats: CompilationStats,
    pub warnings: Vec<String>,
}

/// Route compiler with optimization and validation
#[derive(Debug)]
pub struct RouteCompiler {
    config: CompilerConfig,
    routes: Vec<CompilableRoute>,
    route_ids: HashSet<String>,
}

impl RouteCompiler {
    /// Create a new route compiler
    pub fn new() -> Self {
        Self::with_config(CompilerConfig::default())
    }

    /// Create a new route compiler with custom configuration
    pub fn with_config(config: CompilerConfig) -> Self {
        Self {
            config,
            routes: Vec::new(),
            route_ids: HashSet::new(),
        }
    }

    /// Add a route to be compiled
    pub fn add_route(&mut self, route: CompilableRoute) -> Result<(), CompilationError> {
        // Check for duplicate route IDs
        if self.route_ids.contains(&route.id) {
            return Err(CompilationError::DuplicateRouteId(route.id));
        }

        self.route_ids.insert(route.id.clone());
        self.routes.push(route);
        Ok(())
    }

    /// Add multiple routes
    pub fn add_routes(&mut self, routes: Vec<CompilableRoute>) -> Result<(), CompilationError> {
        for route in routes {
            self.add_route(route)?;
        }
        Ok(())
    }

    /// Compile all routes into optimized structures
    pub fn compile(self) -> Result<CompilationResult, CompilationError> {
        let start_time = std::time::Instant::now();
        let mut warnings = Vec::new();
        let mut optimizations_applied = 0;

        // Check route count warning
        if self.routes.len() > self.config.max_routes_warning {
            warnings.push(format!(
                "Large number of routes detected: {}. Consider route grouping or optimization.",
                self.routes.len()
            ));
        }

        // Parse and validate all route patterns
        let mut parsed_routes = Vec::new();
        let mut static_count = 0;
        let mut dynamic_count = 0;
        let mut parameter_count = 0;
        let mut catch_all_count = 0;

        for route in &self.routes {
            let pattern = RoutePattern::parse(&route.path)?;
            
            // Collect statistics
            if pattern.is_static() {
                static_count += 1;
            } else {
                dynamic_count += 1;
                if pattern.has_catch_all {
                    catch_all_count += 1;
                } else if !pattern.param_names.is_empty() {
                    parameter_count += 1;
                }
            }

            parsed_routes.push((route.clone(), pattern));
        }

        // Create route matcher
        let mut matcher = RouteMatcher::new();
        let mut extractors = HashMap::new();
        let mut route_registry = HashMap::new();
        let mut conflicts_detected = 0;

        // Apply optimizations if enabled
        if self.config.enable_optimization {
            parsed_routes = self.optimize_routes(parsed_routes);
            optimizations_applied += 1;
        }

        // Add routes to matcher and create extractors
        for (route, pattern) in parsed_routes {
            // Create route definition
            let route_def = RouteDefinition {
                id: route.id.clone(),
                method: route.method.clone(),
                path: route.path.clone(),
            };

            // Try to add route, handle conflicts
            match matcher.add_route(route_def) {
                Ok(()) => {
                    // Create parameter extractor for dynamic routes
                    if !pattern.is_static() {
                        let extractor = ParameterExtractor::new(pattern.clone());
                        extractors.insert(route.id.clone(), extractor);
                    }

                    // Create route info for registry
                    let route_info = RouteInfo {
                        name: route.name.clone(),
                        path: route.path.clone(),
                        method: route.method.clone(),
                        params: pattern.param_names.clone(),
                        group: route.metadata.get("group").cloned(),
                    };
                    route_registry.insert(route.id.clone(), route_info);
                }
                Err(RouteMatchError::RouteConflict(source, target)) => {
                    if self.config.detect_conflicts {
                        return Err(CompilationError::RouteConflict(source, target));
                    } else {
                        conflicts_detected += 1;
                        warnings.push(format!("Route conflict detected: {} conflicts with {}", source, target));
                    }
                }
                Err(e) => return Err(CompilationError::MatcherError(e)),
            }
        }

        let compilation_time = start_time.elapsed().as_millis();

        // Performance analysis
        if self.config.performance_analysis && compilation_time > 100 {
            warnings.push(format!(
                "Route compilation took {}ms. Consider optimizing route patterns or reducing route count.",
                compilation_time
            ));
        }

        let stats = CompilationStats {
            total_routes: self.routes.len(),
            static_routes: static_count,
            dynamic_routes: dynamic_count,
            parameter_routes: parameter_count,
            catch_all_routes: catch_all_count,
            conflicts_detected,
            optimizations_applied,
            compilation_time_ms: compilation_time,
        };

        Ok(CompilationResult {
            matcher,
            extractors,
            route_registry,
            stats,
            warnings,
        })
    }

    /// Optimize route ordering for better performance
    fn optimize_routes(&self, mut routes: Vec<(CompilableRoute, RoutePattern)>) -> Vec<(CompilableRoute, RoutePattern)> {
        // Sort routes by specificity (more specific routes first)
        // This improves matching performance for common cases
        routes.sort_by(|(_, pattern_a), (_, pattern_b)| {
            // Primary sort: static routes first
            let static_a = pattern_a.is_static();
            let static_b = pattern_b.is_static();
            
            match (static_a, static_b) {
                (true, false) => std::cmp::Ordering::Less,    // Static before dynamic
                (false, true) => std::cmp::Ordering::Greater, // Dynamic after static
                _ => {
                    // Secondary sort: by priority (lower = more specific)
                    pattern_a.priority().cmp(&pattern_b.priority())
                }
            }
        });

        routes
    }

}

impl Default for RouteCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating route compilers with fluent API
#[derive(Debug)]
pub struct RouteCompilerBuilder {
    config: CompilerConfig,
    routes: Vec<CompilableRoute>,
}

impl RouteCompilerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: CompilerConfig::default(),
            routes: Vec::new(),
        }
    }

    /// Set compiler configuration
    pub fn config(mut self, config: CompilerConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable or disable conflict detection
    pub fn detect_conflicts(mut self, enabled: bool) -> Self {
        self.config.detect_conflicts = enabled;
        self
    }

    /// Enable or disable route optimization
    pub fn optimize(mut self, enabled: bool) -> Self {
        self.config.enable_optimization = enabled;
        self
    }

    /// Set maximum routes warning threshold
    pub fn max_routes_warning(mut self, max: usize) -> Self {
        self.config.max_routes_warning = max;
        self
    }

    /// Add a route
    pub fn route(mut self, route: CompilableRoute) -> Self {
        self.routes.push(route);
        self
    }

    /// Add a GET route
    pub fn get(mut self, id: String, path: String) -> Self {
        self.routes.push(CompilableRoute::new(id, HttpMethod::GET, path));
        self
    }

    /// Add a POST route
    pub fn post(mut self, id: String, path: String) -> Self {
        self.routes.push(CompilableRoute::new(id, HttpMethod::POST, path));
        self
    }

    /// Add a PUT route
    pub fn put(mut self, id: String, path: String) -> Self {
        self.routes.push(CompilableRoute::new(id, HttpMethod::PUT, path));
        self
    }

    /// Add a DELETE route
    pub fn delete(mut self, id: String, path: String) -> Self {
        self.routes.push(CompilableRoute::new(id, HttpMethod::DELETE, path));
        self
    }

    /// Add a PATCH route
    pub fn patch(mut self, id: String, path: String) -> Self {
        self.routes.push(CompilableRoute::new(id, HttpMethod::PATCH, path));
        self
    }

    /// Build and compile the routes
    pub fn build(self) -> Result<CompilationResult, CompilationError> {
        let mut compiler = RouteCompiler::with_config(self.config);
        
        for route in self.routes {
            compiler.add_route(route)?;
        }
        
        compiler.compile()
    }
}

impl Default for RouteCompilerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_compilation() {
        let result = RouteCompilerBuilder::new()
            .get("home".to_string(), "/".to_string())
            .get("users_index".to_string(), "/users".to_string())
            .get("users_show".to_string(), "/users/{id}".to_string())
            .build()
            .unwrap();

        assert_eq!(result.stats.total_routes, 3);
        assert_eq!(result.stats.static_routes, 2);
        assert_eq!(result.stats.dynamic_routes, 1);
        assert_eq!(result.stats.parameter_routes, 1);
    }

    #[test]
    fn test_route_optimization() {
        let result = RouteCompilerBuilder::new()
            .optimize(true)
            .get("catch_all".to_string(), "/files/*path".to_string())
            .get("specific".to_string(), "/files/config.json".to_string())
            .get("param".to_string(), "/files/{name}".to_string())
            .build()
            .unwrap();

        assert_eq!(result.stats.optimizations_applied, 1);
        
        // Test that matching works correctly with optimized order
        let matcher = result.matcher;
        
        // Static route should match first
        let route_match = matcher.resolve(&HttpMethod::GET, "/files/config.json").unwrap();
        assert_eq!(route_match.route_id, "specific");
        
        // Parameter route should match next
        let route_match = matcher.resolve(&HttpMethod::GET, "/files/readme.txt").unwrap();
        assert_eq!(route_match.route_id, "param");
        
        // Catch-all should match complex paths
        let route_match = matcher.resolve(&HttpMethod::GET, "/files/docs/api.md").unwrap();
        assert_eq!(route_match.route_id, "catch_all");
    }

    #[test]
    fn test_conflict_detection() {
        let result = RouteCompilerBuilder::new()
            .detect_conflicts(true)
            .get("route1".to_string(), "/users".to_string())
            .get("route2".to_string(), "/users".to_string())
            .build();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompilationError::RouteConflict(_, _)));
    }

    #[test]
    fn test_conflict_warnings() {
        let result = RouteCompilerBuilder::new()
            .detect_conflicts(false) // Disable conflict errors, enable warnings
            .get("route1".to_string(), "/users".to_string())
            .get("route2".to_string(), "/users".to_string())
            .build()
            .unwrap();

        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("conflict"));
        assert_eq!(result.stats.conflicts_detected, 1);
    }

    #[test]
    fn test_parameter_extractors() {
        let result = RouteCompilerBuilder::new()
            .get("users_show".to_string(), "/users/{id:int}".to_string())
            .get("posts_show".to_string(), "/posts/{slug}/comments/{id:uuid}".to_string())
            .build()
            .unwrap();

        // Should create extractors for dynamic routes
        assert!(result.extractors.contains_key("users_show"));
        assert!(result.extractors.contains_key("posts_show"));
        assert_eq!(result.extractors.len(), 2);

        // Test extractor functionality
        let users_extractor = result.extractors.get("users_show").unwrap();
        let extracted = users_extractor.extract("/users/123").unwrap();
        assert_eq!(extracted.get_int("id").unwrap(), 123);
    }

    #[test]
    fn test_route_registry() {
        let result = RouteCompilerBuilder::new()
            .route(CompilableRoute::new("users_show".to_string(), HttpMethod::GET, "/users/{id}".to_string())
                .with_name("users.show".to_string())
                .with_metadata("group".to_string(), "users".to_string()))
            .build()
            .unwrap();

        let route_info = result.route_registry.get("users_show").unwrap();
        assert_eq!(route_info.name, Some("users.show".to_string()));
        assert_eq!(route_info.group, Some("users".to_string()));
        assert_eq!(route_info.params, vec!["id"]);
    }

    #[test]
    fn test_compilation_stats() {
        let result = RouteCompilerBuilder::new()
            .get("static1".to_string(), "/".to_string())
            .get("static2".to_string(), "/about".to_string())
            .get("param1".to_string(), "/users/{id}".to_string())
            .get("param2".to_string(), "/posts/{slug}".to_string())
            .get("catch_all".to_string(), "/files/*path".to_string())
            .build()
            .unwrap();

        let stats = result.stats;
        assert_eq!(stats.total_routes, 5);
        assert_eq!(stats.static_routes, 2);
        assert_eq!(stats.dynamic_routes, 3);
        assert_eq!(stats.parameter_routes, 2);
        assert_eq!(stats.catch_all_routes, 1);
    }

    #[test]
    fn test_duplicate_route_id() {
        let mut compiler = RouteCompiler::new();
        
        let route1 = CompilableRoute::new("duplicate".to_string(), HttpMethod::GET, "/path1".to_string());
        let route2 = CompilableRoute::new("duplicate".to_string(), HttpMethod::POST, "/path2".to_string());
        
        compiler.add_route(route1).unwrap();
        let result = compiler.add_route(route2);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompilationError::DuplicateRouteId(_)));
    }

    #[test]
    fn test_performance_warnings() {
        // Create many routes to trigger performance warning
        let mut builder = RouteCompilerBuilder::new()
            .max_routes_warning(5);

        for i in 0..10 {
            builder = builder.get(format!("route_{}", i), format!("/route_{}", i));
        }

        let result = builder.build().unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("Large number of routes")));
    }
}