//! AppBootstrap trait and bootstrap implementation for app modules

use crate::{bootstrap::AppBootstrapper, HttpError};
use async_trait::async_trait;

/// Trait for app modules that can bootstrap themselves
///
/// This trait is automatically implemented for modules marked with `#[module]`
/// that are designated as app modules (typically the root module).
#[async_trait]
pub trait AppBootstrap {
    /// Start the bootstrap process for this app module
    ///
    /// This method discovers all modules in the dependency tree, configures
    /// the DI container, registers all controllers, and returns an AppBootstrapper
    /// ready for server startup.
    ///
    /// Returns a `BootstrapResult<AppBootstrapper>` to allow proper error handling
    /// instead of panicking on configuration errors.
    fn bootstrap() -> BootstrapResult<AppBootstrapper>;
}

/// Error type for bootstrap operations
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("Module discovery failed: {message}")]
    ModuleDiscoveryFailed { message: String },
    
    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },
    
    #[error("Missing dependency: module '{module}' depends on '{dependency}' which is not registered")]
    MissingDependency { module: String, dependency: String },
    
    #[error("Container configuration failed: {message}")]
    ContainerConfigurationFailed { message: String },
    
    #[error("Route registration failed: {message}")]
    RouteRegistrationFailed { message: String },
    
    #[error("Server startup failed: {message}")]
    ServerStartupFailed { message: String },
    
    #[error("HTTP error during bootstrap: {0}")]
    HttpError(#[from] HttpError),
}

impl From<BootstrapError> for HttpError {
    fn from(error: BootstrapError) -> Self {
        match error {
            BootstrapError::HttpError(http_error) => http_error,
            _ => HttpError::InternalError { message: format!("Bootstrap failed: {}", error) },
        }
    }
}

/// Result type for bootstrap operations
pub type BootstrapResult<T> = Result<T, BootstrapError>;