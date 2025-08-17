//! Database Factory System
//! 
//! Provides a comprehensive factory system for creating test data and seeding databases
//! with realistic fake data generation and relationship support.

use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use crate::error::{OrmError, OrmResult};
use crate::model::Model;

pub mod traits;
pub mod fake_data;
pub mod states;
pub mod relationships;
pub mod seeder;

pub use traits::*;
pub use fake_data::*;
pub use states::*;
pub use relationships::*;
pub use seeder::*;

/// Core factory trait that all model factories must implement
#[async_trait::async_trait]
pub trait Factory<T: Model>: Send + Sync {
    /// Create a new factory instance
    fn new() -> Self where Self: Sized;
    
    /// Define the default attributes for the model
    async fn definition(&self) -> OrmResult<HashMap<String, Value>>;
    
    /// Create a single model instance without saving to database
    async fn make(&self) -> OrmResult<T>;
    
    /// Create and save a single model instance to database
    async fn create(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<T>;
    
    /// Create multiple model instances without saving
    async fn make_many(&self, count: usize) -> OrmResult<Vec<T>>;
    
    /// Create and save multiple model instances
    async fn create_many(&self, pool: &sqlx::Pool<sqlx::Postgres>, count: usize) -> OrmResult<Vec<T>>;
    
    /// Override specific attributes for this instance
    fn with_attributes(self, attributes: HashMap<String, Value>) -> FactoryBuilder<T, Self> 
    where 
        Self: Sized,
    {
        FactoryBuilder::new(self, attributes)
    }
    
    /// Apply a factory state
    fn state<S: FactoryState<T>>(self, state: S) -> StateBuilder<T, Self, S> 
    where 
        Self: Sized,
    {
        StateBuilder::new(self, state)
    }
}

/// Builder for factory instances with custom attributes
pub struct FactoryBuilder<T: Model, F: Factory<T>> {
    factory: F,
    attributes: HashMap<String, Value>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Model, F: Factory<T>> FactoryBuilder<T, F> {
    pub fn new(factory: F, attributes: HashMap<String, Value>) -> Self {
        Self {
            factory,
            attributes,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Add or override an attribute
    pub fn with(mut self, key: &str, value: Value) -> Self {
        self.attributes.insert(key.to_string(), value);
        self
    }
    
    /// Create model without saving
    pub async fn make(&self) -> OrmResult<T> {
        let mut base_attributes = self.factory.definition().await?;
        
        // Override with custom attributes
        for (key, value) in &self.attributes {
            base_attributes.insert(key.clone(), value.clone());
        }
        
        // TODO: Convert attributes to model instance
        // This will require integration with the model system
        Err(OrmError::Validation("Factory make not yet implemented".to_string()))
    }
    
    /// Create and save model
    pub async fn create(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<T> {
        let model = self.make().await?;
        // TODO: Save model using the create method
        T::create(pool, model).await
    }
}

/// Builder for applying factory states
pub struct StateBuilder<T: Model, F: Factory<T>, S: FactoryState<T>> {
    factory: F,
    state: S,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Model, F: Factory<T>, S: FactoryState<T>> StateBuilder<T, F, S> {
    pub fn new(factory: F, state: S) -> Self {
        Self {
            factory,
            state,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Apply state and create model
    pub async fn make(&self) -> OrmResult<T> {
        let mut attributes = self.factory.definition().await?;
        self.state.apply(&mut attributes).await?;
        
        // TODO: Convert attributes to model instance
        Err(OrmError::Validation("State make not yet implemented".to_string()))
    }
    
    /// Apply state, create and save model
    pub async fn create(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<T> {
        let model = self.make().await?;
        T::create(pool, model).await
    }
}

/// Factory registry for managing all model factories
#[derive(Default)]
pub struct FactoryRegistry {
    factories: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl FactoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register a factory for a model type
    pub fn register<T: Model, F: Factory<T> + 'static>(&mut self, factory: F) {
        self.factories.insert(T::table_name().to_string(), Box::new(factory));
    }
    
    /// Get a factory for a model type
    pub fn get<T: Model, F: Factory<T> + 'static>(&self) -> Option<&F> {
        self.factories
            .get(T::table_name())
            .and_then(|f| f.downcast_ref::<F>())
    }
    
    /// Create a model using the registered factory
    pub async fn create<T: Model>(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> OrmResult<T> 
    where
        T: 'static,
    {
        // TODO: Implementation depends on how we handle the generic constraint
        Err(OrmError::Validation("Registry create not yet implemented".to_string()))
    }
}

/// Global factory registry instance
static mut FACTORY_REGISTRY: Option<FactoryRegistry> = None;
static FACTORY_REGISTRY_INIT: std::sync::Once = std::sync::Once::new();

/// Get the global factory registry
pub fn factory_registry() -> &'static FactoryRegistry {
    unsafe {
        FACTORY_REGISTRY_INIT.call_once(|| {
            FACTORY_REGISTRY = Some(FactoryRegistry::new());
        });
        FACTORY_REGISTRY.as_ref().unwrap()
    }
}

/// Get mutable access to the global factory registry
pub fn factory_registry_mut() -> &'static mut FactoryRegistry {
    unsafe {
        FACTORY_REGISTRY_INIT.call_once(|| {
            FACTORY_REGISTRY = Some(FactoryRegistry::new());
        });
        FACTORY_REGISTRY.as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    // TODO: Add comprehensive tests once the core implementation is complete
    
    #[test]
    fn test_factory_registry_creation() {
        let registry = FactoryRegistry::new();
        assert_eq!(registry.factories.len(), 0);
    }
    
    #[test]
    fn test_factory_builder_creation() {
        // This test will be expanded once we have concrete factory implementations
        assert!(true);
    }
}