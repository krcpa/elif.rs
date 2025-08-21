use std::sync::Arc;
use async_trait::async_trait;

use crate::errors::CoreError;

/// Trait for services that need async initialization
#[async_trait]
pub trait AsyncInitializable: Send + Sync {
    /// Initialize the service asynchronously
    async fn initialize(&self) -> Result<(), CoreError>;
    
    /// Check if the service is initialized
    fn is_initialized(&self) -> bool {
        true // Default implementation assumes immediate initialization
    }
}

/// Trait for services that need proper disposal/cleanup
#[async_trait]
pub trait Disposable: Send + Sync {
    /// Dispose of the service and clean up resources
    async fn dispose(&self) -> Result<(), CoreError>;
}

/// Combined trait for services that support both async initialization and disposal
#[async_trait]
pub trait LifecycleManaged: AsyncInitializable + Disposable {}

// Blanket implementation for types that implement both traits
impl<T> LifecycleManaged for T where T: AsyncInitializable + Disposable {}

/// Service lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Service is registered but not yet created
    Registered,
    /// Service instance is created but not initialized
    Created,
    /// Service is initialized and ready for use
    Initialized,
    /// Service is being disposed
    Disposing,
    /// Service has been disposed
    Disposed,
}

/// Service lifecycle manager that tracks initialization and disposal
pub struct ServiceLifecycleManager {
    /// Services that need async initialization
    initializable_services: Vec<Arc<dyn AsyncInitializable>>,
    /// Services that need disposal (in reverse order of creation)
    disposable_services: Vec<Arc<dyn Disposable>>,
    /// Service type names for initializable services (parallel to initializable_services)
    initializable_service_types: Vec<String>,
    /// Current state of the lifecycle manager
    state: ServiceState,
    /// Optional handle for background disposal task
    disposal_handle: Option<tokio::task::JoinHandle<()>>,
}

impl std::fmt::Debug for ServiceLifecycleManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceLifecycleManager")
            .field("initializable_services_count", &self.initializable_services.len())
            .field("disposable_services_count", &self.disposable_services.len())
            .field("state", &self.state)
            .field("has_disposal_handle", &self.disposal_handle.is_some())
            .finish()
    }
}

impl ServiceLifecycleManager {
    /// Create a new service lifecycle manager
    pub fn new() -> Self {
        Self {
            initializable_services: Vec::new(),
            disposable_services: Vec::new(),
            initializable_service_types: Vec::new(),
            state: ServiceState::Registered,
            disposal_handle: None,
        }
    }
    
