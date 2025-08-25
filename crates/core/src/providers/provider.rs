use crate::container::{Container, ContainerBuilder};
use crate::errors::CoreError;

/// Provider error type
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Circular dependency detected in provider: {provider}")]
    CircularDependency { provider: String },

    #[error("Missing dependency '{dependency}' for provider '{provider}'")]
    MissingDependency {
        provider: String,
        dependency: String,
    },

    #[error("Provider registration failed: {message}")]
    RegistrationFailed { message: String },

    #[error("Provider boot failed: {message}")]
    BootFailed { message: String },

    #[error("Container error: {0}")]
    Container(#[from] CoreError),
}

/// Service provider trait for registering services and managing lifecycle
pub trait ServiceProvider: Send + Sync {
    /// Provider name for identification and dependency resolution
    fn name(&self) -> &'static str;

    /// Register services in the container builder
    /// This is called during the registration phase
    fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ProviderError>;

    /// Boot the provider after all services are registered
    /// This is called during the boot phase with access to the built container
    fn boot(&self, container: &Container) -> Result<(), ProviderError> {
        // Default implementation does nothing
        let _ = container; // Suppress unused parameter warning
        Ok(())
    }

    /// Provider dependencies (other providers that must be registered first)
    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Defer boot phase (useful for providers that need other providers to be booted first)
    fn defer_boot(&self) -> bool {
        false
    }

    /// Provider version for compatibility checking
    fn version(&self) -> Option<&'static str> {
        None
    }

    /// Provider description
    fn description(&self) -> Option<&'static str> {
        None
    }

    /// Check if this provider is optional
    fn is_optional(&self) -> bool {
        true
    }
}

/// Provider metadata for introspection
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub dependencies: Vec<String>,
    pub defer_boot: bool,
    pub is_optional: bool,
}

impl ProviderMetadata {
    /// Create metadata from a provider
    pub fn from_provider<P: ServiceProvider + ?Sized>(provider: &P) -> Self {
        Self {
            name: provider.name().to_string(),
            version: provider.version().map(|v| v.to_string()),
            description: provider.description().map(|d| d.to_string()),
            dependencies: provider
                .dependencies()
                .iter()
                .map(|d| d.to_string())
                .collect(),
            defer_boot: provider.defer_boot(),
            is_optional: provider.is_optional(),
        }
    }
}

/// Base provider implementation for common functionality
#[derive(Debug)]
pub struct BaseProvider {
    name: &'static str,
    version: Option<&'static str>,
    description: Option<&'static str>,
    dependencies: Vec<&'static str>,
    defer_boot: bool,
    is_optional: bool,
}

impl BaseProvider {
    /// Create a new base provider
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            version: None,
            description: None,
            dependencies: Vec::new(),
            defer_boot: false,
            is_optional: true,
        }
    }

    /// Set provider version
    pub fn with_version(mut self, version: &'static str) -> Self {
        self.version = Some(version);
        self
    }

    /// Set provider description
    pub fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    /// Set provider dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<&'static str>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Set defer boot flag
    pub fn with_defer_boot(mut self, defer_boot: bool) -> Self {
        self.defer_boot = defer_boot;
        self
    }

    /// Set if provider is optional
    pub fn with_optional(mut self, is_optional: bool) -> Self {
        self.is_optional = is_optional;
        self
    }
}

impl ServiceProvider for BaseProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, ProviderError> {
        // Base provider doesn't register anything by default
        Ok(builder)
    }

    fn dependencies(&self) -> Vec<&'static str> {
        self.dependencies.clone()
    }

    fn defer_boot(&self) -> bool {
        self.defer_boot
    }

    fn version(&self) -> Option<&'static str> {
        self.version
    }

    fn description(&self) -> Option<&'static str> {
        self.description
    }

    fn is_optional(&self) -> bool {
        self.is_optional
    }
}

/// Macro to simplify provider creation
#[macro_export]
macro_rules! provider {
    (
        name: $name:expr,
        $(version: $version:expr,)?
        $(description: $description:expr,)?
        $(dependencies: [$($dep:expr),* $(,)?],)?
        $(defer_boot: $defer:expr,)?
        $(optional: $optional:expr,)?
        register: |$builder:ident| $register:block
        $(, boot: |$container:ident| $boot:block)?
    ) => {
        {
            struct CustomProvider;

            impl $crate::providers::ServiceProvider for CustomProvider {
                fn name(&self) -> &'static str {
                    $name
                }

                $(fn version(&self) -> Option<&'static str> {
                    Some($version)
                })?

                $(fn description(&self) -> Option<&'static str> {
                    Some($description)
                })?

                $(fn dependencies(&self) -> Vec<&'static str> {
                    vec![$($dep),*]
                })?

                $(fn defer_boot(&self) -> bool {
                    $defer
                })?

                $(fn is_optional(&self) -> bool {
                    $optional
                })?

                fn register(&self, $builder: $crate::container::ContainerBuilder)
                    -> Result<$crate::container::ContainerBuilder, $crate::providers::ProviderError>
                {
                    $register
                }

                $(fn boot(&self, $container: &$crate::container::Container)
                    -> Result<(), $crate::providers::ProviderError>
                {
                    $boot
                })?
            }

            CustomProvider
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let base_provider = BaseProvider::new("test_provider")
            .with_version("1.0.0")
            .with_description("A test provider")
            .with_dependencies(vec!["dependency1", "dependency2"])
            .with_defer_boot(true)
            .with_optional(false);

        let metadata = ProviderMetadata::from_provider(&base_provider);

        assert_eq!(metadata.name, "test_provider");
        assert_eq!(metadata.version, Some("1.0.0".to_string()));
        assert_eq!(metadata.description, Some("A test provider".to_string()));
        assert_eq!(metadata.dependencies, vec!["dependency1", "dependency2"]);
        assert!(metadata.defer_boot);
        assert!(!metadata.is_optional);
    }
}
