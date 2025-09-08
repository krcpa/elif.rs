pub mod bootstrap;
pub mod config;
pub mod container;
pub mod errors;
pub mod examples;
pub mod foundation;
pub mod modules;
pub mod providers;
pub mod specs;

// Re-export key types for convenience (specific exports to avoid ambiguity)
pub use bootstrap::{
    AutoConfigBuilder, ConfigError as BootstrapConfigError, ConfigurationRule, ContainerAutoConfig, ProviderConfigurator,
    ValidationReport as BootstrapValidationReport,
};
pub use errors::{ApiError, ApiErrorResponse, CoreError, ErrorDefinition};
pub use foundation::{
    Finalizable, FrameworkComponent, Initializable, LifecycleManager, LifecycleState,
};
// New IoC container exports (recommended for new projects)
pub use container::{IocContainer, IocContainerBuilder, ServiceBinder, ServiceStatistics};
// Legacy exports (deprecated - use IocContainer instead)
#[deprecated(since = "0.6.0", note = "Use IocContainer instead")]
pub use container::{Container, ContainerBuilder};
// Still active exports
pub use config::validation::ConfigError;
pub use config::{AppConfig, AppConfigTrait, ConfigSource, Environment};
pub use container::{ServiceRegistry, ServiceScope};
pub use modules::{BaseModule, Module, ModuleError, ModuleLoader, ModuleRegistry};
pub use providers::{ProviderLifecycleManager, ProviderRegistry, ServiceProvider};
pub use specs::{ApiSpec, OperationSpec, ResourceSpec, StorageSpec};

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