    /// Add a service that needs async initialization
    pub fn add_initializable<T: AsyncInitializable + 'static>(&mut self, service: Arc<T>) {
        self.initializable_services.push(service);
        self.initializable_service_types.push(std::any::type_name::<T>().to_string());
    }
    
    /// Add a service that needs disposal
    pub fn add_disposable<T: Disposable + 'static>(&mut self, service: Arc<T>) {
        self.disposable_services.push(service);
    }
    
    /// Add a service that needs both initialization and disposal
    pub fn add_lifecycle_managed<T: LifecycleManaged + 'static>(&mut self, service: Arc<T>) {
        let service_clone = service.clone();
        self.initializable_services.push(service_clone);
        self.initializable_service_types.push(std::any::type_name::<T>().to_string());
        self.disposable_services.push(service);
    }
    
    /// Initialize all registered services
    pub async fn initialize_all(&mut self) -> Result<(), CoreError> {
        if self.state != ServiceState::Registered {
            return Err(CoreError::InvalidServiceDescriptor {
                message: format!("Cannot initialize services in state: {:?}", self.state),
            });
        }
        
        self.state = ServiceState::Created;
        
        // Initialize services in registration order
        for (index, service) in self.initializable_services.iter().enumerate() {
            let service_type = self.initializable_service_types
                .get(index)
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            
            service.initialize().await.map_err(|e| CoreError::ServiceInitializationFailed {
                service_type: service_type.to_string(),
                source: Box::new(e),
            })?;
        }
        
        self.state = ServiceState::Initialized;
        Ok(())
    }
    
    /// Initialize services with timeout
    pub async fn initialize_all_with_timeout(
        &mut self, 
        timeout: std::time::Duration
    ) -> Result<(), CoreError> {
        let init_future = self.initialize_all();
        
        match tokio::time::timeout(timeout, init_future).await {
            Ok(result) => result,
            Err(_) => Err(CoreError::ServiceInitializationFailed {
                service_type: "timeout".to_string(),
                source: Box::new(CoreError::InvalidServiceDescriptor {
                    message: format!("Service initialization timed out after {:?}", timeout),
                }),
            }),
        }
    }
    
    /// Dispose all services in reverse order
    pub async fn dispose_all(&mut self) -> Result<(), CoreError> {
        if self.state == ServiceState::Disposed || self.state == ServiceState::Disposing {
            return Ok(()); // Already disposed or disposing
        }
        
        self.state = ServiceState::Disposing;
        
        // Dispose services in reverse order (LIFO)
        for service in self.disposable_services.iter().rev() {
            if let Err(e) = service.dispose().await {
                // Log error but continue disposing other services
                eprintln!("Error disposing service: {:?}", e);
            }
        }
        
        self.state = ServiceState::Disposed;
        self.disposal_handle = None; // Clear any handle
        Ok(())
    }
    
    /// Schedule disposal in the background (non-blocking)
    /// This is useful when you can't await in the current context (like Drop)
    pub fn schedule_disposal(&mut self) {
        if self.is_disposed() || self.disposal_handle.is_some() {
            return; // Already disposed or disposal scheduled
        }
        
        // Take ownership of the services to dispose
        let services = std::mem::take(&mut self.disposable_services);
        self.state = ServiceState::Disposing;
        
        // Spawn a background task to handle disposal
        let handle = tokio::spawn(async move {
            for service in services.iter().rev() {
                if let Err(e) = service.dispose().await {
                    eprintln!("Error disposing service in background: {:?}", e);
                }
            }
        });
        
        self.disposal_handle = Some(handle);
    }
    
    /// Wait for any scheduled disposal to complete
    pub async fn wait_for_disposal(&mut self) -> Result<(), CoreError> {
        if let Some(handle) = self.disposal_handle.take() {
            handle.await.map_err(|e| CoreError::SystemError {
                message: format!("Disposal task failed: {}", e),
                source: None,
            })?;
            self.state = ServiceState::Disposed;
        }
        Ok(())
    }
    
    /// Get the current lifecycle state
    pub fn state(&self) -> ServiceState {
        self.state
    }
    
    /// Check if all services are initialized
    pub fn is_initialized(&self) -> bool {
        self.state == ServiceState::Initialized
    }
    
    /// Check if services are disposed
    pub fn is_disposed(&self) -> bool {
        self.state == ServiceState::Disposed
    }
    
    /// Get the number of services requiring initialization
    pub fn initializable_count(&self) -> usize {
        self.initializable_services.len()
    }
    
    /// Get the number of services requiring disposal
    pub fn disposable_count(&self) -> usize {
        self.disposable_services.len()
    }
}

