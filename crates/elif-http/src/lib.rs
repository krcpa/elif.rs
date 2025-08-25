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
pub mod config;
pub mod controller;
pub mod errors;
pub mod foundation;
pub mod handlers;
pub mod logging;
pub mod middleware;
pub mod request;
pub mod response;
pub mod routing;
pub mod server;
pub mod testing;
pub mod websocket;

// Feature-gated modules
#[cfg(feature = "auth")]
pub mod auth;

// Main server API - NestJS-like experience
pub use config::HttpConfig;
pub use errors::{HttpError, HttpResult, VersionedError, VersionedErrorBuilder, VersionedErrorExt};
pub use server::Server;

// Re-export foundation types
pub use foundation::{BoxFuture, GenericHandler, IntoElifResponse, RequestExtractor};

// Re-export routing types
pub use routing::{
    header_versioned_router,
    path_versioned_router,
    versioned_router,
    ElifRouter,
    GroupBuilder,
    HttpMethod,
    ParamError,
    ParamType,
    PathParams,
    RouteBuilder,
    RouteGroup,
    RouteInfo,
    RouteParam as RoutingRouteParam,
    RouteRegistry,
    VersionedRouteBuilder,
    // Versioned routing
    VersionedRouter,
};

// Re-export request/response types
pub use request::{ElifMethod, ElifPath, ElifQuery, ElifRequest, ElifState};
pub use response::{
    ElifHeaderMap, ElifHeaderName, ElifHeaderValue, ElifResponse, ElifStatusCode, ResponseBody,
};

// Re-export JSON handling
pub use response::{ApiResponse, ElifJson, JsonError, JsonResponse, ValidationErrors};

// Re-export middleware types - V2 system is now the default
pub use middleware::{
    body_limit::{BodyLimitConfig, BodyLimitInfo, BodyLimitMiddleware},
    enhanced_logging::{
        EnhancedLoggingMiddleware, LoggingConfig as MiddlewareLoggingConfig, RequestContext,
    },
    // Core middleware
    error_handler::{
        error_handler, error_handler_with_config, ErrorHandlerConfig, ErrorHandlerMiddleware,
    },
    logging::LoggingMiddleware as LegacyLoggingMiddleware,
    timeout::{apply_timeout, TimeoutConfig, TimeoutInfo, TimeoutMiddleware},
    timing::{format_duration, RequestStartTime, TimingMiddleware},
    tracing::{RequestMetadata, TracingConfig, TracingMiddleware},
    // V2 Middleware System (default)
    v2::{
        LoggingMiddleware, Middleware, MiddlewarePipelineV2 as MiddlewarePipeline, Next,
        SimpleAuthMiddleware,
    },
    // Versioning middleware
    versioning::{
        default_versioning_middleware, versioning_layer, versioning_middleware, ApiVersion,
        RequestVersionExt, VersionInfo, VersionStrategy, VersioningConfig, VersioningLayer,
        VersioningMiddleware, VersioningService,
    },
};

// Re-export authentication types (if auth feature is enabled)
#[cfg(feature = "auth")]
pub use auth::{AuthMiddleware, RequestAuthExt};

// Re-export logging types
pub use logging::{init_logging, log_shutdown_info, log_startup_info, structured, LoggingConfig};
// Re-export specific LoggingContext from context module
pub use logging::context::LoggingContext;

// Re-export controller types
pub use controller::{
    BaseController, Controller, ControllerRoute, ElifController, RouteParam as ControllerRouteParam,
};
// Re-export from specific modules to avoid conflicts
pub use controller::pagination::{PaginationMeta, QueryParams};

// Re-export derive macros (if derive feature is enabled)
#[cfg(feature = "derive")]
pub use elif_http_derive::{
    body, controller, delete, get, group, head, middleware, options, param, patch, post, put,
    resource, routes,
};

// Re-export handler types
pub use handlers::{elif_handler, ElifHandler};

// Re-export testing utilities (for development and testing)
pub use testing::{
    create_test_container, get_test_port, test_error_handler, test_handler, test_http_config,
    test_json_handler, test_socket_addr, ErrorAssertions, HttpAssertions, TestContainerBuilder,
    TestQuery, TestServerBuilder, TestUser,
};

// Re-export WebSocket types
pub use websocket::{
    ConnectionEvent,
    ConnectionId,
    ConnectionRegistry,
    ConnectionState,
    MessageType,
    SimpleWebSocketHandler,
    WebSocketConfig,
    // Connection management
    WebSocketConnection,
    WebSocketError,
    WebSocketHandler,
    // Core types
    WebSocketMessage,
    WebSocketResult,
    // Server and handler
    WebSocketServer,
    WebSocketUpgrade,
};

// Legacy compatibility re-exports
pub use errors::HttpError as ElifError;
pub use response::ElifJson as Json;

// Framework-native types - Use these instead of raw Axum types
// Note: Use Router from routing module, not axum::Router
// Note: Use ElifJson instead of axum::Json
// Note: Use ElifResponse instead of axum::Response
// Note: HTTP types are available through response/request modules when needed
