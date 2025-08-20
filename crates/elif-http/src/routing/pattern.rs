//! Route pattern matching system for elif.rs
//!
//! This module provides the core route pattern parsing and matching functionality
//! that is independent of Axum and uses pure elif types.

use super::HttpMethod;
use regex::Regex;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during route pattern operations
#[derive(Error, Debug)]
pub enum RoutePatternError {
    #[error("Invalid pattern syntax: {0}")]
    InvalidSyntax(String),
    #[error("Multiple catch-all segments not allowed")]
    MultipleCatchAll,
    #[error("Catch-all must be the last segment")]
    CatchAllNotLast,
    #[error("Invalid constraint syntax: {0}")]
    InvalidConstraint(String),
    #[error("Duplicate parameter name: {0}")]
    DuplicateParameter(String),
}

/// Parameter constraints for validation
#[derive(Debug, Clone)]
pub enum ParamConstraint {
    /// No constraint - any non-empty string
    None,
    /// Must be a valid integer
    Int,
    /// Must be a valid UUID
    Uuid,
    /// Must contain only alphabetic characters
    Alpha,
    /// Must be a valid slug (alphanumeric + hyphens/underscores)
    Slug,
    /// Custom regex pattern
    Custom(Regex),
}

impl PartialEq for ParamConstraint {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ParamConstraint::None, ParamConstraint::None) => true,
            (ParamConstraint::Int, ParamConstraint::Int) => true,
            (ParamConstraint::Uuid, ParamConstraint::Uuid) => true,
            (ParamConstraint::Alpha, ParamConstraint::Alpha) => true,
            (ParamConstraint::Slug, ParamConstraint::Slug) => true,
            (ParamConstraint::Custom(regex1), ParamConstraint::Custom(regex2)) => {
                regex1.as_str() == regex2.as_str()
            }
            _ => false,
        }
    }
}

impl ParamConstraint {
    /// Parse constraint from string (e.g., "int", "uuid", "alpha")
    pub fn from_str(s: &str) -> Result<Self, RoutePatternError> {
        match s {
            "int" => Ok(ParamConstraint::Int),
            "uuid" => Ok(ParamConstraint::Uuid),
            "alpha" => Ok(ParamConstraint::Alpha),
            "slug" => Ok(ParamConstraint::Slug),
            _ => {
                // Try to parse as regex
                match Regex::new(s) {
                    Ok(regex) => Ok(ParamConstraint::Custom(regex)),
                    Err(e) => Err(RoutePatternError::InvalidConstraint(
                        format!("Invalid regex pattern '{}': {}", s, e)
                    )),
                }
            }
        }
    }

    /// Validate a parameter value against this constraint
    pub fn validate(&self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        match self {
            ParamConstraint::None => true,
            ParamConstraint::Int => value.parse::<i64>().is_ok(),
            ParamConstraint::Uuid => uuid::Uuid::parse_str(value).is_ok(),
            ParamConstraint::Alpha => value.chars().all(|c| c.is_alphabetic()),
            ParamConstraint::Slug => value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
            ParamConstraint::Custom(regex) => regex.is_match(value),
        }
    }
}

/// A single path segment in a route pattern
#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    /// Static text segment
    Static(String),
    /// Parameter segment with optional constraint
    Parameter { 
        name: String, 
        constraint: ParamConstraint 
    },
    /// Catch-all segment (must be last)
    CatchAll { 
        name: String 
    },
}

/// Parsed route pattern with compiled segments
#[derive(Debug, Clone)]
pub struct RoutePattern {
    /// The original path string
    pub original_path: String,
    /// Parsed path segments
    pub segments: Vec<PathSegment>,
    /// Parameter names in order
    pub param_names: Vec<String>,
    /// Whether this pattern has a catch-all segment
    pub has_catch_all: bool,
    /// Number of static segments (for priority calculation)
    pub static_segments: usize,
}