impl Default for ServiceLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ServiceLifecycleManager {
    fn drop(&mut self) {
        if !self.is_disposed() && !self.disposable_services.is_empty() {
            // Check if we're in a tokio runtime context
            if let Ok(_handle) = tokio::runtime::Handle::try_current() {
                // We're in a tokio context, schedule disposal
                self.schedule_disposal();
            } else {
                // No tokio runtime available, log a warning
                eprintln!(
                    "Warning: ServiceLifecycleManager dropped with {} undisposed services. \
                     No tokio runtime available for async disposal. \
                     Call dispose_all() explicitly before dropping.",
                    self.disposable_services.len()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[derive(Default)]
    struct TestService {
        initialized: AtomicBool,
        disposed: AtomicBool,
    }

    #[async_trait]
    impl AsyncInitializable for TestService {
        async fn initialize(&self) -> Result<(), CoreError> {
            self.initialized.store(true, Ordering::SeqCst);
            Ok(())
        }
        
        fn is_initialized(&self) -> bool {
            self.initialized.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Disposable for TestService {
        async fn dispose(&self) -> Result<(), CoreError> {
            self.disposed.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_lifecycle_manager_initialization() {
        let mut manager = ServiceLifecycleManager::new();
        let service = Arc::new(TestService::default());
        
        assert!(!service.is_initialized());
        manager.add_lifecycle_managed(service.clone());
        
        manager.initialize_all().await.unwrap();
        
        assert!(service.is_initialized());
        assert!(manager.is_initialized());
    }

    #[tokio::test]
    async fn test_lifecycle_manager_disposal() {
        let mut manager = ServiceLifecycleManager::new();
        let service = Arc::new(TestService::default());
        
        manager.add_lifecycle_managed(service.clone());
        manager.initialize_all().await.unwrap();
        
        assert!(!service.disposed.load(Ordering::SeqCst));
        
        manager.dispose_all().await.unwrap();
        
        assert!(service.disposed.load(Ordering::SeqCst));
        assert!(manager.is_disposed());
    }

    #[tokio::test]
    async fn test_initialization_timeout() {
        #[derive(Default)]
        struct SlowService;

        #[async_trait]
        impl AsyncInitializable for SlowService {
            async fn initialize(&self) -> Result<(), CoreError> {
                // Simulate slow initialization
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                Ok(())
            }
        }

        let mut manager = ServiceLifecycleManager::new();
        let service = Arc::new(SlowService::default());
        
        manager.add_initializable(service);
        
        // Should timeout
        let result = manager.initialize_all_with_timeout(
            std::time::Duration::from_millis(50)
        ).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_background_disposal() {
        let mut manager = ServiceLifecycleManager::new();
        let service = Arc::new(TestService::default());
        
        manager.add_lifecycle_managed(service.clone());
        manager.initialize_all().await.unwrap();
        
        assert!(!service.disposed.load(Ordering::SeqCst));
        
        // Schedule disposal in background
        manager.schedule_disposal();
        
        // Wait for it to complete
        manager.wait_for_disposal().await.unwrap();
        
        assert!(service.disposed.load(Ordering::SeqCst));
        assert!(manager.is_disposed());
    }
    
    #[tokio::test]
    async fn test_drop_with_runtime() {
        let service = Arc::new(TestService::default());
        
        {
            let mut manager = ServiceLifecycleManager::new();
            manager.add_lifecycle_managed(service.clone());
            manager.initialize_all().await.unwrap();
            
            // Drop the manager without calling dispose_all()
            // It should schedule disposal automatically
        }
        
        // Give background task time to run
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        // Service should be disposed by Drop
        assert!(service.disposed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_initialization_error_with_service_type() {
        #[derive(Default)]
        struct FailingService;

        #[async_trait]
        impl AsyncInitializable for FailingService {
            async fn initialize(&self) -> Result<(), CoreError> {
                Err(CoreError::InvalidServiceDescriptor {
                    message: "Test initialization failure".to_string(),
                })
            }
        }

        let mut manager = ServiceLifecycleManager::new();
        let service = Arc::new(FailingService::default());
        
        manager.add_initializable(service);
        
        // Should fail with proper service type name
        let result = manager.initialize_all().await;
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        
        if let CoreError::ServiceInitializationFailed { service_type, .. } = error {
            // Check that it contains the actual service type name
            assert!(service_type.contains("FailingService"));
            assert!(!service_type.eq("unknown"));
        } else {
            panic!("Expected ServiceInitializationFailed error, got: {:?}", error);
        }
    }
}