//! HTTP routing system for elif.rs
//!
//! This module provides flexible HTTP routing with:
//! - Framework-independent route pattern matching
//! - Advanced parameter extraction with type conversion
//! - Route compilation and optimization
//! - Efficient route resolution
//! - Route groups and prefixes
//! - Route naming and URL generation
//! - Parameter validation and constraints

// Legacy modules (will be refactored to use new engine)
pub mod params;
pub mod router;
pub mod group;
pub mod versioned;

// New framework-independent routing engine
pub mod pattern;
pub mod matcher;
pub mod extraction;
pub mod compiler;

// Legacy exports (for backward compatibility)
pub use router::{Router as ElifRouter, Router, Route, RouteBuilder};
pub use params::{PathParams, RouteParam, ParamError, ParamType};
pub use group::{RouteGroup, GroupBuilder};
pub use versioned::{VersionedRouter, VersionedRouteBuilder, versioned_router, path_versioned_router, header_versioned_router};

// New engine exports
pub use pattern::{RoutePattern, PathSegment, ParamConstraint, CompiledRoute, RouteMatch};
pub use matcher::{RouteMatcher, RouteDefinition, MatcherStats, RouteMatcherBuilder};
pub use extraction::{ParameterExtractor, ExtractedParams, TypedExtractorBuilder, ExtractionError};
pub use compiler::{RouteCompiler, CompilableRoute, CompilationResult, CompilationStats, RouteCompilerBuilder};

use axum::http::Method;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// HTTP methods supported by the router
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    TRACE,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::PUT => write!(f, "PUT"),
            HttpMethod::DELETE => write!(f, "DELETE"),
            HttpMethod::PATCH => write!(f, "PATCH"),
            HttpMethod::HEAD => write!(f, "HEAD"),
            HttpMethod::OPTIONS => write!(f, "OPTIONS"),
            HttpMethod::TRACE => write!(f, "TRACE"),
        }
    }
}

impl From<Method> for HttpMethod {
    fn from(method: Method) -> Self {
        match method {
            Method::GET => HttpMethod::GET,
            Method::POST => HttpMethod::POST,
            Method::PUT => HttpMethod::PUT,
            Method::DELETE => HttpMethod::DELETE,
            Method::PATCH => HttpMethod::PATCH,
            Method::HEAD => HttpMethod::HEAD,
            Method::OPTIONS => HttpMethod::OPTIONS,
            Method::TRACE => HttpMethod::TRACE,
            _ => HttpMethod::GET, // Default fallback
        }
    }
}

impl From<HttpMethod> for Method {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::GET => Method::GET,
            HttpMethod::POST => Method::POST,
            HttpMethod::PUT => Method::PUT,
            HttpMethod::DELETE => Method::DELETE,
            HttpMethod::PATCH => Method::PATCH,
            HttpMethod::HEAD => Method::HEAD,
            HttpMethod::OPTIONS => Method::OPTIONS,
            HttpMethod::TRACE => Method::TRACE,
        }
    }
}

/// Route metadata for introspection and URL generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub name: Option<String>,
    pub path: String,
    pub method: HttpMethod,
    pub params: Vec<String>,
    pub group: Option<String>,
}

/// Route registry for managing all registered routes
#[derive(Debug, Default)]
pub struct RouteRegistry {
    routes: HashMap<String, RouteInfo>,
    named_routes: HashMap<String, String>, // name -> route_id
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            named_routes: HashMap::new(),
        }
    }

    pub fn register(&mut self, route_id: String, info: RouteInfo) {
        if let Some(ref name) = info.name {
            self.named_routes.insert(name.clone(), route_id.clone());
        }
        self.routes.insert(route_id, info);
    }

    pub fn get(&self, route_id: &str) -> Option<&RouteInfo> {
        self.routes.get(route_id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&RouteInfo> {
        self.named_routes.get(name)
            .and_then(|id| self.routes.get(id))
    }

    pub fn all_routes(&self) -> &HashMap<String, RouteInfo> {
        &self.routes
    }
}