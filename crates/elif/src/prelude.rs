//! # Prelude
//!
//! The prelude module provides convenient imports for common elif.rs functionality.
//!
//! ```rust
//! use elif::prelude::*;
//! ```

// Essential HTTP types
pub use crate::{HttpError, HttpResult};
pub use crate::{Router, Server};
pub use elif_http::{ElifRequest as Request, ElifResponse as Response};
// Also export original types for macro compatibility
pub use elif_http::{ElifRequest, ElifResponse};

// Common traits - using correct exports
pub use elif_http::GenericHandler as Handler;
pub use elif_http::{IntoElifResponse, Middleware};
pub use elif_orm::Model;

// Core types
pub use crate::{
    ApiError, ApiErrorResponse, AppConfig, AppConfigTrait, BaseModule, ConfigSource, Container,
    ContainerBuilder, CoreError, Environment, ErrorDefinition, Module, ModuleLoader,
    ModuleRegistry, ProviderRegistry, ServiceProvider, ServiceRegistry, ServiceScope,
};

// Utility functions - check if these exist or create aliases
// pub use elif_http::response::{response};
// pub use elif_http::request::{request};

// JSON helper
pub use serde_json::json;

// Common derives
pub use serde::{Deserialize, Serialize};

// Async traits
pub use async_trait::async_trait;

// Macros
pub use crate::main;
pub use elif_macros::bootstrap;
pub use crate::{controller, module};
pub use elif_http_derive::{get, post, put, delete, patch, head, options, param, body};

// Bootstrap traits and types
pub use elif_http::AppBootstrap;
pub use elif_http::bootstrap::AppBootstrapper;
