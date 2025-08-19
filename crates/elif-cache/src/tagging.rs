//! Cache tagging and invalidation system
//! 
//! This module provides functionality to group cache entries by tags and 
//! perform batch invalidation operations.

use crate::{CacheBackend, CacheError, CacheResult, CacheTag, CacheKey};
use async_trait::async_trait;
use dashmap::DashMap;
use std::{
    collections::HashSet,
    time::Duration,
};
use serde::{Deserialize, Serialize};

/// Cache entry metadata with tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggedEntry {
    /// The actual cache key
    pub key: CacheKey,
    
    /// Tags associated with this entry
    pub tags: HashSet<CacheTag>,
    
    /// When this entry was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// When this entry expires (if any)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Tag registry that tracks which keys belong to which tags
#[async_trait]
pub trait TagRegistry: Send + Sync {
    /// Add tags to a cache key
    async fn tag_key(&self, key: &str, tags: &[&str]) -> CacheResult<()>;
    
    /// Remove tags from a cache key
    async fn untag_key(&self, key: &str, tags: &[&str]) -> CacheResult<()>;
    
    /// Get all keys associated with a tag
    async fn get_keys_by_tag(&self, tag: &str) -> CacheResult<Vec<String>>;
    
    /// Get all tags for a key
    async fn get_tags_for_key(&self, key: &str) -> CacheResult<Vec<String>>;
    
    /// Remove a key from all tags
    async fn remove_key(&self, key: &str) -> CacheResult<()>;
    
    /// Clear all tag mappings
    async fn clear(&self) -> CacheResult<()>;
}

/// In-memory tag registry implementation
#[derive(Debug)]
pub struct MemoryTagRegistry {
    /// Maps tag -> set of keys
    tag_to_keys: DashMap<CacheTag, HashSet<CacheKey>>,
    
    /// Maps key -> set of tags
    key_to_tags: DashMap<CacheKey, HashSet<CacheTag>>,
}

impl MemoryTagRegistry {
    pub fn new() -> Self {
        Self {
            tag_to_keys: DashMap::new(),
            key_to_tags: DashMap::new(),
        }
    }
}

impl Default for MemoryTagRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TagRegistry for MemoryTagRegistry {
    async fn tag_key(&self, key: &str, tags: &[&str]) -> CacheResult<()> {
        let key = key.to_string();
        
        for tag in tags {
            let tag = tag.to_string();
            
            // Add key to tag mapping
            self.tag_to_keys
                .entry(tag.clone())
                .or_insert_with(HashSet::new)
                .insert(key.clone());
            
            // Add tag to key mapping
            self.key_to_tags
                .entry(key.clone())
                .or_insert_with(HashSet::new)
                .insert(tag);
        }
        
        Ok(())
    }
    
    async fn untag_key(&self, key: &str, tags: &[&str]) -> CacheResult<()> {
        let key = key.to_string();
        
        for tag in tags {
            let tag = tag.to_string();
            
            // Remove key from tag mapping
            if let Some(mut tag_keys) = self.tag_to_keys.get_mut(&tag) {
                tag_keys.remove(&key);
                if tag_keys.is_empty() {
                    drop(tag_keys);
                    self.tag_to_keys.remove(&tag);
                }
            }
            
            // Remove tag from key mapping
            if let Some(mut key_tags) = self.key_to_tags.get_mut(&key) {
                key_tags.remove(&tag);
                if key_tags.is_empty() {
                    drop(key_tags);
                    self.key_to_tags.remove(&key);
                }
            }
        }
        
        Ok(())
    }
    
    async fn get_keys_by_tag(&self, tag: &str) -> CacheResult<Vec<String>> {
        if let Some(keys) = self.tag_to_keys.get(tag) {
            Ok(keys.iter().cloned().collect())
        } else {
            Ok(vec![])
        }
    }
    
    async fn get_tags_for_key(&self, key: &str) -> CacheResult<Vec<String>> {
        if let Some(tags) = self.key_to_tags.get(key) {
            Ok(tags.iter().cloned().collect())
        } else {
            Ok(vec![])
        }
    }
    
