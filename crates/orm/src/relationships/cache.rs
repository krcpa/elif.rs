//! Relationship Cache - Memory-efficient caching for loaded relationships

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde_json::Value;

/// Configuration for relationship cache
#[derive(Debug, Clone)]
pub struct RelationshipCacheConfig {
    /// Maximum number of cached relationships per model type
    pub max_relationships_per_type: usize,
    /// Maximum memory usage in bytes (0 = unlimited)
    pub max_memory_bytes: usize,
    /// Time to live for cached relationships
    pub ttl: Option<Duration>,
    /// Whether to enable cache metrics
    pub enable_metrics: bool,
}

impl Default for RelationshipCacheConfig {
    fn default() -> Self {
        Self {
            max_relationships_per_type: 1000,
            max_memory_bytes: 50 * 1024 * 1024, // 50MB
            ttl: Some(Duration::from_secs(300)), // 5 minutes
            enable_metrics: true,
        }
    }
}

/// Cached relationship entry
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached data
    data: Value,
    /// When the entry was created
    created_at: Instant,
    /// When the entry was last accessed
    last_accessed: Instant,
    /// Estimated size in bytes
    size_bytes: usize,
}

impl CacheEntry {
    fn new(data: Value) -> Self {
        let now = Instant::now();
        let size_bytes = estimate_json_size(&data);
        
        Self {
            data,
            created_at: now,
            last_accessed: now,
            size_bytes,
        }
    }
    
    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
    
    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// Cache key for relationships
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CacheKey {
    model_type: String,
    model_id: String,
    relationship: String,
}

impl CacheKey {
    fn new(model_type: &str, model_id: &str, relationship: &str) -> Self {
        Self {
            model_type: model_type.to_string(),
            model_id: model_id.to_string(),
            relationship: relationship.to_string(),
        }
    }
}

/// Optimized relationship cache with simple HashMap storage
pub struct OptimizedRelationshipCache {
    /// Cache storage
    cache: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,
    /// Cache configuration
    config: RelationshipCacheConfig,
    /// Cache metrics
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl OptimizedRelationshipCache {
    /// Create a new optimized cache
    pub fn new(config: RelationshipCacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(RwLock::new(CacheMetrics::new())),
        }
    }

    /// Store a relationship in the cache
    pub async fn store(&self, model_type: &str, model_id: &str, relationship: &str, data: Value) -> bool {
        let key = CacheKey::new(model_type, model_id, relationship);
        let entry = CacheEntry::new(data);
        
        // Check memory limits
        if self.config.max_memory_bytes > 0 {
            let current_memory = self.get_memory_usage().await;
            if current_memory + entry.size_bytes > self.config.max_memory_bytes {
                // Evict some entries to make room
                self.evict_by_memory().await;
                
                // Check again after eviction
                let current_memory = self.get_memory_usage().await;
                if current_memory + entry.size_bytes > self.config.max_memory_bytes {
                    return false; // Could not make enough room
                }
            }
        }

        // Store the entry
        {
            let mut cache = self.cache.write().await;
            cache.insert(key, entry);
        }

        // Update metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.stores += 1;
        }

        true
    }

    /// Retrieve a relationship from the cache
    pub async fn get(&self, model_type: &str, model_id: &str, relationship: &str) -> Option<Value> {
        let key = CacheKey::new(model_type, model_id, relationship);
        
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(&key) {
            // Check if expired
            if let Some(ttl) = self.config.ttl {
                if entry.is_expired(ttl) {
                    cache.remove(&key);
                    
                    // Update metrics
                    if self.config.enable_metrics {
                        let mut metrics = self.metrics.write().await;
                        metrics.misses += 1;
                        metrics.expired += 1;
                    }
                    
                    return None;
                }
            }
            
            // Touch the entry and return data
            entry.touch();
            let data = entry.data.clone();
            
            // Update metrics
            if self.config.enable_metrics {
                let mut metrics = self.metrics.write().await;
                metrics.hits += 1;
            }
            
            Some(data)
        } else {
            // Update metrics
            if self.config.enable_metrics {
                let mut metrics = self.metrics.write().await;
                metrics.misses += 1;
            }
            
            None
        }
    }

    /// Check if a relationship is cached and not expired
    pub async fn contains(&self, model_type: &str, model_id: &str, relationship: &str) -> bool {
        self.get(model_type, model_id, relationship).await.is_some()
    }

    /// Remove a specific cached relationship
    pub async fn remove(&self, model_type: &str, model_id: &str, relationship: &str) -> bool {
        let key = CacheKey::new(model_type, model_id, relationship);
        
        let mut cache = self.cache.write().await;
        cache.remove(&key).is_some()
    }