impl RoutePattern {
    /// Parse a route pattern from a path string
    pub fn parse(path: &str) -> Result<Self, RoutePatternError> {
        let mut segments = Vec::new();
        let mut param_names = Vec::new();
        let mut has_catch_all = false;
        let mut static_segments = 0;
        let mut seen_params = std::collections::HashSet::new();

        let path_segments: Vec<&str> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        for (index, segment) in path_segments.iter().enumerate() {
            let segment = segment.trim();
            
            if segment.starts_with('{') && segment.ends_with('}') {
                // Parameter segment: {name} or {name:constraint}
                let param_def = &segment[1..segment.len()-1];
                let (name, constraint) = Self::parse_parameter_definition(param_def)?;
                
                // Check for duplicate parameters
                if seen_params.contains(&name) {
                    return Err(RoutePatternError::DuplicateParameter(name));
                }
                seen_params.insert(name.clone());
                
                segments.push(PathSegment::Parameter { 
                    name: name.clone(), 
                    constraint 
                });
                param_names.push(name);
                
            } else if segment.starts_with('*') {
                // Catch-all segment: *name
                if has_catch_all {
                    return Err(RoutePatternError::MultipleCatchAll);
                }
                
                // Catch-all must be the last segment
                if index != path_segments.len() - 1 {
                    return Err(RoutePatternError::CatchAllNotLast);
                }
                
                let name = segment[1..].to_string();
                if name.is_empty() {
                    return Err(RoutePatternError::InvalidSyntax(
                        "Catch-all segment must have a name".to_string()
                    ));
                }
                
                // Check for duplicate parameters
                if seen_params.contains(&name) {
                    return Err(RoutePatternError::DuplicateParameter(name));
                }
                seen_params.insert(name.clone());
                
                segments.push(PathSegment::CatchAll { name: name.clone() });
                param_names.push(name);
                has_catch_all = true;
                
            } else {
                // Static segment
                if segment.is_empty() {
                    return Err(RoutePatternError::InvalidSyntax(
                        "Empty path segments not allowed".to_string()
                    ));
                }
                segments.push(PathSegment::Static(segment.to_string()));
                static_segments += 1;
            }
        }

        Ok(RoutePattern {
            original_path: path.to_string(),
            segments,
            param_names,
            has_catch_all,
            static_segments,
        })
    }

    /// Parse parameter definition (e.g., "id", "id:int", "slug:alpha")
    fn parse_parameter_definition(param_def: &str) -> Result<(String, ParamConstraint), RoutePatternError> {
        if let Some(colon_pos) = param_def.find(':') {
            let name = param_def[..colon_pos].trim().to_string();
            let constraint_str = param_def[colon_pos + 1..].trim();
            
            if name.is_empty() {
                return Err(RoutePatternError::InvalidSyntax(
                    "Parameter name cannot be empty".to_string()
                ));
            }
            
            let constraint = ParamConstraint::from_str(constraint_str)?;
            Ok((name, constraint))
        } else {
            let name = param_def.trim().to_string();
            if name.is_empty() {
                return Err(RoutePatternError::InvalidSyntax(
                    "Parameter name cannot be empty".to_string()
                ));
            }
            Ok((name, ParamConstraint::None))
        }
    }

    /// Check if this pattern matches a given path
    pub fn matches(&self, path: &str) -> bool {
        let path_segments: Vec<&str> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut pattern_idx = 0;
        let mut path_idx = 0;

        while pattern_idx < self.segments.len() && path_idx < path_segments.len() {
            match &self.segments[pattern_idx] {
                PathSegment::Static(expected) => {
                    if expected != path_segments[path_idx] {
                        return false;
                    }
                    pattern_idx += 1;
                    path_idx += 1;
                }
                
                PathSegment::Parameter { constraint, .. } => {
                    if !constraint.validate(path_segments[path_idx]) {
                        return false;
                    }
                    pattern_idx += 1;
                    path_idx += 1;
                }
                
                PathSegment::CatchAll { .. } => {
                    // Catch-all matches everything remaining
                    return true;
                }
            }
        }

        // For exact match: all pattern segments consumed and all path segments consumed
        // For catch-all: pattern is consumed (catch-all handled above)
        pattern_idx == self.segments.len() && (path_idx == path_segments.len() || self.has_catch_all)
    }

