//! Route matching engine for elif.rs
//!
//! This module provides the core route matching functionality that efficiently
//! resolves incoming requests to the appropriate route handlers.

use super::HttpMethod;
use super::pattern::{RoutePattern, CompiledRoute, RouteMatch, RouteId, RoutePatternError};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during route matching
#[derive(Error, Debug)]
pub enum RouteMatchError {
    #[error("No matching route found")]
    NoMatch,
    #[error("Route pattern error: {0}")]
    PatternError(#[from] RoutePatternError),
    #[error("Conflicting routes: {0} conflicts with {1}")]
    RouteConflict(String, String),
}

/// Definition of a route to be compiled
#[derive(Debug, Clone)]
pub struct RouteDefinition {
    pub id: RouteId,
    pub method: HttpMethod,
    pub path: String,
}

/// High-performance route matcher
#[derive(Debug)]
pub struct RouteMatcher {
    /// Static routes for O(1) lookup - nested to avoid string allocation
    /// Structure: method -> path -> route_id
    static_routes: HashMap<HttpMethod, HashMap<String, RouteId>>,
    /// Dynamic routes sorted by priority
    dynamic_routes: Vec<CompiledRoute>,
    /// All route definitions for introspection
    route_definitions: HashMap<RouteId, RouteDefinition>,
}

impl RouteMatcher {
    /// Create a new empty route matcher
    pub fn new() -> Self {
        Self {
            static_routes: HashMap::new(),
            dynamic_routes: Vec::new(),
            route_definitions: HashMap::new(),
        }
    }

    /// Add a route to the matcher
    pub fn add_route(&mut self, definition: RouteDefinition) -> Result<(), RouteMatchError> {
        let pattern = RoutePattern::parse(&definition.path)?;
        
        // Check for route conflicts
        self.check_conflicts(&definition, &pattern)?;
        
        // Store the definition
        self.route_definitions.insert(definition.id.clone(), definition.clone());
        
        if pattern.is_static() {
            // Static route - add to nested lookup table
            self.static_routes
                .entry(definition.method.clone())
                .or_insert_with(HashMap::new)
                .insert(definition.path.clone(), definition.id);
        } else {
            // Dynamic route - compile and add to sorted list
            let compiled_route = CompiledRoute::new(definition.id, definition.method, pattern);
            
            // Insert in priority order (lower priority value = higher precedence)
            let insert_pos = self.dynamic_routes
                .binary_search_by_key(&compiled_route.priority, |r| r.priority)
                .unwrap_or_else(|pos| pos);
            
            self.dynamic_routes.insert(insert_pos, compiled_route);
        }
        
        Ok(())
    }

    /// Resolve an incoming request to a matching route
    pub fn resolve(&self, method: &HttpMethod, path: &str) -> Option<RouteMatch> {
        // Fast path: check static routes first (no allocation!)
        if let Some(method_routes) = self.static_routes.get(method) {
            if let Some(route_id) = method_routes.get(path) {
                return Some(RouteMatch {
                    route_id: route_id.clone(),
                    params: HashMap::new(),
                });
            }
        }
        
        // Dynamic route matching
        for compiled_route in &self.dynamic_routes {
            if compiled_route.matches(method, path) {
                let params = compiled_route.extract_params(path);
                return Some(RouteMatch {
                    route_id: compiled_route.id.clone(),
                    params,
                });
            }
        }
        
        None
    }

    /// Check for route conflicts before adding a new route
    fn check_conflicts(&self, new_route: &RouteDefinition, new_pattern: &RoutePattern) -> Result<(), RouteMatchError> {
        // Check against static routes
        if new_pattern.is_static() {
            if let Some(method_routes) = self.static_routes.get(&new_route.method) {
                if let Some(existing_id) = method_routes.get(&new_route.path) {
                    return Err(RouteMatchError::RouteConflict(
                        new_route.id.clone(),
                        existing_id.clone(),
                    ));
                }
            }
        }
        
        // Check against dynamic routes
        for existing_route in &self.dynamic_routes {
            if existing_route.method == new_route.method {
                if self.patterns_conflict(new_pattern, &existing_route.pattern) {
                    return Err(RouteMatchError::RouteConflict(
                        new_route.id.clone(),
                        existing_route.id.clone(),
                    ));
                }
            }
        }
        
        Ok(())
    }

