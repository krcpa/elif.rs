//! Database Integration - Service providers for database connectivity
//! 
//! Provides service providers for PostgreSQL connection pooling and ORM integration
//! with the DI container system.

use std::sync::Arc;
use std::time::Duration;
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

    /// Create a database pool using this provider's configuration
    pub async fn create_pool(&self) -> Result<Arc<Pool<Postgres>>, HttpError> {
        create_database_pool_with_config(
            &self.database_url,
            self.max_connections,
            self.min_connections,
            self.acquire_timeout
        ).await
    }

    /// Get the database URL (for diagnostic purposes)
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    /// Get the service name for this provider
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

impl ServiceProvider for DatabaseServiceProvider {
    fn name(&self) -> &'static str {
        "DatabaseServiceProvider"
    }
    
    fn register(&self, builder: ContainerBuilder) -> Result<ContainerBuilder, elif_core::ProviderError> {
        // Store database configuration for later pool creation
        // The actual pool will be created during boot phase
        tracing::debug!("Registering database service with URL: {}", 
            self.database_url.split('@').last().unwrap_or("unknown"));
        Ok(builder)
    }
    
    fn boot(&self, _container: &Container) -> Result<(), elif_core::ProviderError> {
        tracing::info!("✅ Database service provider booted successfully");
        tracing::debug!("Database pool configuration: max_connections={}, min_connections={}, acquire_timeout={}s", 
            self.max_connections, self.min_connections, self.acquire_timeout);
        Ok(())
    }
}

/// Helper function to create a database pool directly
pub async fn create_database_pool(database_url: &str) -> Result<Arc<Pool<Postgres>>, HttpError> {
    create_database_pool_with_config(database_url, 10, 1, 30).await
}

/// Helper function to create a database pool with custom configuration
pub async fn create_database_pool_with_config(
    database_url: &str,
    max_connections: u32,
    min_connections: u32,
    acquire_timeout_seconds: u64
) -> Result<Arc<Pool<Postgres>>, HttpError> {
    tracing::debug!("Creating database pool with config: max={}, min={}, timeout={}s", 
        max_connections, min_connections, acquire_timeout_seconds);
    
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout_seconds))
        .connect(database_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create database pool: {}", e);
            HttpError::database_error(format!("Failed to create database pool: {}", e))
        })?;
    
    tracing::info!("✅ Database pool created successfully with {} max connections", max_connections);
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

    #[test]
    fn test_database_service_provider_accessors() {
        let provider = DatabaseServiceProvider::new("postgresql://test_db".to_string())
            .with_service_name("custom_service".to_string());
        
        assert_eq!(provider.database_url(), "postgresql://test_db");
        assert_eq!(provider.service_name(), "custom_service");
    }

    #[test] 
    fn test_database_service_provider_defaults() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string());
        
        assert_eq!(provider.max_connections, 10);
        assert_eq!(provider.min_connections, 1);
        assert_eq!(provider.acquire_timeout, 30);
        assert_eq!(provider.service_name(), "database_pool");
    }

    #[test]
    fn test_database_service_provider_fluent_configuration() {
        let provider = DatabaseServiceProvider::new("postgresql://test".to_string())
            .with_max_connections(50)
            .with_min_connections(10)
            .with_acquire_timeout(120)
            .with_service_name("production_db".to_string());

        assert_eq!(provider.max_connections, 50);
        assert_eq!(provider.min_connections, 10);
        assert_eq!(provider.acquire_timeout, 120);
        assert_eq!(provider.service_name(), "production_db");
        assert_eq!(provider.database_url(), "postgresql://test");
    }
}