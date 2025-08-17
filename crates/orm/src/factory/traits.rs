//! Factory trait definitions and core abstractions

use std::collections::HashMap;
use std::sync::RwLock;
use serde_json::Value;
use once_cell::sync::Lazy;
use crate::error::OrmResult;

/// Trait for factory states that can modify model attributes
#[async_trait::async_trait]
pub trait FactoryState<T>: Send + Sync {
    /// Apply state modifications to the attributes
    async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()>;
    
    /// Get the name of this state for debugging
    fn state_name(&self) -> &'static str;
}

/// Trait for relationship factories
#[async_trait::async_trait]
pub trait RelationshipFactory<Parent, Related>: Send + Sync {
    /// Create related models for a parent
    async fn create_for_parent(
        &self,
        parent: &Parent,
        pool: &sqlx::Pool<sqlx::Postgres>,
    ) -> OrmResult<Vec<Related>>;
    
    /// Make related models without saving
    async fn make_for_parent(&self, parent: &Parent) -> OrmResult<Vec<Related>>;
    
    /// Get the relationship type
    fn relationship_type(&self) -> RelationshipType;
}

/// Types of relationships supported by factories
#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipType {
    HasOne,
    HasMany,
    BelongsTo,
    BelongsToMany,
}

/// Trait for models that can be created by factories
pub trait Factoryable: crate::model::Model {
    /// Get the factory type for this model
    type Factory: super::Factory<Self>;
    
    /// Create a new factory instance
    fn factory() -> Self::Factory;
}

/// Trait for batch operations
#[async_trait::async_trait]
pub trait BatchFactory<T>: Send + Sync {
    /// Create multiple instances efficiently
    async fn create_batch(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        count: usize,
    ) -> OrmResult<Vec<T>>;
    
    /// Make multiple instances efficiently
    async fn make_batch(&self, count: usize) -> OrmResult<Vec<T>>;
    
    /// Get optimal batch size for this factory
    fn optimal_batch_size(&self) -> usize {
        100
    }
}

/// Configuration for factory behavior
#[derive(Debug, Clone)]
pub struct FactoryConfig {
    /// Whether to validate models before saving
    pub validate_models: bool,
    /// Whether to use database transactions for batch operations
    pub use_transactions: bool,
    /// Maximum batch size for bulk operations
    pub max_batch_size: usize,
    /// Whether to generate realistic timestamps
    pub realistic_timestamps: bool,
    /// Seed for deterministic fake data generation
    pub seed: Option<u64>,
}

impl Default for FactoryConfig {
    fn default() -> Self {
        Self {
            validate_models: true,
            use_transactions: true,
            max_batch_size: 1000,
            realistic_timestamps: true,
            seed: None,
        }
    }
}

/// Global factory configuration
static FACTORY_CONFIG: Lazy<RwLock<FactoryConfig>> = Lazy::new(|| RwLock::new(FactoryConfig::default()));

/// Get a read guard for the global factory configuration.
///
/// # Panics
/// Panics if the lock is poisoned.
pub fn factory_config() -> std::sync::RwLockReadGuard<'static, FactoryConfig> {
    FACTORY_CONFIG.read().unwrap()
}

/// Set the global factory configuration
///
/// # Panics
/// Panics if the lock is poisoned.
pub fn set_factory_config(config: FactoryConfig) {
    *FACTORY_CONFIG.write().unwrap() = config;
}

/// Macro for easily implementing the Factory trait
#[macro_export]
macro_rules! impl_factory {
    ($factory:ident for $model:ty {
        definition: |$def_self:ident| $definition:block
    }) => {
        #[async_trait::async_trait]
        impl $crate::factory::Factory<$model> for $factory {
            fn new() -> Self {
                Self::default()
            }
            
            async fn definition(&self) -> $crate::error::OrmResult<std::collections::HashMap<String, serde_json::Value>> {
                let $def_self = self;
                Ok($definition)
            }
            
            async fn make(&self) -> $crate::error::OrmResult<$model> {
                let attributes = self.definition().await?;
                // TODO: Convert attributes to model
                Err($crate::error::OrmError::ValidationError("Factory make not implemented".to_string()))
            }
            
            async fn create(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> $crate::error::OrmResult<$model> {
                let model = self.make().await?;
                <$model as $crate::model::Model>::create(pool, model).await
            }
            
            async fn make_many(&self, count: usize) -> $crate::error::OrmResult<Vec<$model>> {
                let mut models = Vec::with_capacity(count);
                for _ in 0..count {
                    models.push(self.make().await?);
                }
                Ok(models)
            }
            
            async fn create_many(&self, pool: &sqlx::Pool<sqlx::Postgres>, count: usize) -> $crate::error::OrmResult<Vec<$model>> {
                let models = self.make_many(count).await?;
                let mut created_models = Vec::with_capacity(models.len());
                
                // TODO: Use batch operations for better performance
                for model in models {
                    let created = <$model as $crate::model::Model>::create(pool, model).await?;
                    created_models.push(created);
                }
                
                Ok(created_models)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone)]
    struct TestState {
        name: String,
    }

    #[async_trait::async_trait]
    impl FactoryState<()> for TestState {
        async fn apply(&self, attributes: &mut HashMap<String, Value>) -> OrmResult<()> {
            attributes.insert("state".to_string(), json!(self.name));
            Ok(())
        }

        fn state_name(&self) -> &'static str {
            "TestState"
        }
    }

    #[tokio::test]
    async fn test_factory_state_application() {
        let state = TestState {
            name: "active".to_string(),
        };
        
        let mut attributes = HashMap::new();
        attributes.insert("id".to_string(), json!(1));
        
        FactoryState::<()>::apply(&state, &mut attributes).await.unwrap();
        
        assert_eq!(attributes.get("state").unwrap(), &json!("active"));
        assert_eq!(state.state_name(), "TestState");
    }

    #[test]
    fn test_factory_config_defaults() {
        let config = FactoryConfig::default();
        
        assert!(config.validate_models);
        assert!(config.use_transactions);
        assert_eq!(config.max_batch_size, 1000);
        assert!(config.realistic_timestamps);
        assert!(config.seed.is_none());
    }

    #[test]
    fn test_relationship_type_variants() {
        let types = vec![
            RelationshipType::HasOne,
            RelationshipType::HasMany,
            RelationshipType::BelongsTo,
            RelationshipType::BelongsToMany,
        ];
        
        // Test that all variants exist and can be compared
        assert_eq!(types.len(), 4);
        assert!(types.contains(&RelationshipType::HasOne));
    }
}