    /// Check if two patterns would conflict (ambiguous matching)
    fn patterns_conflict(&self, pattern1: &RoutePattern, pattern2: &RoutePattern) -> bool {
        // Two patterns conflict if they are structurally identical.
        // This means they have the same number of segments, and each corresponding
        // segment is of the same type with the same static value or constraint.
        if pattern1.segments.len() != pattern2.segments.len() {
            return false;
        }
        
        for (seg1, seg2) in pattern1.segments.iter().zip(pattern2.segments.iter()) {
            match (seg1, seg2) {
                (super::pattern::PathSegment::Static(s1), super::pattern::PathSegment::Static(s2)) if s1 == s2 => continue,
                (super::pattern::PathSegment::Parameter { constraint: c1, .. }, super::pattern::PathSegment::Parameter { constraint: c2, .. }) if c1 == c2 => continue,
                (super::pattern::PathSegment::CatchAll { .. }, super::pattern::PathSegment::CatchAll { .. }) => continue,
                _ => return false, // Segments are not structurally identical
            }
        }
        
        true // All segments are structurally identical, so the patterns conflict.
    }

    /// Get all route definitions for introspection
    pub fn all_routes(&self) -> &HashMap<RouteId, RouteDefinition> {
        &self.route_definitions
    }

    /// Get a specific route definition
    pub fn get_route(&self, route_id: &RouteId) -> Option<&RouteDefinition> {
        self.route_definitions.get(route_id)
    }

    /// Get statistics about the matcher
    pub fn stats(&self) -> MatcherStats {
        let static_routes_count = self.static_routes
            .values()
            .map(|method_routes| method_routes.len())
            .sum();
        
        MatcherStats {
            static_routes: static_routes_count,
            dynamic_routes: self.dynamic_routes.len(),
            total_routes: self.route_definitions.len(),
        }
    }

    /// Clear all routes
    pub fn clear(&mut self) {
        self.static_routes.clear();
        self.dynamic_routes.clear();
        self.route_definitions.clear();
    }
}

impl Default for RouteMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the route matcher
#[derive(Debug, Clone)]
pub struct MatcherStats {
    pub static_routes: usize,
    pub dynamic_routes: usize,
    pub total_routes: usize,
}

/// Builder for creating route matchers
pub struct RouteMatcherBuilder {
    routes: Vec<RouteDefinition>,
}

impl RouteMatcherBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
        }
    }

    /// Add a route to the builder
    pub fn route(mut self, id: String, method: HttpMethod, path: String) -> Self {
        self.routes.push(RouteDefinition { id, method, path });
        self
    }

    /// Add a GET route
    pub fn get(self, id: String, path: String) -> Self {
        self.route(id, HttpMethod::GET, path)
    }

    /// Add a POST route
    pub fn post(self, id: String, path: String) -> Self {
        self.route(id, HttpMethod::POST, path)
    }

    /// Add a PUT route
    pub fn put(self, id: String, path: String) -> Self {
        self.route(id, HttpMethod::PUT, path)
    }

    /// Add a DELETE route
    pub fn delete(self, id: String, path: String) -> Self {
        self.route(id, HttpMethod::DELETE, path)
    }

    /// Add a PATCH route
    pub fn patch(self, id: String, path: String) -> Self {
        self.route(id, HttpMethod::PATCH, path)
    }

    /// Build the route matcher
    pub fn build(self) -> Result<RouteMatcher, RouteMatchError> {
        let mut matcher = RouteMatcher::new();
        
        for route_def in self.routes {
            matcher.add_route(route_def)?;
        }
        
        Ok(matcher)
    }
}

impl Default for RouteMatcherBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_route_matching() {
        let mut matcher = RouteMatcher::new();
        
        let route_def = RouteDefinition {
            id: "home".to_string(),
            method: HttpMethod::GET,
            path: "/".to_string(),
        };
        
        matcher.add_route(route_def).unwrap();
        
        let result = matcher.resolve(&HttpMethod::GET, "/");
        assert!(result.is_some());
        
