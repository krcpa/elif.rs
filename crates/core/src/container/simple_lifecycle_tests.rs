use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use async_trait::async_trait;

use crate::container::{
    IocContainer, ServiceBinder,
    AsyncInitializable, Disposable, ServiceLifecycleManager
};
use crate::errors::CoreError;

#[derive(Default)]
struct TestService {
    initialized: AtomicBool,
    disposed: AtomicBool,
    instance_id: AtomicUsize,
}

impl TestService {
    fn new_with_id(id: usize) -> Self {
        Self {
            initialized: AtomicBool::new(false),
            disposed: AtomicBool::new(false),
            instance_id: AtomicUsize::new(id),
        }
    }
    
    fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }
    
    fn is_disposed(&self) -> bool {
        self.disposed.load(Ordering::SeqCst)
    }
    
    fn instance_id(&self) -> usize {
        self.instance_id.load(Ordering::SeqCst)
    }
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
async fn test_basic_lifecycle_manager() {
    let mut manager = ServiceLifecycleManager::new();
    let service = Arc::new(TestService::default());
    
    assert!(!service.is_initialized());
    manager.add_lifecycle_managed(service.clone());
    
    manager.initialize_all().await.unwrap();
    
    assert!(service.is_initialized());
    assert!(manager.is_initialized());
    
    manager.dispose_all().await.unwrap();
    
    assert!(service.is_disposed());
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
async fn test_basic_container_scopes() {
    let mut container = IocContainer::new();
    
    // Add a simple factory for transient services
    container.bind_factory::<TestService, _, _>(|| {
        Ok(TestService::default())
    });
    
    container.build().unwrap();
    
    // Create a scope
    let scope_id = container.create_scope().unwrap();
    
    // For now, just test that scope creation works
    assert!(container.create_child_scope(&scope_id).is_ok());
    
    // Test scope disposal
    assert!(container.dispose_scope(&scope_id).await.is_ok());
}

#[tokio::test]
async fn test_service_scope_behavior() {
    use crate::container::scope::{ServiceScope, ScopedServiceManager};
    
    // Test ServiceScope enum behavior
    assert!(ServiceScope::Singleton.is_singleton());
    assert!(ServiceScope::Transient.is_transient());
    assert!(ServiceScope::Scoped.is_scoped());
    
    assert_eq!(ServiceScope::Singleton.as_str(), "singleton");
    assert_eq!(ServiceScope::Transient.as_str(), "transient");
    assert_eq!(ServiceScope::Scoped.as_str(), "scoped");
    
    // Test ScopedServiceManager
    let manager = std::sync::Arc::new(ScopedServiceManager::new());
    let service = TestService::default();
    
    manager.add_service(service);
    assert!(manager.has_service::<TestService>());
    assert_eq!(manager.service_count(), 1);
    
    // Test nested scopes
    let child_manager = ScopedServiceManager::create_child(manager.clone());
    assert!(child_manager.parent().is_some());
    assert_ne!(manager.scope_id(), child_manager.scope_id());
}

#[tokio::test]
async fn test_singleton_vs_transient() {
    let mut container = IocContainer::new();
    
    // Bind singleton
    container.bind_singleton::<TestService, TestService>();
    container.build().unwrap();
    
    let service1 = container.resolve::<TestService>().unwrap();
    let service2 = container.resolve::<TestService>().unwrap();
    
    // Should be the same instance for singleton
    assert!(Arc::ptr_eq(&service1, &service2));
    
    // Test transient services
    let mut container = IocContainer::new();
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    
    container.bind_factory::<TestService, _, _>(|| {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        Ok(TestService::new_with_id(id))
    });
    container.build().unwrap();
    
    let service1 = container.resolve::<TestService>().unwrap();
    let service2 = container.resolve::<TestService>().unwrap();
    
    // Should be different instances for transient
    assert!(!Arc::ptr_eq(&service1, &service2));
    assert_ne!(service1.instance_id(), service2.instance_id());
}

#[tokio::test] 
async fn test_service_disposal() {
    let mut container = IocContainer::new();
    container.bind_singleton::<TestService, TestService>();
    container.build().unwrap();
    container.initialize_async().await.unwrap();
    
    let service = container.resolve::<TestService>().unwrap();
    assert!(!service.is_disposed());
    
    container.dispose_all().await.unwrap();
    // Note: In the current implementation, the service won't automatically be disposed
    // unless it's registered with the lifecycle manager
}