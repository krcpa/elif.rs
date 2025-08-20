//! # Prelude
//! 
//! The prelude module provides convenient imports for common elif.rs functionality.
//! 
//! ```rust
//! use elif::prelude::*;
//! ```

// Essential HTTP types
pub use crate::{Server, Router, Request, Response};
pub use crate::{HttpResult, HttpError};

// Common traits - using correct exports
pub use elif_http::{IntoElifResponse, Middleware};
pub use elif_http::{GenericHandler as Handler};
pub use elif_orm::{Model};

// Core types
pub use crate::{
    Container, ContainerBuilder, ServiceRegistry, ServiceScope,
    Module, ModuleRegistry, ModuleLoader, BaseModule,
    AppConfigTrait, Environment, AppConfig, ConfigSource,
    ServiceProvider, ProviderRegistry,
    CoreError, ErrorDefinition, ApiError, ApiErrorResponse
};

// Utility functions - check if these exist or create aliases
// pub use elif_http::response::{response};
// pub use elif_http::request::{request};

// JSON helper
pub use serde_json::json;

// Common derives
pub use serde::{Serialize, Deserialize};

// Async traits
pub use async_trait::async_trait;