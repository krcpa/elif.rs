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

// Core modules
pub mod foundation;
pub mod server;
pub mod config;
pub mod errors;
pub mod routing;
pub mod request;
pub mod response;
pub mod middleware;
pub mod handlers;
pub mod logging;
pub mod controller;
pub mod testing;
pub mod websocket;

// Feature-gated modules
#[cfg(feature = "auth")]
pub mod auth;

// Main server API - NestJS-like experience
pub use server::Server;
pub use config::HttpConfig;
pub use errors::{HttpError, HttpResult};

// Re-export foundation types
pub use foundation::{GenericHandler, IntoElifResponse, RequestExtractor, BoxFuture};

// Re-export routing types
pub use routing::{
    HttpMethod, RouteInfo, RouteRegistry,
    ElifRouter, Route, RouteBuilder,
    PathParams, RouteParam, ParamError, ParamType,
    RouteGroup, GroupBuilder,
};

// Re-export request/response types  
pub use request::{ElifRequest, ElifQuery, ElifPath, ElifState};
pub use response::{ElifResponse, ResponseBody, ElifStatusCode, ElifHeaderMap};

// Re-export JSON handling
pub use response::{ElifJson, JsonError, JsonResponse, ValidationErrors, ApiResponse};

// Re-export middleware types
pub use middleware::{
    Middleware, MiddlewarePipeline, ErrorHandlingMiddleware, BoxFuture as MiddlewareBoxFuture,
    // Core middleware
    error_handler::{
        ErrorHandlerMiddleware, ErrorHandlerConfig, ErrorHandlerLayer,
        error_handler_middleware, error_handler_with_config, 
        error_handler_layer, error_handler_layer_with_config
    },
    logging::LoggingMiddleware,
    enhanced_logging::{EnhancedLoggingMiddleware, LoggingConfig as MiddlewareLoggingConfig, RequestContext},
    timing::{TimingMiddleware, RequestStartTime, format_duration},
    tracing::{TracingMiddleware, TracingConfig, RequestMetadata},
    timeout::{TimeoutMiddleware, TimeoutConfig, TimeoutInfo, apply_timeout},
    body_limit::{BodyLimitMiddleware, BodyLimitConfig, BodyLimitInfo, limit_body_size, limits},
};

// Re-export authentication types (if auth feature is enabled)
#[cfg(feature = "auth")]
pub use auth::{RequestAuthExt, AuthMiddleware};

// Re-export logging types
pub use logging::{
    LoggingConfig, init_logging, log_startup_info, log_shutdown_info, 
    structured,
};
// Re-export specific LoggingContext from context module
pub use logging::context::LoggingContext;

// Re-export controller types
pub use controller::{Controller, BaseController};
// Re-export from specific modules to avoid conflicts
pub use controller::pagination::{QueryParams, PaginationMeta};

// Re-export handler types
pub use handlers::{ElifHandler, elif_handler};

// Re-export testing utilities (for development and testing)
pub use testing::{
    TestUser, TestQuery, test_http_config, test_handler, test_json_handler, test_error_handler,
    create_test_container, TestContainerBuilder, TestServerBuilder,
    HttpAssertions, ErrorAssertions,
    get_test_port, test_socket_addr
};

// Re-export WebSocket types
pub use websocket::{
    // Core types
    WebSocketMessage, WebSocketError, WebSocketResult, ConnectionId, ConnectionState,
    WebSocketConfig, MessageType,
    // Connection management
    WebSocketConnection, ConnectionRegistry, ConnectionEvent,
    // Server and handler
    WebSocketServer, WebSocketHandler, WebSocketUpgrade, SimpleWebSocketHandler,
};

// Legacy compatibility re-exports
pub use response::ElifJson as Json;
pub use errors::HttpError as ElifError;

// Framework-native types - Use these instead of raw Axum types
// Note: Use Router from routing module, not axum::Router
// Note: Use ElifJson instead of axum::Json
// Note: Use ElifResponse instead of axum::Response  
// Note: HTTP types are available through response/request modules when needed