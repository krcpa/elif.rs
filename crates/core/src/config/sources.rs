/// Configuration source information for debugging and hot-reload
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Value loaded from environment variable
    EnvVar(String),
    /// Default value used
    Default(String),
    /// Value loaded from nested configuration
    Nested,
    /// Value loaded from file
    File(String),
    /// Value provided programmatically
    Programmatic,
}

impl ConfigSource {
    /// Check if source is environment variable
    pub fn is_env_var(&self) -> bool {
        matches!(self, ConfigSource::EnvVar(_))
    }
    
    /// Check if source is default value
    pub fn is_default(&self) -> bool {
        matches!(self, ConfigSource::Default(_))
    }
    
    /// Check if source is from file
    pub fn is_file(&self) -> bool {
        matches!(self, ConfigSource::File(_))
    }
    
    /// Get source description
    pub fn description(&self) -> String {
        match self {
            ConfigSource::EnvVar(var) => format!("Environment variable: {}", var),
            ConfigSource::Default(value) => format!("Default value: {}", value),
            ConfigSource::Nested => "Nested configuration".to_string(),
            ConfigSource::File(path) => format!("Configuration file: {}", path),
            ConfigSource::Programmatic => "Programmatically set".to_string(),
        }
    }
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}