use elif_core_derive::injectable;
use elif_core::container::{Injectable, ServiceId, IocContainerBuilder, ServiceBinder};
use std::sync::Arc;

struct UserRepository {
    name: String,
}

impl UserRepository {
    pub fn new() -> Self {
        Self {
            name: "UserRepository".to_string(),
        }
    }
    
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

unsafe impl Send for UserRepository {}
unsafe impl Sync for UserRepository {}

struct EmailService {
    smtp_server: String,
}

impl EmailService {
    pub fn new() -> Self {
        Self {
            smtp_server: "localhost:587".to_string(),
        }
    }
    
    pub fn get_server(&self) -> &str {
        &self.smtp_server
    }
}

unsafe impl Send for EmailService {}
unsafe impl Sync for EmailService {}

struct MetricsCollector {
    enabled: bool,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self { enabled: true }
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

unsafe impl Send for MetricsCollector {}
unsafe impl Sync for MetricsCollector {}

#[injectable]
pub struct UserService {
    user_repo: Arc<UserRepository>,
    email_service: Arc<EmailService>,
    metrics: Option<Arc<MetricsCollector>>,
}

impl UserService {
    pub fn get_user_repo_name(&self) -> &str {
        self.user_repo.get_name()
    }
    
    pub fn get_email_server(&self) -> &str {
        self.email_service.get_server()
    }
    
    pub fn has_metrics(&self) -> bool {
        self.metrics.is_some()
    }
    
    pub fn is_metrics_enabled(&self) -> Option<bool> {
        self.metrics.as_ref().map(|m| m.is_enabled())
    }
}

#[tokio::test]
async fn test_injectable_macro_generates_correct_implementation() {
    // Test that the Injectable trait is properly implemented
    let dependencies = UserService::dependencies();
    
    // Should have 3 dependencies (including optional MetricsCollector)
    assert_eq!(dependencies.len(), 3);
    
    // Verify the dependency types
    assert!(dependencies.contains(&ServiceId::of::<UserRepository>()));
    assert!(dependencies.contains(&ServiceId::of::<EmailService>()));
    assert!(dependencies.contains(&ServiceId::of::<MetricsCollector>()));
}

#[tokio::test]
async fn test_injectable_works_with_ioc_container() {
    // Create IoC container and register services
    let mut builder = IocContainerBuilder::new();
    
    builder
        .bind_factory::<UserRepository, _, _>(|| Ok(UserRepository::new()))
        .bind_factory::<EmailService, _, _>(|| Ok(EmailService::new()))
        .bind_factory::<MetricsCollector, _, _>(|| Ok(MetricsCollector::new()));
    
    let container = builder.build().expect("Failed to build container");

    // Create UserService using Injectable trait - need to implement this API
    let user_service = UserService::create(&container).expect("Failed to create UserService");

    // Verify dependencies were injected correctly
    assert_eq!(user_service.get_user_repo_name(), "UserRepository");
    assert_eq!(user_service.get_email_server(), "localhost:587");
    assert!(user_service.has_metrics());
    assert_eq!(user_service.is_metrics_enabled(), Some(true));
}

#[tokio::test]
async fn test_injectable_with_missing_optional_dependency() {
    // Create IoC container without MetricsCollector
    let mut builder = IocContainerBuilder::new();
    
    builder
        .bind_factory::<UserRepository, _, _>(|| Ok(UserRepository::new()))
        .bind_factory::<EmailService, _, _>(|| Ok(EmailService::new()));
        // Note: MetricsCollector is not registered
    
    let container = builder.build().expect("Failed to build container");

    // Create UserService using Injectable trait
    let user_service = UserService::create(&container).expect("Failed to create UserService");

    // Verify required dependencies were injected, optional was not
    assert_eq!(user_service.get_user_repo_name(), "UserRepository");
    assert_eq!(user_service.get_email_server(), "localhost:587");
    assert!(!user_service.has_metrics()); // Optional dependency should be None
    assert_eq!(user_service.is_metrics_enabled(), None);
}