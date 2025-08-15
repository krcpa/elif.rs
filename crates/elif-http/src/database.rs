//! Database Integration - Service providers for database connectivity
//! 
//! Provides service providers for PostgreSQL connection pooling and ORM integration
//! with the DI container system.

use std::sync::Arc;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use elif_core::{ServiceProvider, Container, ContainerBuilder};
use crate::{HttpError, HttpResult};

/// Database service provider for PostgreSQL connection pool
pub struct DatabaseServiceProvider {
    database_url: String,
    max_connections: u32,
    min_connections: u32,
    acquire_timeout: u64,
    service_name: String,
}

impl DatabaseServiceProvider {
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: 30,
            service_name: "database_pool".to_string(),
        }
    }

    pub fn with_max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = max_connections;
        self
    }

    pub fn with_min_connections(mut self, min_connections: u32) -> Self {
        self.min_connections = min_connections;
        self
    }

    pub fn with_acquire_timeout(mut self, timeout_seconds: u64) -> Self {
        self.acquire_timeout = timeout_seconds;
        self
    }

    pub fn with_service_name(mut self, service_name: String) -> Self {
        self.service_name = service_name;
        self
    }
}

impl ServiceProvider for DatabaseServiceProvider {
    fn name(&self) -> &'static str {
        "DatabaseServiceProvider"
    }
    
    fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, elif_core::ProviderError> {
        // For now, return the builder unchanged
        // In a real implementation, we would register the database pool here
        Ok(builder)
    }
    
    fn boot(&self, _container: &Container) -> Result<(), elif_core::ProviderError> {
        // For now, just log that the database provider is booted
        println!("✅ Database service provider booted");
        Ok(())
    }
}

/// Helper function to create a database pool directly
pub async fn create_database_pool(database_url: &str) -> Result<Arc<Pool<Postgres>>, HttpError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(database_url)
        .await
        .map_err(|e| HttpError::database_error(format!("Failed to create database pool: {}", e)))?;
    
    Ok(Arc::new(pool))
}

/// Helper function to get database pool from container (for future implementation)
pub async fn get_database_pool(container: &Container) -> Result<Arc<Pool<Postgres>>, String> {
    // For now, return an error since the container doesn't have service registry yet
    Err("Database pool not yet integrated with current Container implementation".to_string())
}

/// Helper function to get custom named database pool from container (for future implementation)  
pub async fn get_named_database_pool(
    container: &Container, 
    service_name: &str
) -> Result<Arc<Pool<Postgres>>, String> {
    // For now, return an error since the container doesn't have service registry yet
    Err(format!("Database pool '{}' not yet integrated with current Container implementation", service_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_service_provider_creation() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string());
        assert_eq!(provider.database_url, "postgresql://test");
        assert_eq!(provider.max_connections, 10);
        assert_eq!(provider.min_connections, 1);
        assert_eq!(provider.acquire_timeout, 30);
        assert_eq!(provider.service_name, "database_pool");
    }

    #[test]
    fn test_database_service_provider_configuration() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string())
            .with_max_connections(20)
            .with_min_connections(5)
            .with_acquire_timeout(60)
            .with_service_name("custom_db".to_string());

        assert_eq!(provider.max_connections, 20);
        assert_eq!(provider.min_connections, 5);
        assert_eq!(provider.acquire_timeout, 60);
        assert_eq!(provider.service_name, "custom_db");
    }

    #[test]
    fn test_provider_name() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string());
        assert_eq!(provider.name(), "DatabaseServiceProvider");
    }
}