    /// Extract parameter values from a path that matches this pattern
    pub fn extract_params(&self, path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        let path_segments: Vec<&str> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut pattern_idx = 0;
        let mut path_idx = 0;

        while pattern_idx < self.segments.len() && path_idx < path_segments.len() {
            match &self.segments[pattern_idx] {
                PathSegment::Static(_) => {
                    pattern_idx += 1;
                    path_idx += 1;
                }
                
                PathSegment::Parameter { name, .. } => {
                    params.insert(name.clone(), path_segments[path_idx].to_string());
                    pattern_idx += 1;
                    path_idx += 1;
                }
                
                PathSegment::CatchAll { name } => {
                    // Collect all remaining segments for catch-all
                    let remaining: Vec<&str> = path_segments[path_idx..].to_vec();
                    params.insert(name.clone(), remaining.join("/"));
                    break;
                }
            }
        }

        params
    }

    /// Calculate priority for route matching (lower = higher priority)
    /// 
    /// Priority system accounts for constraint specificity:
    /// - Static segment: 1 (highest priority)
    /// - Parameter with specific constraint (Int, Uuid): 5
    /// - Parameter with general constraint (Alpha, Slug): 8  
    /// - Parameter with custom regex constraint: 6 (between specific and general)
    /// - Parameter with no constraint: 10
    /// - Catch-all segment: 100 (lowest priority)
    pub fn priority(&self) -> usize {
        let mut priority = 0;
        
        for segment in &self.segments {
            match segment {
                PathSegment::Static(_) => {
                    priority += 1; // Highest priority - exact match
                }
                PathSegment::Parameter { constraint, .. } => {
                    priority += match constraint {
                        ParamConstraint::Int | ParamConstraint::Uuid => 5,    // Specific constraints
                        ParamConstraint::Custom(_) => 6,                      // Custom regex (medium-high)
                        ParamConstraint::Alpha | ParamConstraint::Slug => 8,  // General constraints  
                        ParamConstraint::None => 10,                          // No constraint (most general)
                    };
                }
                PathSegment::CatchAll { .. } => {
                    priority += 100; // Lowest priority - catches everything
                }
            }
        }
        
        priority
    }

    /// Check if this is a static route (no parameters or catch-all)
    pub fn is_static(&self) -> bool {
        self.segments.iter().all(|seg| matches!(seg, PathSegment::Static(_)))
    }
}

/// Unique identifier for a route
pub type RouteId = String;

/// Information about a matched route
#[derive(Debug, Clone)]
pub struct RouteMatch {
    pub route_id: RouteId,
    pub params: HashMap<String, String>,
}

/// A compiled route ready for matching
#[derive(Debug, Clone)]
pub struct CompiledRoute {
    pub id: RouteId,
    pub method: HttpMethod,
    pub pattern: RoutePattern,
    pub priority: usize,
}

impl CompiledRoute {
    pub fn new(id: RouteId, method: HttpMethod, pattern: RoutePattern) -> Self {
        let priority = pattern.priority();
        Self {
            id,
            method,
            pattern,
            priority,
        }
    }

    /// Check if this route matches the given method and path
    pub fn matches(&self, method: &HttpMethod, path: &str) -> bool {
        self.method == *method && self.pattern.matches(path)
    }

