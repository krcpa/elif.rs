use elif_core::ElifError;

/// Interactive configuration utilities
pub struct InteractiveConfig;

impl InteractiveConfig {
    /// Validate project structure
    pub fn validate_project() -> Result<bool, ElifError> {
        // Check if we're in a valid project directory
        if std::path::Path::new("Cargo.toml").exists() {
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Get recommended project settings
    pub fn get_recommended_settings() -> ProjectSettings {
        ProjectSettings {
            use_hot_reload: true,
            default_port: 3000,
            include_auth: false,
            include_database: true,
        }
    }
}

/// Project configuration settings
pub struct ProjectSettings {
    pub use_hot_reload: bool,
    pub default_port: u16,
    pub include_auth: bool,
    pub include_database: bool,
}