    /// Clear all cached relationships for a model instance
    pub async fn clear_model(&self, model_type: &str, model_id: &str) {
        let mut cache = self.cache.write().await;
        
        // Collect keys to remove
        let keys_to_remove: Vec<CacheKey> = cache
            .iter()
            .filter_map(|(key, _)| {
                if key.model_type == model_type && key.model_id == model_id {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
        
        // Remove the keys
        for key in keys_to_remove {
            cache.remove(&key);
        }
    }

    /// Clear all cached relationships for a model type
    pub async fn clear_model_type(&self, model_type: &str) {
        let mut cache = self.cache.write().await;
        
        // Collect keys to remove
        let keys_to_remove: Vec<CacheKey> = cache
            .iter()
            .filter_map(|(key, _)| {
                if key.model_type == model_type {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
        
        // Remove the keys
        for key in keys_to_remove {
            cache.remove(&key);
        }
    }

    /// Clear all cached relationships
    pub async fn clear_all(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        
        // Reset metrics
        if self.config.enable_metrics {
            let mut metrics = self.metrics.write().await;
            *metrics = CacheMetrics::new();
        }
    }

    /// Get current memory usage in bytes
    pub async fn get_memory_usage(&self) -> usize {
        let cache = self.cache.read().await;
        cache.iter().map(|(_, entry)| entry.size_bytes).sum()
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStatistics {
        let cache = self.cache.read().await;
        let metrics = self.metrics.read().await;
        
        let total_entries = cache.len();
        let memory_usage = cache.iter().map(|(_, entry)| entry.size_bytes).sum();
        
        // Count by model type
        let mut model_type_counts = HashMap::new();
        for (key, _) in cache.iter() {
            *model_type_counts.entry(key.model_type.clone()).or_insert(0) += 1;
        }

        CacheStatistics {
            total_entries,
            memory_usage_bytes: memory_usage,
            model_type_counts,
            hits: metrics.hits,
            misses: metrics.misses,
            stores: metrics.stores,
            expired: metrics.expired,
            hit_rate: if metrics.hits + metrics.misses > 0 {
                metrics.hits as f64 / (metrics.hits + metrics.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Evict entries to free memory
    async fn evict_by_memory(&self) {
        let target_memory = (self.config.max_memory_bytes as f64 * 0.8) as usize; // Target 80% of max
        
        let mut cache = self.cache.write().await;
        
        // Simple eviction strategy: remove oldest entries
        let mut entries: Vec<(CacheKey, CacheEntry)> = cache.drain().collect();
        entries.sort_by_key(|(_, entry)| entry.created_at);
        
        let mut current_memory = 0;
        for (key, entry) in entries {
            if current_memory + entry.size_bytes <= target_memory {
                current_memory += entry.size_bytes;
                cache.insert(key, entry);
            } else {
                break; // Skip remaining entries to stay under target
            }
        }
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) {
        if let Some(ttl) = self.config.ttl {
            let mut cache = self.cache.write().await;
            
            let keys_to_remove: Vec<CacheKey> = cache
                .iter()
                .filter_map(|(key, entry)| {
                    if entry.is_expired(ttl) {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect();
            
            let expired_count = keys_to_remove.len();
            
            for key in keys_to_remove {
                cache.remove(&key);
            }
            
            // Update metrics
            if self.config.enable_metrics && expired_count > 0 {
                let mut metrics = self.metrics.write().await;
                metrics.expired += expired_count;
            }
        }
    }
}

impl Default for OptimizedRelationshipCache {
    fn default() -> Self {
        Self::new(RelationshipCacheConfig::default())
    }
}

/// Cache metrics for monitoring and debugging
#[derive(Debug, Clone)]
struct CacheMetrics {
    hits: usize,
    misses: usize,
    stores: usize,
    expired: usize,
}

impl CacheMetrics {
    fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            stores: 0,
            expired: 0,
        }
    }
}

/// Public cache statistics
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    pub total_entries: usize,
    pub memory_usage_bytes: usize,
    pub model_type_counts: HashMap<String, usize>,
    pub hits: usize,
    pub misses: usize,
    pub stores: usize,
    pub expired: usize,
    pub hit_rate: f64,
}

/// Estimate the size of a JSON value in bytes
fn estimate_json_size(value: &Value) -> usize {
    match value {
        Value::Null => 4,
        Value::Bool(_) => 4,
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                8
            } else {
                8 // f64
            }
        }
        Value::String(s) => s.len() + 24, // String overhead
        Value::Array(arr) => {
            24 + arr.iter().map(estimate_json_size).sum::<usize>() // Vec overhead
        }
        Value::Object(obj) => {
            48 + obj.iter().map(|(k, v)| k.len() + estimate_json_size(v)).sum::<usize>() // HashMap overhead
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_cache_store_and_retrieve() {
        let cache = OptimizedRelationshipCache::default();
        let data = json!({"id": 1, "name": "Test"});
        
        // Store data
        let stored = cache.store("User", "1", "posts", data.clone()).await;
        assert!(stored);
        
        // Retrieve data
        let retrieved = cache.get("User", "1", "posts").await;
        assert_eq!(retrieved, Some(data));
    }
    
    #[tokio::test]
    async fn test_cache_expiration() {
        let mut config = RelationshipCacheConfig::default();
        config.ttl = Some(Duration::from_millis(100));
        
        let cache = OptimizedRelationshipCache::new(config);
        let data = json!({"id": 1, "name": "Test"});
        
        // Store data
        cache.store("User", "1", "posts", data.clone()).await;
        
        // Should be available immediately
        assert_eq!(cache.get("User", "1", "posts").await, Some(data));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be expired
        assert_eq!(cache.get("User", "1", "posts").await, None);
    }
    
    #[tokio::test]
    async fn test_memory_estimation() {
        let data = json!({
            "id": 1,
            "name": "Test User",
            "email": "test@example.com"
        });
        
        let size = estimate_json_size(&data);
        assert!(size > 50); // Should have reasonable size estimation
    }
}