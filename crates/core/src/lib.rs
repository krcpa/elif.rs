pub mod foundation;
pub mod errors;
pub mod container;
pub mod modules;
pub mod config;
pub mod providers;
pub mod specs;

// Re-export key types for convenience
pub use foundation::*;
pub use errors::*;
pub use container::*;
pub use modules::*;
pub use config::*;
pub use providers::*;
pub use specs::*;

// Specific re-exports to maintain compatibility
pub use config::validation::ConfigError;
pub use config::sources::ConfigSource;
pub use config::app_config::{AppConfig, AppConfigTrait, Environment};

// Legacy re-exports for backward compatibility
pub use errors::CoreError as ElifError;
pub use container::Container;
pub use modules::{Module, ModuleRegistry, ModuleLoader};

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