    async fn remove_key(&self, key: &str) -> CacheResult<()> {
        let key = key.to_string();
        
        // Get all tags for this key
        if let Some((_, tags)) = self.key_to_tags.remove(&key) {
            // Remove key from all tag mappings
            for tag in tags {
                if let Some(mut tag_keys) = self.tag_to_keys.get_mut(&tag) {
                    tag_keys.remove(&key);
                    if tag_keys.is_empty() {
                        drop(tag_keys);
                        self.tag_to_keys.remove(&tag);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn clear(&self) -> CacheResult<()> {
        self.tag_to_keys.clear();
        self.key_to_tags.clear();
        Ok(())
    }
}

/// Cache backend wrapper that adds tagging support
pub struct TaggedCache<B, R>
where
    B: CacheBackend,
    R: TagRegistry,
{
    backend: B,
    registry: R,
}

impl<B, R> TaggedCache<B, R>
where
    B: CacheBackend,
    R: TagRegistry,
{
    pub fn new(backend: B, registry: R) -> Self {
        Self { backend, registry }
    }
    
    /// Put a value in cache with tags
    pub async fn put_with_tags(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<Duration>,
        tags: &[&str],
    ) -> CacheResult<()> {
        // Store in backend first
        self.backend.put(key, value, ttl).await?;
        
        // Then update tag registry
        if !tags.is_empty() {
            self.registry.tag_key(key, tags).await?;
        }
        
        Ok(())
    }
    
    /// Forget keys by tag
    pub async fn forget_by_tag(&self, tag: &str) -> CacheResult<Vec<String>> {
        let keys = self.registry.get_keys_by_tag(tag).await?;
        
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        
        // Remove keys individually to track which ones were actually removed
        let mut removed_keys = Vec::new();
        for key in keys {
            // Check if this specific key was removed
            let was_removed = self.backend.forget(&key).await?;
            
            // Always remove from registry
            self.registry.remove_key(&key).await?;
            
            if was_removed {
                removed_keys.push(key);
            }
        }
        
        Ok(removed_keys)
    }
    
    /// Forget keys by multiple tags (union)
    pub async fn forget_by_tags(&self, tags: &[&str]) -> CacheResult<Vec<String>> {
        let mut all_keys = HashSet::new();
        
        // Collect all keys from all tags
        for tag in tags {
            let keys = self.registry.get_keys_by_tag(tag).await?;
            all_keys.extend(keys);
        }
        
        if all_keys.is_empty() {
            return Ok(Vec::new());
        }
        
        // Remove keys individually to track which ones were actually removed
        let mut removed_keys = Vec::new();
        for key in all_keys {
            // Check if this specific key was removed
            let was_removed = self.backend.forget(&key).await?;
            
            // Always remove from registry
            self.registry.remove_key(&key).await?;
            
            if was_removed {
                removed_keys.push(key);
            }
        }
        
        Ok(removed_keys)
    }
    
    /// Get keys associated with a tag
    pub async fn keys_by_tag(&self, tag: &str) -> CacheResult<Vec<String>> {
        self.registry.get_keys_by_tag(tag).await
    }
    
    /// Get tags for a key
    pub async fn tags_for_key(&self, key: &str) -> CacheResult<Vec<String>> {
        self.registry.get_tags_for_key(key).await
    }
    
    /// Tag an existing cache key
    pub async fn tag_existing(&self, key: &str, tags: &[&str]) -> CacheResult<()> {
        // Check if key exists in cache
        if self.backend.exists(key).await? {
            self.registry.tag_key(key, tags).await
        } else {
            Err(CacheError::KeyNotFound(key.to_string()))
        }
    }
    
    /// Remove tags from an existing key
    pub async fn untag(&self, key: &str, tags: &[&str]) -> CacheResult<()> {
        self.registry.untag_key(key, tags).await
    }
    
    /// Get cache statistics with tag information
    pub async fn tagged_stats(&self) -> CacheResult<TaggedCacheStats> {
        let base_stats = self.backend.stats().await?;
        
        // For now, we don't expose internal registry details
        // In a real implementation, we'd add methods to TagRegistry trait
        Ok(TaggedCacheStats {
            base_stats,
            total_tags: 0,
            tagged_keys: 0,
        })
    }
}

/// Statistics for tagged cache
#[derive(Debug, Clone)]
pub struct TaggedCacheStats {
    pub base_stats: crate::CacheStats,
    pub total_tags: u64,
    pub tagged_keys: u64,
}

#[async_trait]
impl<B, R> CacheBackend for TaggedCache<B, R>
where
    B: CacheBackend,
    R: TagRegistry,
{
    async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
        self.backend.get(key).await
    }
    
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> CacheResult<()> {
        self.backend.put(key, value, ttl).await
    }
    
    async fn forget(&self, key: &str) -> CacheResult<bool> {
        let result = self.backend.forget(key).await?;
        
        if result {
            // Remove from tag registry as well
            self.registry.remove_key(key).await?;
        }
        
        Ok(result)
    }
    
    async fn exists(&self, key: &str) -> CacheResult<bool> {
        self.backend.exists(key).await
    }
    
    async fn flush(&self) -> CacheResult<()> {
        // Clear backend first
        self.backend.flush().await?;
        
        // Then clear registry
        self.registry.clear().await?;
        
        Ok(())
    }
    
    async fn get_many(&self, keys: &[&str]) -> CacheResult<Vec<Option<Vec<u8>>>> {
        self.backend.get_many(keys).await
    }
    
    async fn put_many(&self, entries: &[(&str, Vec<u8>, Option<Duration>)]) -> CacheResult<()> {
        self.backend.put_many(entries).await
    }
    
    async fn stats(&self) -> CacheResult<crate::CacheStats> {
        self.backend.stats().await
    }
}

/// High-level tagged cache API
pub struct TaggedCacheManager<B, R>
where
    B: CacheBackend,
    R: TagRegistry,
{
    cache: TaggedCache<B, R>,
}

impl<B, R> TaggedCacheManager<B, R>
where
    B: CacheBackend,
    R: TagRegistry,
{
    pub fn new(backend: B, registry: R) -> Self {
        Self {
            cache: TaggedCache::new(backend, registry),
        }
    }
    
    /// Remember pattern with tags
    pub async fn remember_with_tags<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        tags: &[&str],
        compute: F,
    ) -> CacheResult<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        // Try to get from cache first
        if let Some(cached_bytes) = self.cache.get(key).await? {
            let value: T = serde_json::from_slice(&cached_bytes)
                .map_err(CacheError::Serialization)?;
            return Ok(value);
        }
        
        // Compute the value
        let value = compute().await;
        
        // Serialize and store with tags
        let bytes = serde_json::to_vec(&value).map_err(CacheError::Serialization)?;
        self.cache.put_with_tags(key, bytes, Some(ttl), tags).await?;
        
        Ok(value)
    }
    
    /// Invalidate cache by tags
    pub async fn invalidate_by_tags(&self, tags: &[&str]) -> CacheResult<u32> {
        let removed_keys = self.cache.forget_by_tags(tags).await?;
        Ok(removed_keys.len() as u32)
    }
    
    /// Tag management operations
    pub async fn add_tags(&self, key: &str, tags: &[&str]) -> CacheResult<()> {
        self.cache.tag_existing(key, tags).await
    }
    
    pub async fn remove_tags(&self, key: &str, tags: &[&str]) -> CacheResult<()> {
        self.cache.untag(key, tags).await
    }
    
    pub async fn get_key_tags(&self, key: &str) -> CacheResult<Vec<String>> {
        self.cache.tags_for_key(key).await
    }
    
    pub async fn get_tagged_keys(&self, tag: &str) -> CacheResult<Vec<String>> {
        self.cache.keys_by_tag(tag).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::MemoryBackend;
    use crate::config::CacheConfig;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_memory_tag_registry() {
        let registry = MemoryTagRegistry::new();
        
        // Tag a key
        registry.tag_key("user:1", &["users", "active"]).await.unwrap();
        registry.tag_key("user:2", &["users"]).await.unwrap();
        registry.tag_key("post:1", &["posts", "active"]).await.unwrap();
        
        // Test getting keys by tag
        let users = registry.get_keys_by_tag("users").await.unwrap();
        assert_eq!(users.len(), 2);
        assert!(users.contains(&"user:1".to_string()));
        assert!(users.contains(&"user:2".to_string()));
        
        let active = registry.get_keys_by_tag("active").await.unwrap();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"user:1".to_string()));
        assert!(active.contains(&"post:1".to_string()));
        
        // Test getting tags for key
        let tags = registry.get_tags_for_key("user:1").await.unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"users".to_string()));
        assert!(tags.contains(&"active".to_string()));
        
