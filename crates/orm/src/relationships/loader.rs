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

/// Access pattern tracking for auto-loading optimization
#[derive(Debug, Clone)]
pub struct AccessPattern {
    /// Number of times this relationship has been accessed
    pub access_count: usize,
    /// Whether this relationship should be auto-loaded based on patterns
    pub should_auto_load: bool,
    /// Last access timestamp (for eviction policies)
    pub last_accessed: std::time::Instant,
}

impl Default for AccessPattern {
    fn default() -> Self {
        Self {
            access_count: 0,
            should_auto_load: false,
            last_accessed: std::time::Instant::now(),
        }
    }
}

/// Lazy loading wrapper for relationships
pub struct Lazy<T> {
    loader: Box<dyn RelationshipLoader<T>>,
    loaded: bool,
    value: Option<T>,
    /// Access pattern tracking for optimization
    access_pattern: Arc<RwLock<AccessPattern>>,
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
            access_pattern: Arc::new(RwLock::new(AccessPattern::default())),
        }
    }

    /// Create a lazy relationship with a pre-loaded value
    pub fn loaded(value: T) -> Self {
        Self {
            loader: Box::new(NoOpLoader::new(value.clone())),
            loaded: true,
            value: Some(value),
            access_pattern: Arc::new(RwLock::new(AccessPattern::default())),
        }
    }

}

impl<T> Lazy<T> 
where
    T: Send + Sync,
{
    /// Get the loaded value, loading if necessary
    pub async fn get(&mut self, pool: &Pool<Postgres>) -> ModelResult<&T> {
        // Track access pattern
        {
            let mut pattern = self.access_pattern.write().await;
            pattern.access_count += 1;
            pattern.last_accessed = std::time::Instant::now();
            
            // Auto-enable after multiple accesses
            if pattern.access_count >= 3 {
                pattern.should_auto_load = true;
            }
        }
        
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

    /// Get access pattern statistics
    pub async fn get_access_pattern(&self) -> AccessPattern {
        self.access_pattern.read().await.clone()
    }

    /// Check if this relationship should be auto-loaded based on access patterns
    pub async fn should_auto_load(&self) -> bool {
        self.access_pattern.read().await.should_auto_load
    }

    /// Force enable auto-loading for this relationship
    pub async fn enable_auto_load(&self) {
        let mut pattern = self.access_pattern.write().await;
        pattern.should_auto_load = true;
    }

    /// Disable auto-loading for this relationship
    pub async fn disable_auto_load(&self) {
        let mut pattern = self.access_pattern.write().await;
        pattern.should_auto_load = false;
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

/// Lazy HasOne relationship - wraps a single optional related model
pub type LazyHasOne<T> = Lazy<Option<T>>;

/// Lazy HasMany relationship - wraps a collection of related models  
pub type LazyHasMany<T> = Lazy<Vec<T>>;

/// Lazy BelongsTo relationship - wraps a single related model
pub type LazyBelongsTo<T> = Lazy<T>;

/// Helper trait to create lazy relationship loaders
pub trait LazyRelationshipBuilder<Parent, Related>
where
    Parent: Model + Send + Sync + Clone + 'static,
    Related: Model + Send + Sync + Clone + 'static,
{
    /// Create a lazy HasOne loader for a relationship
    fn lazy_has_one(parent: &Parent, foreign_key: String) -> LazyHasOne<Related> {
        let parent_id = parent.primary_key().map(|pk| pk.to_string()).unwrap_or_default();
        let loader = CachedRelationshipLoader::new(move |_pool| {
            let _foreign_key = foreign_key.clone();
            let _parent_id = parent_id.clone();
            async move {
                // This would typically execute a query like:
                // SELECT * FROM related_table WHERE foreign_key_field = parent_id LIMIT 1
                // For now, we return None as a placeholder
                Ok(None)
            }
        });
        Lazy::new(loader)
    }

    /// Create a lazy HasMany loader for a relationship
    fn lazy_has_many(parent: &Parent, foreign_key: String) -> LazyHasMany<Related> {
        let parent_id = parent.primary_key().map(|pk| pk.to_string()).unwrap_or_default();
        let loader = CachedRelationshipLoader::new(move |_pool| {
            let _foreign_key = foreign_key.clone();
            let _parent_id = parent_id.clone();
            async move {
                // This would typically execute a query like:
                // SELECT * FROM related_table WHERE foreign_key_field = parent_id
                // For now, we return an empty Vec as a placeholder
                Ok(Vec::<Related>::new())
            }
        });
        Lazy::new(loader)
    }

    /// Create a lazy BelongsTo loader for a relationship
    fn lazy_belongs_to(_child: &Related, _parent_id_field: String) -> LazyBelongsTo<Parent> {
        let parent_id = "placeholder_id".to_string(); // Would extract from child model
        let loader = CachedRelationshipLoader::new(move |_pool| {
            let _parent_id = parent_id.clone();
            async move {
                // This would typically execute a query like:
                // SELECT * FROM parent_table WHERE id = parent_id LIMIT 1
                // For now, this is a placeholder implementation
                Err(crate::error::ModelError::Database("Placeholder implementation".to_string()))
            }
        });
        Lazy::new(loader)
    }
}