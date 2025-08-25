//! Examples demonstrating the new IoC container features

use crate::container::{IocContainerBuilder, ServiceBinder};
use crate::errors::CoreError;
use std::sync::Arc;

/// Example repository trait
pub trait UserRepository: Send + Sync {
    fn find_by_id(&self, id: u32) -> Option<String>;
    fn create(&self, name: &str) -> Result<u32, String>;
}

/// PostgreSQL implementation
#[derive(Default)]
pub struct PostgresUserRepository {
    connection_string: String,
}

unsafe impl Send for PostgresUserRepository {}
unsafe impl Sync for PostgresUserRepository {}

impl PostgresUserRepository {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }
}

impl UserRepository for PostgresUserRepository {
    fn find_by_id(&self, id: u32) -> Option<String> {
        Some(format!("User {} from {}", id, self.connection_string))
    }

    fn create(&self, _name: &str) -> Result<u32, String> {
        Ok(42) // Mock implementation
    }
}

/// Example service that depends on repository
#[derive(Default)]
pub struct UserService {
    // In a real implementation, this would be injected
}

unsafe impl Send for UserService {}
unsafe impl Sync for UserService {}

impl UserService {
    pub fn get_user(&self, id: u32) -> Option<String> {
        // This would use the injected repository
        Some(format!("User {}", id))
    }
}

/// Example demonstrating basic IoC container usage
pub fn basic_container_example() -> Result<(), CoreError> {
    let mut builder = IocContainerBuilder::new();

    // Bind services
    builder
        .bind_singleton::<PostgresUserRepository, PostgresUserRepository>()
        .bind::<UserService, UserService>();

    // Build container
    let container = builder.build()?;

    // Resolve services
    let repo = container.resolve::<PostgresUserRepository>()?;
    let user = repo.find_by_id(1);
    assert!(user.is_some());

    let service = container.resolve::<UserService>()?;
    let user_data = service.get_user(1);
    assert!(user_data.is_some());

    Ok(())
}

/// Example demonstrating named services
pub fn named_services_example() -> Result<(), CoreError> {
    let mut builder = IocContainerBuilder::new();

    // Bind multiple implementations with names
    builder
        .bind_named::<PostgresUserRepository, PostgresUserRepository>("primary")
        .bind_named::<PostgresUserRepository, PostgresUserRepository>("backup");

    let container = builder.build()?;

    // Resolve by name
    let primary_repo = container.resolve_named::<PostgresUserRepository>("primary")?;
    let backup_repo = container.resolve_named::<PostgresUserRepository>("backup")?;

    assert!(primary_repo.find_by_id(1).is_some());
    assert!(backup_repo.find_by_id(1).is_some());

    Ok(())
}

/// Example demonstrating factory-based services
pub fn factory_services_example() -> Result<(), CoreError> {
    let mut builder = IocContainerBuilder::new();

    // Bind service with factory
    builder.bind_factory::<PostgresUserRepository, _, _>(|| {
        Ok(PostgresUserRepository::new(
            "postgres://localhost/db".to_string(),
        ))
    });

    let container = builder.build()?;

    let repo = container.resolve::<PostgresUserRepository>()?;
    assert!(repo.find_by_id(1).is_some());

    Ok(())
}

/// Example demonstrating lifetime behaviors
pub fn lifetime_example() -> Result<(), CoreError> {
    let mut builder = IocContainerBuilder::new();

    // Singleton - same instance every time
    builder.bind_singleton::<UserService, UserService>();

    // Transient - new instance every time
    builder.bind_transient::<PostgresUserRepository, PostgresUserRepository>();

    let container = builder.build()?;

    // Singleton behavior
    let service1 = container.resolve::<UserService>()?;
    let service2 = container.resolve::<UserService>()?;
    assert!(Arc::ptr_eq(&service1, &service2));

    // Transient behavior
    let repo1 = container.resolve::<PostgresUserRepository>()?;
    let repo2 = container.resolve::<PostgresUserRepository>()?;
    assert!(!Arc::ptr_eq(&repo1, &repo2));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_container_example() {
        basic_container_example().unwrap();
    }

    #[test]
    fn test_named_services_example() {
        named_services_example().unwrap();
    }

    #[test]
    fn test_factory_services_example() {
        factory_services_example().unwrap();
    }

    #[test]
    fn test_lifetime_example() {
        lifetime_example().unwrap();
    }
}
