use crate::errors::CoreError;
use std::any::{Any, TypeId};
use std::fmt;

/// Core trait for framework components that can be registered and managed
pub trait FrameworkComponent: Send + Sync + 'static {
    /// Get the type name of this component
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Get the TypeId of this component
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// Trait for components that require initialization
pub trait Initializable {
    type Config;
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Initialize the component with given configuration
    async fn initialize(&mut self, config: Self::Config) -> Result<(), Self::Error>;
    
    /// Check if the component is initialized
    fn is_initialized(&self) -> bool;
}

/// Trait for components that need cleanup
pub trait Finalizable {
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Perform cleanup operations
    async fn finalize(&mut self) -> Result<(), Self::Error>;
}

/// Trait for components that can be validated
pub trait Validatable {
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Validate the component's current state
    fn validate(&self) -> Result<(), Self::Error>;
}

/// Trait for components that can be cloned safely
pub trait CloneableComponent: FrameworkComponent + Clone {}

impl<T> CloneableComponent for T where T: FrameworkComponent + Clone {}

/// Service trait for dependency injection
pub trait Service: FrameworkComponent {
    /// Service identifier - usually the type name
    fn service_id(&self) -> String {
        self.type_name().to_string()
    }
}

/// Factory trait for creating services
pub trait ServiceFactory: Send + Sync + 'static {
    type Service: Service;
    type Config;
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Create a new service instance
    async fn create_service(&self, config: Self::Config) -> Result<Self::Service, Self::Error>;
}

/// Marker trait for singleton services
pub trait Singleton: Service {}

/// Marker trait for transient services
pub trait Transient: Service {}

impl fmt::Debug for dyn FrameworkComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameworkComponent")
            .field("type_name", &self.type_name())
            .field("type_id", &self.type_id())
            .finish()
    }
}

impl fmt::Debug for dyn Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Service")
            .field("service_id", &self.service_id())
            .field("type_name", &self.type_name())
            .finish()
    }
}