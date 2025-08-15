//! Relationship Loading Utilities - Handles the loading and caching of relationships

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use tokio::sync::RwLock;

use crate::error::{ModelError, ModelResult};
use crate::model::Model;

/// Trait for loading relationships dynamically
#[async_trait]
pub trait RelationshipLoader<T>: Send + Sync {
    /// Load the relationship from the database
    async fn load(&self, pool: &Pool<Postgres>) -> ModelResult<T>;
    
    /// Reload the relationship, bypassing cache
    async fn reload(&self, pool: &Pool<Postgres>) -> ModelResult<T>;
}

/// Cached relationship loader that implements lazy loading with caching
pub struct CachedRelationshipLoader<T> {
    /// The loader function
    loader_fn: Arc<dyn Fn(&Pool<Postgres>) -> Pin<Box<dyn Future<Output = ModelResult<T>> + Send>> + Send + Sync>,
    /// Cached value
    cache: Arc<RwLock<Option<T>>>,
    /// Whether the relationship has been loaded
    loaded: Arc<RwLock<bool>>,
}

use std::pin::Pin;
use std::future::Future;

impl<T> CachedRelationshipLoader<T> 
where
    T: Send + Sync,
{
    /// Create a new cached loader with a loading function
    pub fn new<F, Fut>(loader: F) -> Self 
    where
        F: Fn(&Pool<Postgres>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ModelResult<T>> + Send + 'static,
    {
        Self {
            loader_fn: Arc::new(move |pool| Box::pin(loader(pool))),
            cache: Arc::new(RwLock::new(None)),
            loaded: Arc::new(RwLock::new(false)),
        }
    }
}

#[async_trait]
impl<T> RelationshipLoader<T> for CachedRelationshipLoader<T>
where
    T: Send + Sync + Clone,
{
    async fn load(&self, pool: &Pool<Postgres>) -> ModelResult<T> {
        // Check if already loaded
        {
            let loaded = self.loaded.read().await;
            if *loaded {
                let cache = self.cache.read().await;
                if let Some(ref value) = *cache {
                    return Ok(value.clone());
                }
            }
        }

        // Load the relationship
        let result = (self.loader_fn)(pool).await?;
        
        // Cache the result
        {
            let mut cache = self.cache.write().await;
            *cache = Some(result.clone());
        }
        {
            let mut loaded = self.loaded.write().await;
            *loaded = true;
        }

        Ok(result)
    }

    async fn reload(&self, pool: &Pool<Postgres>) -> ModelResult<T> {
        // Clear cache and reload
        {
            let mut cache = self.cache.write().await;
            *cache = None;
        }
        {
            let mut loaded = self.loaded.write().await;
            *loaded = false;
        }

        self.load(pool).await
    }
}

/// Lazy loading wrapper for relationships
pub struct Lazy<T> {
    loader: Box<dyn RelationshipLoader<T>>,
    loaded: bool,
    value: Option<T>,
}

impl<T> Lazy<T> 
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new lazy relationship
    pub fn new<L>(loader: L) -> Self 
    where
        L: RelationshipLoader<T> + 'static,
    {
        Self {
            loader: Box::new(loader),
            loaded: false,
            value: None,
        }
    }

    /// Create a lazy relationship with a pre-loaded value
    pub fn loaded(value: T) -> Self {
        Self {
            loader: Box::new(NoOpLoader::new(value.clone())),
            loaded: true,
            value: Some(value),
        }
    }

}

impl<T> Lazy<T> 
where
    T: Send + Sync,
{
    /// Get the loaded value, loading if necessary
    pub async fn get(&mut self, pool: &Pool<Postgres>) -> ModelResult<&T> {
        if !self.loaded {
            self.load(pool).await?;
        }
        
        self.value
            .as_ref()
            .ok_or_else(|| ModelError::Database("Lazy relationship value not available".to_string()))
    }

    /// Load the relationship
    pub async fn load(&mut self, pool: &Pool<Postgres>) -> ModelResult<&T> {
        let value = self.loader.load(pool).await?;
        self.value = Some(value);
        self.loaded = true;
        
        self.value
            .as_ref()
            .ok_or_else(|| ModelError::Database("Failed to store lazy relationship value".to_string()))
    }

    /// Reload the relationship, bypassing cache
    pub async fn reload(&mut self, pool: &Pool<Postgres>) -> ModelResult<&T> {
        let value = self.loader.reload(pool).await?;
        self.value = Some(value);
        self.loaded = true;
        
        self.value
            .as_ref()
            .ok_or_else(|| ModelError::Database("Failed to store reloaded relationship value".to_string()))
    }
}

