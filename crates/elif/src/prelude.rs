//! # Prelude
//!
//! The prelude module provides convenient imports for common elif.rs functionality.
//!
//! ```rust
//! use elif::prelude::*;
//! ```

// Essential HTTP types
pub use crate::{HttpError, HttpResult};
pub use crate::{Request, Response, Router, Server};

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

// Bootstrap traits
pub use elif_http::AppBootstrap;
