//! # elif-http
//! 
//! HTTP server core for the elif.rs LLM-friendly web framework.
//! 
//! This crate provides the fundamental HTTP server functionality including:
//! - Axum-based HTTP server with async support
//! - Integration with the elif-core DI container
//! - Configuration management
//! - Graceful shutdown handling
//! - Health check endpoints

// pub mod server;
// pub mod simple_server;
pub mod minimal_server;
pub mod server_with_middleware;
// pub mod stateful_server;
pub mod simple_stateful_server;
pub mod config;
pub mod error;
pub mod tests;
pub mod routing;
pub mod request;
pub mod response;
pub mod json;
pub mod middleware;
pub mod controller;
pub mod database;

// pub use server::{HttpServer, HttpServerBuilder};
// pub use simple_server::SimpleHttpServer;
pub use minimal_server::MinimalHttpServer;
pub use server_with_middleware::MiddlewareHttpServer;
// pub use stateful_server::{StatefulHttpServer, StatefulHttpServerBuilder, AppState};
pub use simple_stateful_server::SimpleStatefulHttpServer;
pub use config::HttpConfig;
pub use error::{HttpError, HttpResult};

// Re-export routing types
pub use routing::{
    HttpMethod, RouteInfo, RouteRegistry,
    ElifRouter, Route, RouteBuilder,
    PathParams, RouteParam, ParamError, ParamType,
    RouteGroup, GroupBuilder,
};

// Re-export request/response types  
pub use request::{ElifRequest, RequestExtractor, ElifQuery, ElifPath, ElifState};
pub use response::{ElifResponse, ResponseBody, IntoElifResponse, ElifStatusCode, ElifHeaderMap};
pub use json::{ElifJson, JsonError, JsonResponse, ValidationErrors, ApiResponse};

// Re-export middleware types
pub use middleware::{
    Middleware, MiddlewarePipeline, ErrorHandlingMiddleware,
    logging::LoggingMiddleware,
    timing::{TimingMiddleware, RequestStartTime, format_duration},
};

// Re-export controller types
pub use controller::{Controller, BaseController, QueryParams, PaginationMeta};

// Re-export database types
pub use database::{DatabaseServiceProvider, create_database_pool, get_database_pool, get_named_database_pool};

// Framework-native types - Use these instead of raw Axum types
// Note: Use Router from routing module, not axum::Router
// Note: Use ElifJson instead of axum::Json
// Note: Use ElifResponse instead of axum::Response  
// Note: HTTP types are available through response/request modules when needed