impl<T> Lazy<T> {

    /// Check if the relationship has been loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Take the loaded value, leaving None in its place
    pub fn take(&mut self) -> Option<T> {
        self.loaded = false;
        self.value.take()
    }

    /// Set a pre-loaded value
    pub fn set(&mut self, value: T) {
        self.value = Some(value);
        self.loaded = true;
    }

    /// Clear the cached value
    pub fn clear(&mut self) {
        self.value = None;
        self.loaded = false;
    }
}

/// No-op loader for pre-loaded values
struct NoOpLoader<T> {
    value: T,
}

impl<T> NoOpLoader<T> {
    fn new(value: T) -> Self {
        Self { value }
    }
}

#[async_trait]
impl<T> RelationshipLoader<T> for NoOpLoader<T>
where
    T: Send + Sync + Clone,
{
    async fn load(&self, _pool: &Pool<Postgres>) -> ModelResult<T> {
        Ok(self.value.clone())
    }

    async fn reload(&self, _pool: &Pool<Postgres>) -> ModelResult<T> {
        Ok(self.value.clone())
    }
}

/// Relationship cache for managing loaded relationships across models
pub struct RelationshipCache {
    /// Cache storage organized by model type, model id, and relationship name
    cache: Arc<RwLock<HashMap<String, HashMap<String, HashMap<String, serde_json::Value>>>>>,
}

impl RelationshipCache {
    /// Create a new relationship cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a relationship in the cache
    pub async fn store(&self, model_type: &str, model_id: &str, relation: &str, data: serde_json::Value) {
        let mut cache = self.cache.write().await;
        
        cache
            .entry(model_type.to_string())
            .or_insert_with(HashMap::new)
            .entry(model_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(relation.to_string(), data);
    }

    /// Retrieve a relationship from the cache
    pub async fn get(&self, model_type: &str, model_id: &str, relation: &str) -> Option<serde_json::Value> {
        let cache = self.cache.read().await;
        
        cache
            .get(model_type)?
            .get(model_id)?
            .get(relation)
            .cloned()
    }

    /// Check if a relationship is cached
    pub async fn contains(&self, model_type: &str, model_id: &str, relation: &str) -> bool {
        let cache = self.cache.read().await;
        
        cache
            .get(model_type)
            .and_then(|models| models.get(model_id))
            .and_then(|relations| relations.get(relation))
            .is_some()
    }

    /// Clear all cached relationships for a model instance
    pub async fn clear_model(&self, model_type: &str, model_id: &str) {
        let mut cache = self.cache.write().await;
        
        if let Some(models) = cache.get_mut(model_type) {
            models.remove(model_id);
        }
    }

    /// Clear all cached relationships for a model type
    pub async fn clear_model_type(&self, model_type: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(model_type);
    }

    /// Clear all cached relationships
    pub async fn clear_all(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        
        let model_types = cache.len();
        let total_models = cache.values().map(|m| m.len()).sum();
        let total_relationships = cache
            .values()
            .flat_map(|models| models.values())
            .map(|relations| relations.len())
            .sum();

        CacheStats {
            model_types,
            total_models,
            total_relationships,
        }
    }
}

impl Default for RelationshipCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub model_types: usize,
    pub total_models: usize,
    pub total_relationships: usize,
}

/// Global relationship cache instance
static RELATIONSHIP_CACHE: tokio::sync::OnceCell<RelationshipCache> = tokio::sync::OnceCell::const_new();

/// Get the global relationship cache
pub async fn get_relationship_cache() -> &'static RelationshipCache {
    RELATIONSHIP_CACHE
        .get_or_init(|| async { RelationshipCache::new() })
        .await
}