        // Test untagging
        registry.untag_key("user:1", &["active"]).await.unwrap();
        let tags = registry.get_tags_for_key("user:1").await.unwrap();
        assert_eq!(tags.len(), 1);
        assert!(tags.contains(&"users".to_string()));
        
        let active = registry.get_keys_by_tag("active").await.unwrap();
        assert_eq!(active.len(), 1);
        assert!(active.contains(&"post:1".to_string()));
        
        // Test removing key
        registry.remove_key("user:2").await.unwrap();
        let users = registry.get_keys_by_tag("users").await.unwrap();
        assert_eq!(users.len(), 1);
        assert!(users.contains(&"user:1".to_string()));
    }
    
    #[tokio::test]
    async fn test_tagged_cache() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let registry = MemoryTagRegistry::new();
        let cache = TaggedCache::new(backend, registry);
        
        // Put with tags
        cache.put_with_tags(
            "user:1", 
            b"user data".to_vec(), 
            Some(Duration::from_secs(60)),
            &["users", "active"]
        ).await.unwrap();
        
        cache.put_with_tags(
            "user:2", 
            b"user data 2".to_vec(), 
            Some(Duration::from_secs(60)),
            &["users"]
        ).await.unwrap();
        
        cache.put_with_tags(
            "post:1", 
            b"post data".to_vec(), 
            Some(Duration::from_secs(60)),
            &["posts", "active"]
        ).await.unwrap();
        
        // Test normal cache operations still work
        let data = cache.get("user:1").await.unwrap();
        assert_eq!(data, Some(b"user data".to_vec()));
        
        // Test tag-based queries
        let user_keys = cache.keys_by_tag("users").await.unwrap();
        assert_eq!(user_keys.len(), 2);
        
        let active_keys = cache.keys_by_tag("active").await.unwrap();
        assert_eq!(active_keys.len(), 2);
        
        // Test invalidation by tag
        let removed = cache.forget_by_tag("active").await.unwrap();
        assert_eq!(removed.len(), 2);
        
        // Verify items are gone
        assert_eq!(cache.get("user:1").await.unwrap(), None);
        assert_eq!(cache.get("post:1").await.unwrap(), None);
        assert_eq!(cache.get("user:2").await.unwrap(), Some(b"user data 2".to_vec()));
    }
    
    #[tokio::test]
    async fn test_tagged_cache_manager() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let registry = MemoryTagRegistry::new();
        let manager = TaggedCacheManager::new(backend, registry);
        
        let mut call_count = 0;
        
        // Test remember with tags
        let result = manager.remember_with_tags(
            "expensive:1",
            Duration::from_secs(60),
            &["expensive", "computations"],
            || async {
                call_count += 1;
                format!("result_{}", call_count)
            }
        ).await.unwrap();
        
        assert_eq!(result, "result_1");
        
        // Second call should use cache
        let result2 = manager.remember_with_tags(
            "expensive:1",
            Duration::from_secs(60),
            &["expensive", "computations"],
            || async {
                call_count += 1;
                format!("result_{}", call_count)
            }
        ).await.unwrap();
        
        assert_eq!(result2, "result_1");
        assert_eq!(call_count, 1); // Should not have been called again
        
        // Test invalidation
        let invalidated = manager.invalidate_by_tags(&["expensive"]).await.unwrap();
        assert_eq!(invalidated, 1);
        
        // Third call should compute again
        let result3 = manager.remember_with_tags(
            "expensive:1",
            Duration::from_secs(60),
            &["expensive", "computations"],
            || async {
                call_count += 1;
                format!("result_{}", call_count)
            }
        ).await.unwrap();
        
        assert_eq!(result3, "result_2");
        assert_eq!(call_count, 2);
    }
    
    #[tokio::test]
    async fn test_forget_by_tag_selective_removal() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let registry = MemoryTagRegistry::new();
        let cache = TaggedCache::new(backend, registry);
        
        // Put some data with same tags
        cache.put_with_tags("key1", b"data1".to_vec(), Some(Duration::from_secs(60)), &["tag1"]).await.unwrap();
        cache.put_with_tags("key2", b"data2".to_vec(), Some(Duration::from_secs(60)), &["tag1"]).await.unwrap();
        cache.put_with_tags("key3", b"data3".to_vec(), Some(Duration::from_secs(60)), &["tag1"]).await.unwrap();
        
        // Manually remove one key from backend only (not registry)
        cache.backend.forget("key2").await.unwrap();
        
        // Now forget by tag - should only report keys that were actually in backend
        let removed = cache.forget_by_tag("tag1").await.unwrap();
        
        // Should report key1 and key3 as removed (key2 was already gone)
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&"key1".to_string()));
        assert!(removed.contains(&"key3".to_string()));
        assert!(!removed.contains(&"key2".to_string()));
    }
}