    /// Extract parameters from a matching path
    pub fn extract_params(&self, path: &str) -> HashMap<String, String> {
        self.pattern.extract_params(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_static_route() {
        let pattern = RoutePattern::parse("/users").unwrap();
        assert_eq!(pattern.segments.len(), 1);
        assert!(matches!(&pattern.segments[0], PathSegment::Static(s) if s == "users"));
        assert!(pattern.param_names.is_empty());
        assert!(!pattern.has_catch_all);
        assert_eq!(pattern.static_segments, 1);
    }

    #[test]
    fn test_parse_parameter_route() {
        let pattern = RoutePattern::parse("/users/{id}").unwrap();
        assert_eq!(pattern.segments.len(), 2);
        assert!(matches!(&pattern.segments[0], PathSegment::Static(s) if s == "users"));
        assert!(matches!(&pattern.segments[1], PathSegment::Parameter { name, constraint } 
            if name == "id" && matches!(constraint, ParamConstraint::None)));
        assert_eq!(pattern.param_names, vec!["id"]);
        assert!(!pattern.has_catch_all);
        assert_eq!(pattern.static_segments, 1);
    }

    #[test]
    fn test_parse_constrained_parameter() {
        let pattern = RoutePattern::parse("/users/{id:int}").unwrap();
        assert!(matches!(&pattern.segments[1], PathSegment::Parameter { name, constraint } 
            if name == "id" && matches!(constraint, ParamConstraint::Int)));
    }

    #[test]
    fn test_parse_catch_all_route() {
        let pattern = RoutePattern::parse("/files/*path").unwrap();
        assert_eq!(pattern.segments.len(), 2);
        assert!(matches!(&pattern.segments[1], PathSegment::CatchAll { name } if name == "path"));
        assert!(pattern.has_catch_all);
        assert_eq!(pattern.param_names, vec!["path"]);
    }

    #[test]
    fn test_invalid_patterns() {
        assert!(RoutePattern::parse("/users/{id}/files/*path/more").is_err()); // Catch-all not last
        assert!(RoutePattern::parse("/users/{id}/{id}").is_err()); // Duplicate parameter
        assert!(RoutePattern::parse("/users/{}").is_err()); // Empty parameter name
        assert!(RoutePattern::parse("/files/*").is_err()); // Empty catch-all name
    }

    #[test]
    fn test_pattern_matching() {
        let pattern = RoutePattern::parse("/users/{id}/posts/{slug}").unwrap();
        
        assert!(pattern.matches("/users/123/posts/hello-world"));
        assert!(!pattern.matches("/users/123/posts")); // Missing slug
        assert!(!pattern.matches("/users/123/posts/hello/world")); // Too many segments
        assert!(!pattern.matches("/posts/123/posts/hello")); // Wrong static segment
    }

    #[test]
    fn test_parameter_extraction() {
        let pattern = RoutePattern::parse("/users/{id}/posts/{slug}").unwrap();
        let params = pattern.extract_params("/users/123/posts/hello-world");
        
        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("slug"), Some(&"hello-world".to_string()));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_catch_all_extraction() {
        let pattern = RoutePattern::parse("/files/*path").unwrap();
        let params = pattern.extract_params("/files/docs/images/logo.png");
        
        assert_eq!(params.get("path"), Some(&"docs/images/logo.png".to_string()));
    }

    #[test]
    fn test_constraint_validation() {
        assert!(ParamConstraint::Int.validate("123"));
        assert!(!ParamConstraint::Int.validate("abc"));
        
        assert!(ParamConstraint::Alpha.validate("hello"));
        assert!(!ParamConstraint::Alpha.validate("hello123"));
        
        assert!(ParamConstraint::Slug.validate("hello-world_123"));
        assert!(!ParamConstraint::Slug.validate("hello world!"));
        
        // UUID validation
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        assert!(ParamConstraint::Uuid.validate(uuid_str));
        assert!(!ParamConstraint::Uuid.validate("not-a-uuid"));
    }

    #[test]
    fn test_pattern_priorities() {
        let static_pattern = RoutePattern::parse("/users").unwrap();
        let param_pattern = RoutePattern::parse("/users/{id}").unwrap();
        let catch_all_pattern = RoutePattern::parse("/users/*path").unwrap();
        let mixed_pattern = RoutePattern::parse("/api/v1/users/{id}/posts/{slug}").unwrap();
        
        assert!(static_pattern.priority() < param_pattern.priority());
        assert!(param_pattern.priority() < catch_all_pattern.priority());
        
        // Mixed pattern: 4 static + 2 unconstrained parameters = 4*1 + 2*10 = 24
        assert_eq!(mixed_pattern.priority(), 24);
    }

    #[test]
    fn test_constraint_based_priorities() {
        // Test that more specific constraints have higher priority (lower numbers)
        let static_route = RoutePattern::parse("/users/123").unwrap();
        let int_constraint = RoutePattern::parse("/users/{id:int}").unwrap();
        let custom_constraint = RoutePattern::parse("/users/{id:[0-9]+}").unwrap();
        let alpha_constraint = RoutePattern::parse("/users/{slug:alpha}").unwrap();
        let no_constraint = RoutePattern::parse("/users/{name}").unwrap();
        let catch_all = RoutePattern::parse("/users/*path").unwrap();
        
        // Priority order: static < int < custom < alpha < none < catch-all
        assert!(static_route.priority() < int_constraint.priority());
        assert!(int_constraint.priority() < custom_constraint.priority());
        assert!(custom_constraint.priority() < alpha_constraint.priority());
        assert!(alpha_constraint.priority() < no_constraint.priority());
        assert!(no_constraint.priority() < catch_all.priority());
        
        // Verify exact priority values
        assert_eq!(static_route.priority(), 2);     // 2 static segments: 2*1 = 2
        assert_eq!(int_constraint.priority(), 6);   // 1 static + 1 int param: 1*1 + 1*5 = 6
        assert_eq!(custom_constraint.priority(), 7); // 1 static + 1 custom param: 1*1 + 1*6 = 7
        assert_eq!(alpha_constraint.priority(), 9);  // 1 static + 1 alpha param: 1*1 + 1*8 = 9
        assert_eq!(no_constraint.priority(), 11);    // 1 static + 1 unconstrained param: 1*1 + 1*10 = 11
        assert_eq!(catch_all.priority(), 101);       // 1 static + 1 catch-all: 1*1 + 1*100 = 101
    }

    #[test]
    fn test_complex_priority_scenarios() {
        // Test realistic routing scenarios where order matters
        
        // Scenario 1: API versioning with different constraint specificity
        let api_v1_int = RoutePattern::parse("/api/v1/users/{id:int}").unwrap();
        let api_v1_uuid = RoutePattern::parse("/api/v1/users/{id:uuid}").unwrap();
        let api_v1_slug = RoutePattern::parse("/api/v1/users/{slug:alpha}").unwrap();
        let api_v1_any = RoutePattern::parse("/api/v1/users/{identifier}").unwrap();
        
        // More specific constraints should have higher priority
        assert!(api_v1_int.priority() == api_v1_uuid.priority()); // Both specific constraints
        assert!(api_v1_int.priority() < api_v1_slug.priority());  // Specific < general
        assert!(api_v1_slug.priority() < api_v1_any.priority());  // General < unconstrained
        
        // Scenario 2: Mixed static and dynamic routing
        let users_profile = RoutePattern::parse("/users/{id:int}/profile").unwrap();
        let users_posts = RoutePattern::parse("/users/{id:int}/posts/{post_id:int}").unwrap();
        let users_files = RoutePattern::parse("/users/{id:int}/files/*path").unwrap();
        
        // More static segments = higher priority
        assert!(users_profile.priority() < users_posts.priority()); // profile is more specific
        assert!(users_posts.priority() < users_files.priority());   // files has catch-all
        
        // Verify calculations
        // users_profile: 2 static + 1 int = 2*1 + 1*5 = 7
        assert_eq!(users_profile.priority(), 7);
        // users_posts: 2 static + 2 int = 2*1 + 2*5 = 12  
        assert_eq!(users_posts.priority(), 12);
        // users_files: 2 static + 1 int + 1 catch-all = 2*1 + 1*5 + 1*100 = 107
        assert_eq!(users_files.priority(), 107);
    }

    #[test]
    fn test_compiled_route_matching() {
        let pattern = RoutePattern::parse("/users/{id:int}").unwrap();
        let route = CompiledRoute::new("test".to_string(), HttpMethod::GET, pattern);
        
        assert!(route.matches(&HttpMethod::GET, "/users/123"));
        assert!(!route.matches(&HttpMethod::POST, "/users/123")); // Wrong method
        assert!(!route.matches(&HttpMethod::GET, "/users/abc")); // Constraint violation
    }
}