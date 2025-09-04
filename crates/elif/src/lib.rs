//! # elif.rs - The Rust web framework
//!
//! elif.rs is a modern web framework designed for both exceptional developer
//! experience and AI-native development.
//!
//! This is the main umbrella package that provides a unified API and convenient
//! imports for the entire elif.rs ecosystem.

// Re-export all sub-packages as modules
pub use elif_auth as auth;
pub use elif_cache as cache;
pub use elif_core as core;
pub use elif_http as http;
pub use elif_orm as orm;

// Re-export common types at root level for convenience
pub use elif_http::request::ElifRequest as Request;
pub use elif_http::response::ElifResponse as Response;
pub use elif_http::routing::ElifRouter as Router;
pub use elif_http::Server;
pub use elif_http::{HttpError, HttpResult};

// Re-export core functionality
pub use elif_core::{
    ApiError, ApiErrorResponse, AppConfig, AppConfigTrait, BaseModule, ConfigSource, Container,
    ContainerBuilder, CoreError, Environment, ErrorDefinition, Module, ModuleLoader,
    ModuleRegistry, ProviderRegistry, ServiceProvider, ServiceRegistry, ServiceScope,
};

// Prelude module for convenient imports
pub mod prelude;

// Macro functionality - re-export from elif-macros crate
pub use elif_macros as macros;
pub use elif_macros::main;

/// Current version of elif.rs
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Framework information
pub const FRAMEWORK_NAME: &str = "elif.rs";

/// Get framework version
pub fn version() -> &'static str {
    VERSION
}

/// Get framework name
pub fn name() -> &'static str {
    FRAMEWORK_NAME
}
