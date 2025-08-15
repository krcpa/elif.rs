//! # elif-http
//! 
//! HTTP server core for the elif.rs LLM-friendly web framework.
//! 
//! This crate provides a NestJS-like HTTP server experience with:
//! - Clean, intuitive API that abstracts Axum complexity
//! - Integration with the elif-core DI container
//! - Built-in middleware pipeline
//! - Graceful shutdown handling
//! - Health check endpoints
//! - Framework-native routing system

pub mod server;
pub mod config;
pub mod error;
pub mod tests;
pub mod integration_tests;
pub mod routing;
pub mod request;
pub mod response;
pub mod json;
pub mod middleware;
pub mod controller;
pub mod handler;
pub mod logging;

#[cfg(feature = "auth")]
pub mod auth;

// Main server API - NestJS-like experience
pub use server::Server;
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
    error_handler::{
        ErrorHandlerMiddleware, ErrorHandlerConfig, ErrorHandlerLayer,
        error_handler_middleware, error_handler_with_config, 
        error_handler_layer, error_handler_layer_with_config
    },
};

// Re-export authentication types (if auth feature is enabled)
#[cfg(feature = "auth")]
pub use auth::{RequestAuthExt, AuthMiddleware};

// Re-export additional middleware types
pub use middleware::{
    logging::LoggingMiddleware,
    enhanced_logging::{EnhancedLoggingMiddleware, LoggingConfig as MiddlewareLoggingConfig, RequestContext},
    timing::{TimingMiddleware, RequestStartTime, format_duration},
    tracing::{TracingMiddleware, TracingConfig, RequestMetadata},
    timeout::{TimeoutMiddleware, TimeoutConfig, TimeoutInfo, apply_timeout},
    body_limit::{BodyLimitMiddleware, BodyLimitConfig, BodyLimitInfo, limit_body_size, limits},
};

// Re-export structured logging types
pub use logging::{
    LoggingConfig, init_logging, log_startup_info, log_shutdown_info, 
    LoggingContext, structured,
};

// Re-export controller types
pub use controller::{Controller, BaseController, QueryParams, PaginationMeta};


// Re-export handler types
pub use handler::{ElifHandler, elif_handler};

// Framework-native types - Use these instead of raw Axum types
// Note: Use Router from routing module, not axum::Router
// Note: Use ElifJson instead of axum::Json
// Note: Use ElifResponse instead of axum::Response  
// Note: HTTP types are available through response/request modules when needed