        let route_match = result.unwrap();
        assert_eq!(route_match.route_id, "home");
        assert!(route_match.params.is_empty());
        
        // Should not match different method
        assert!(matcher.resolve(&HttpMethod::POST, "/").is_none());
        
        // Should not match different path
        assert!(matcher.resolve(&HttpMethod::GET, "/users").is_none());
    }

    #[test]
    fn test_dynamic_route_matching() {
        let mut matcher = RouteMatcher::new();
        
        let route_def = RouteDefinition {
            id: "user_show".to_string(),
            method: HttpMethod::GET,
            path: "/users/{id}".to_string(),
        };
        
        matcher.add_route(route_def).unwrap();
        
        let result = matcher.resolve(&HttpMethod::GET, "/users/123");
        assert!(result.is_some());
        
        let route_match = result.unwrap();
        assert_eq!(route_match.route_id, "user_show");
        assert_eq!(route_match.params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_route_priority() {
        let mut matcher = RouteMatcher::new();
        
        // Add in reverse priority order to test sorting
        matcher.add_route(RouteDefinition {
            id: "catch_all".to_string(),
            method: HttpMethod::GET,
            path: "/files/*path".to_string(),
        }).unwrap();
        
        matcher.add_route(RouteDefinition {
            id: "specific".to_string(),
            method: HttpMethod::GET,
            path: "/files/config.json".to_string(),
        }).unwrap();
        
        matcher.add_route(RouteDefinition {
            id: "param".to_string(),
            method: HttpMethod::GET,
            path: "/files/{name}".to_string(),
        }).unwrap();
        
        // Static route should match first
        let result = matcher.resolve(&HttpMethod::GET, "/files/config.json");
        assert_eq!(result.unwrap().route_id, "specific");
        
        // Parameter route should match before catch-all
        let result = matcher.resolve(&HttpMethod::GET, "/files/other.txt");
        assert_eq!(result.unwrap().route_id, "param");
        
        // Catch-all should match multi-segment paths
        let result = matcher.resolve(&HttpMethod::GET, "/files/docs/readme.md");
        assert_eq!(result.unwrap().route_id, "catch_all");
    }

    #[test]
    fn test_route_conflict_detection() {
        let mut matcher = RouteMatcher::new();
        
        // Add first route
        matcher.add_route(RouteDefinition {
            id: "route1".to_string(),
            method: HttpMethod::GET,
            path: "/users".to_string(),
        }).unwrap();
        
        // Try to add conflicting static route
        let result = matcher.add_route(RouteDefinition {
            id: "route2".to_string(),
            method: HttpMethod::GET,
            path: "/users".to_string(),
        });
        
        assert!(result.is_err());
        assert!(matches!(result, Err(RouteMatchError::RouteConflict(_, _))));
    }

    #[test]
    fn test_advanced_conflict_detection() {
        let mut matcher = RouteMatcher::new();
        
        // Test 1: Parameter routes with same structure should conflict
        matcher.add_route(RouteDefinition {
            id: "users_by_id".to_string(),
            method: HttpMethod::GET,
            path: "/users/{id}".to_string(),
        }).unwrap();
        
        let result = matcher.add_route(RouteDefinition {
            id: "users_by_name".to_string(),
            method: HttpMethod::GET,
            path: "/users/{name}".to_string(),
        });
        assert!(result.is_err(), "Parameters with different names should conflict");
        
        // Test 2: Different methods should not conflict
        let result = matcher.add_route(RouteDefinition {
            id: "users_post".to_string(),
            method: HttpMethod::POST,
            path: "/users/{id}".to_string(),
        });
        assert!(result.is_ok(), "Different methods should not conflict");
        
        // Test 3: Different static segments should not conflict
        let result = matcher.add_route(RouteDefinition {
            id: "posts_by_id".to_string(),
            method: HttpMethod::GET,
            path: "/posts/{id}".to_string(),
        });
        assert!(result.is_ok(), "Different static segments should not conflict");
        
        // Test 4: Catch-all routes with same structure should conflict
        matcher.add_route(RouteDefinition {
            id: "files_serve".to_string(),
            method: HttpMethod::GET,
            path: "/files/*path".to_string(),
        }).unwrap();
        
        let result = matcher.add_route(RouteDefinition {
            id: "files_download".to_string(),
            method: HttpMethod::GET,
            path: "/files/*file_path".to_string(),
        });
        assert!(result.is_err(), "Catch-all routes with same structure should conflict");
        
        // Test 5: Different segment types should not conflict
        let result = matcher.add_route(RouteDefinition {
            id: "admin_static".to_string(),
            method: HttpMethod::GET,
            path: "/admin/dashboard".to_string(),
        });
        assert!(result.is_ok(), "Static vs parameter segments should not conflict");
    }

    #[test] 
    fn test_constraint_based_conflicts() {
        let mut matcher = RouteMatcher::new();
        
        // Test 1: Same constraints should conflict
        matcher.add_route(RouteDefinition {
            id: "user_by_int_id".to_string(),
            method: HttpMethod::GET,
            path: "/users/{id:int}".to_string(),
        }).unwrap();
        
        let result = matcher.add_route(RouteDefinition {
            id: "user_by_int_uid".to_string(),
            method: HttpMethod::GET,
            path: "/users/{uid:int}".to_string(),
        });
        assert!(result.is_err(), "Same constraints should conflict");
        
        // Test 2: Different constraints should not conflict (they have different precedence)
        let result = matcher.add_route(RouteDefinition {
            id: "user_by_uuid".to_string(),
            method: HttpMethod::GET,
            path: "/users/{id:uuid}".to_string(),
        });
        assert!(result.is_ok(), "Different constraints should not conflict");
        
        // Test 3: Constrained vs unconstrained should not conflict
        let result = matcher.add_route(RouteDefinition {
            id: "user_by_string".to_string(),
            method: HttpMethod::GET,
            path: "/users/{name}".to_string(),
        });
        assert!(result.is_ok(), "Constrained vs unconstrained should not conflict");
    }

    #[test]
    fn test_complex_pattern_conflicts() {
        let mut matcher = RouteMatcher::new();
        
        // Test complex multi-segment patterns
        matcher.add_route(RouteDefinition {
            id: "api_user_posts".to_string(),
            method: HttpMethod::GET,
            path: "/api/v1/users/{user_id}/posts/{post_id}".to_string(),
        }).unwrap();
        
        // Same structure should conflict
        let result = matcher.add_route(RouteDefinition {
            id: "api_member_articles".to_string(),
            method: HttpMethod::GET,
            path: "/api/v1/users/{member_id}/posts/{article_id}".to_string(),
        });
        assert!(result.is_err(), "Structurally identical complex patterns should conflict");
        
        // Different static segment should not conflict
        let result = matcher.add_route(RouteDefinition {
            id: "api_user_comments".to_string(),
            method: HttpMethod::GET,
            path: "/api/v1/users/{user_id}/comments/{comment_id}".to_string(),
        });
        assert!(result.is_ok(), "Different static segments should not conflict");
        
        // Different segment count should not conflict
        let result = matcher.add_route(RouteDefinition {
            id: "api_user_profile".to_string(),
            method: HttpMethod::GET,
            path: "/api/v1/users/{user_id}/profile".to_string(),
        });
        assert!(result.is_ok(), "Different segment count should not conflict");
    }

    #[test]
    fn test_matcher_builder() {
        let matcher = RouteMatcherBuilder::new()
            .get("home".to_string(), "/".to_string())
            .post("users_create".to_string(), "/users".to_string())
            .get("users_show".to_string(), "/users/{id}".to_string())
            .build()
            .unwrap();
        
        let stats = matcher.stats();
        assert_eq!(stats.total_routes, 3);
        assert_eq!(stats.static_routes, 2); // "/" and "/users" for POST
        assert_eq!(stats.dynamic_routes, 1); // "/users/{id}"
        
        // Test that routes work
        assert!(matcher.resolve(&HttpMethod::GET, "/").is_some());
        assert!(matcher.resolve(&HttpMethod::POST, "/users").is_some());
        assert!(matcher.resolve(&HttpMethod::GET, "/users/123").is_some());
    }

    #[test]
    fn test_constraint_validation_in_matching() {
        let mut matcher = RouteMatcher::new();
        
        matcher.add_route(RouteDefinition {
            id: "user_by_id".to_string(),
            method: HttpMethod::GET,
            path: "/users/{id:int}".to_string(),
        }).unwrap();
        
        // Should match valid integer
        let result = matcher.resolve(&HttpMethod::GET, "/users/123");
        assert!(result.is_some());
        assert_eq!(result.unwrap().route_id, "user_by_id");
        
        // Should not match invalid integer
        let result = matcher.resolve(&HttpMethod::GET, "/users/abc");
        assert!(result.is_none());
    }

    #[test]
    fn test_mixed_static_and_dynamic_routes() {
        let mut matcher = RouteMatcher::new();
        
        // Mix of static and dynamic routes
        matcher.add_route(RouteDefinition {
            id: "api_status".to_string(),
            method: HttpMethod::GET,
            path: "/api/status".to_string(),
        }).unwrap();
        
        matcher.add_route(RouteDefinition {
            id: "api_user".to_string(),
            method: HttpMethod::GET,
            path: "/api/users/{id}".to_string(),
        }).unwrap();
        
        matcher.add_route(RouteDefinition {
            id: "root".to_string(),
            method: HttpMethod::GET,
            path: "/".to_string(),
        }).unwrap();
        
        // Test static route lookup
        let result = matcher.resolve(&HttpMethod::GET, "/api/status");
        assert_eq!(result.unwrap().route_id, "api_status");
        
        // Test root route
        let result = matcher.resolve(&HttpMethod::GET, "/");
        assert_eq!(result.unwrap().route_id, "root");
        
        // Test dynamic route
        let result = matcher.resolve(&HttpMethod::GET, "/api/users/456");
        let route_match = result.unwrap();
        assert_eq!(route_match.route_id, "api_user");
        assert_eq!(route_match.params.get("id"), Some(&"456".to_string()));
    }

    #[test]
    fn test_no_match() {
        let matcher = RouteMatcherBuilder::new()
            .get("home".to_string(), "/".to_string())
            .build()
            .unwrap();
        
        assert!(matcher.resolve(&HttpMethod::GET, "/nonexistent").is_none());
        assert!(matcher.resolve(&HttpMethod::POST, "/").is_none());
    }

    #[test]
    fn test_static_route_lookup_performance() {
        // Create a matcher with many static routes across different methods
        let mut builder = RouteMatcherBuilder::new();
        
        for i in 0..100 {
            builder = builder
                .get(format!("get_{}", i), format!("/static/path/{}", i))
                .post(format!("post_{}", i), format!("/api/v1/{}", i))
                .put(format!("put_{}", i), format!("/resource/{}", i));
        }
        
        let matcher = builder.build().unwrap();
        let stats = matcher.stats();
        
        // Verify we have static routes
        assert_eq!(stats.static_routes, 300); // 100 routes × 3 methods
        assert_eq!(stats.dynamic_routes, 0);
        
        // Test lookup performance - these should be O(1) with no allocations
        let start = std::time::Instant::now();
        
        // Perform many lookups
        for i in 0..1000 {
            let test_index = i % 100;
            
            // These lookups should not allocate strings
            let result = matcher.resolve(&HttpMethod::GET, &format!("/static/path/{}", test_index));
            assert!(result.is_some());
            
            let result = matcher.resolve(&HttpMethod::POST, &format!("/api/v1/{}", test_index));
            assert!(result.is_some());
            
            let result = matcher.resolve(&HttpMethod::PUT, &format!("/resource/{}", test_index));
            assert!(result.is_some());
            
            // Test non-existent path
            let result = matcher.resolve(&HttpMethod::GET, "/nonexistent/path");
            assert!(result.is_none());
        }
        
        let elapsed = start.elapsed();
        
        // This test primarily verifies that the optimization doesn't break functionality
        // The performance benefit (no string allocation) can't be directly tested in a unit test,
        // but the nested HashMap structure ensures we only do &str lookups
        
        // Should complete very quickly due to O(1) lookups
        assert!(elapsed.as_millis() < 100, "Static route lookups took too long: {}ms", elapsed.as_millis());
        
        println!("3000 static route lookups completed in {}μs", elapsed.as_micros());
    }
}