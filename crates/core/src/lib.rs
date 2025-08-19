pub mod foundation;
pub mod errors;
pub mod container;
pub mod modules;
pub mod config;
pub mod providers;
pub mod specs;

// Re-export key types for convenience (specific exports to avoid ambiguity)
pub use foundation::{FrameworkComponent, Initializable, Finalizable, LifecycleManager, LifecycleState};
pub use errors::{CoreError, ErrorDefinition, ApiError, ApiErrorResponse};
pub use container::{Container, ContainerBuilder, ServiceRegistry, ServiceScope};
pub use modules::{Module, ModuleRegistry, ModuleLoader, BaseModule, ModuleError};
pub use config::{AppConfigTrait, Environment, AppConfig, ConfigSource};
pub use config::validation::ConfigError;
pub use providers::{ServiceProvider, ProviderRegistry, ProviderLifecycleManager};
pub use specs::{ResourceSpec, ApiSpec, OperationSpec, StorageSpec};

// Legacy re-exports for backward compatibility
pub use errors::CoreError as ElifError;

/// Version information
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