//! Cache invalidation strategies
//! 
//! This module provides various strategies for cache invalidation including
//! pattern-based, tag-based, time-based and event-driven invalidation.

use crate::{CacheBackend, CacheError, CacheResult, CacheKey, CacheTag};
use async_trait::async_trait;
use futures::future::join_all;
use regex::Regex;
use wildmatch::WildMatch;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tracing;

/// Pattern matching strategy for cache key invalidation
#[derive(Debug, Clone)]
pub enum PatternStrategy {
    /// Simple wildcard matching (*, ?)
    Wildcard(String),
    /// Full regex pattern matching
    Regex(String),
    /// Prefix matching
    Prefix(String),
    /// Suffix matching
    Suffix(String),
    /// Contains matching
    Contains(String),
}

impl PatternStrategy {
    /// Check if a key matches this pattern
    pub fn matches(&self, key: &str) -> CacheResult<bool> {
        match self {
            PatternStrategy::Wildcard(pattern) => {
                let matcher = WildMatch::new(pattern);
                Ok(matcher.matches(key))
            }
            PatternStrategy::Regex(pattern) => {
                let regex = Regex::new(pattern)
                    .map_err(|e| CacheError::Configuration(format!("Invalid regex: {}", e)))?;
                Ok(regex.is_match(key))
            }
            PatternStrategy::Prefix(prefix) => {
                Ok(key.starts_with(prefix))
            }
            PatternStrategy::Suffix(suffix) => {
                Ok(key.ends_with(suffix))
            }
            PatternStrategy::Contains(substring) => {
                Ok(key.contains(substring))
            }
        }
    }
}


/// Time-based invalidation policies
#[derive(Debug, Clone)]
pub enum TimeBasedPolicy {
    /// Absolute expiration time
    ExpiresAt(SystemTime),
    /// Time-to-live duration
    Ttl(Duration),
    /// Scheduled expiration (cron-like, simplified)
    Scheduled {
        /// Interval between expirations
        interval: Duration,
        /// Last execution time
        last_run: SystemTime,
    },
}

impl TimeBasedPolicy {
    /// Check if this policy indicates expiration
    pub fn should_expire(&self, created_at: SystemTime) -> bool {
        let now = SystemTime::now();
        
        match self {
            TimeBasedPolicy::ExpiresAt(expires_at) => now >= *expires_at,
            TimeBasedPolicy::Ttl(ttl) => {
                if let Ok(elapsed) = now.duration_since(created_at) {
                    elapsed >= *ttl
                } else {
                    false
                }
            }
            TimeBasedPolicy::Scheduled { interval, last_run } => {
                if let Ok(elapsed) = now.duration_since(*last_run) {
                    elapsed >= *interval
                } else {
                    false
                }
            }
        }
    }
}

/// Cache entry with invalidation metadata
#[derive(Debug, Clone)]
pub struct InvalidatableEntry {
    /// The cache key
    pub key: CacheKey,
    /// Associated tags
    pub tags: HashSet<CacheTag>,
    /// When this entry was created
    pub created_at: SystemTime,
    /// Time-based invalidation policies
    pub time_policies: Vec<TimeBasedPolicy>,
    /// Dependencies on other cache keys
    pub dependencies: HashSet<CacheKey>,
}

impl InvalidatableEntry {
    pub fn new(key: CacheKey) -> Self {
        Self {
            key,
            tags: HashSet::new(),
            created_at: SystemTime::now(),
            time_policies: Vec::new(),
            dependencies: HashSet::new(),
        }
    }
    
    /// Check if this entry should be invalidated based on time policies
    pub fn should_expire(&self) -> bool {
        self.time_policies.iter().any(|policy| policy.should_expire(self.created_at))
    }
    
    /// Add a tag to this entry
    pub fn add_tag(&mut self, tag: CacheTag) {
        self.tags.insert(tag);
    }
    
    /// Add multiple tags to this entry
    pub fn add_tags(&mut self, tags: impl IntoIterator<Item = CacheTag>) {
        self.tags.extend(tags);
    }
    
    /// Add a time-based policy
    pub fn add_time_policy(&mut self, policy: TimeBasedPolicy) {
        self.time_policies.push(policy);
    }
    
    /// Add a dependency on another key
    pub fn add_dependency(&mut self, key: CacheKey) {
        self.dependencies.insert(key);
    }
}

/// Advanced invalidation manager
#[async_trait]
pub trait InvalidationManager: Send + Sync {
    /// Invalidate cache entries matching a pattern
    async fn invalidate_pattern(&self, pattern: PatternStrategy) -> CacheResult<Vec<CacheKey>>;
    
    /// Invalidate cache entries by tags
    async fn invalidate_tags(&self, tags: &[&str]) -> CacheResult<Vec<CacheKey>>;
    
    /// Invalidate expired entries based on time policies
    async fn invalidate_expired(&self) -> CacheResult<Vec<CacheKey>>;
    
    /// Invalidate entries dependent on given keys
    async fn invalidate_dependencies(&self, keys: &[&str]) -> CacheResult<Vec<CacheKey>>;
    
    /// Get all keys tracked by this manager
    async fn get_tracked_keys(&self) -> CacheResult<Vec<CacheKey>>;
    
    /// Register a cache entry for invalidation tracking
    async fn register_entry(&self, entry: InvalidatableEntry) -> CacheResult<()>;
    
    /// Unregister a cache entry
    async fn unregister_entry(&self, key: &str) -> CacheResult<()>;
}

/// In-memory invalidation manager implementation
pub struct MemoryInvalidationManager<B: CacheBackend> {
    backend: Arc<B>,
    entries: Arc<RwLock<HashMap<CacheKey, InvalidatableEntry>>>,
}

impl<B: CacheBackend> MemoryInvalidationManager<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get keys matching a pattern
    async fn find_matching_keys(&self, pattern: &PatternStrategy) -> CacheResult<Vec<CacheKey>> {
        let entries = self.entries.read().await;
        let mut matching_keys = Vec::new();
        
        for key in entries.keys() {
            if pattern.matches(key)? {
                matching_keys.push(key.clone());
            }
        }
        
        Ok(matching_keys)
    }
    
    /// Get keys associated with any of the given tags
    async fn find_tagged_keys(&self, tags: &[&str]) -> CacheResult<Vec<CacheKey>> {
        let entries = self.entries.read().await;
        let mut matching_keys = Vec::new();
        let tag_set: HashSet<String> = tags.iter().map(|s| s.to_string()).collect();
        
        for (key, entry) in entries.iter() {
            if entry.tags.iter().any(|tag| tag_set.contains(tag)) {
                matching_keys.push(key.clone());
            }
        }
        
        Ok(matching_keys)
    }
    
    /// Get expired keys based on time policies
    async fn find_expired_keys(&self) -> CacheResult<Vec<CacheKey>> {
        let entries = self.entries.read().await;
        let mut expired_keys = Vec::new();
        
        for (key, entry) in entries.iter() {
            if entry.should_expire() {
                expired_keys.push(key.clone());
            }
        }
        
        Ok(expired_keys)
    }
    
    /// Get keys dependent on the given keys
    async fn find_dependent_keys(&self, dependency_keys: &[&str]) -> CacheResult<Vec<CacheKey>> {
        let entries = self.entries.read().await;
        let mut dependent_keys = Vec::new();
        let dep_set: HashSet<String> = dependency_keys.iter().map(|s| s.to_string()).collect();
        
        for (key, entry) in entries.iter() {
            if entry.dependencies.iter().any(|dep| dep_set.contains(dep)) {
                dependent_keys.push(key.clone());
            }
        }
        
        Ok(dependent_keys)
    }
    
    /// Actually invalidate the given keys from cache and tracking
    /// Uses parallel execution for better performance with network backends like Redis
    async fn execute_invalidation(&self, keys: Vec<CacheKey>) -> CacheResult<Vec<CacheKey>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        // Parallelize backend forget operations for better performance
        let forget_futures = keys.iter().map(|k| self.backend.forget(k));
        let results = join_all(forget_futures).await;

        // Collect successfully invalidated keys and handle any errors gracefully
        let mut invalidated_keys = Vec::new();
        let mut failed_keys = Vec::new();
        
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(was_removed) => {
                    if was_removed {
                        invalidated_keys.push(keys[i].clone());
                    }
                }
                Err(_) => {
                    failed_keys.push(keys[i].clone());
                }
            }
        }

        // Always remove from tracking, even if backend removal failed
        // This prevents inconsistent state where tracking believes keys exist but backend doesn't
        {
            let mut entries = self.entries.write().await;
            for key in &keys {
                entries.remove(key);
            }
        }

        // If there were backend failures, log them but don't fail the entire operation
        if !failed_keys.is_empty() {
            tracing::warn!(
                "Failed to invalidate {} keys from backend: {:?}", 
                failed_keys.len(), 
                failed_keys
            );
        }

        Ok(invalidated_keys)
    }
}

#[async_trait]
impl<B: CacheBackend> InvalidationManager for MemoryInvalidationManager<B> {
    async fn invalidate_pattern(&self, pattern: PatternStrategy) -> CacheResult<Vec<CacheKey>> {
        let matching_keys = self.find_matching_keys(&pattern).await?;
        self.execute_invalidation(matching_keys).await
    }
    
    async fn invalidate_tags(&self, tags: &[&str]) -> CacheResult<Vec<CacheKey>> {
        let tagged_keys = self.find_tagged_keys(tags).await?;
        self.execute_invalidation(tagged_keys).await
    }
    
    async fn invalidate_expired(&self) -> CacheResult<Vec<CacheKey>> {
        let expired_keys = self.find_expired_keys().await?;
        self.execute_invalidation(expired_keys).await
    }
    
    async fn invalidate_dependencies(&self, keys: &[&str]) -> CacheResult<Vec<CacheKey>> {
        let dependent_keys = self.find_dependent_keys(keys).await?;
        self.execute_invalidation(dependent_keys).await
    }
    
    async fn get_tracked_keys(&self) -> CacheResult<Vec<CacheKey>> {
        let entries = self.entries.read().await;
        Ok(entries.keys().cloned().collect())
    }
    
    async fn register_entry(&self, entry: InvalidatableEntry) -> CacheResult<()> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.key.clone(), entry);
        Ok(())
    }
    
    async fn unregister_entry(&self, key: &str) -> CacheResult<()> {
        let mut entries = self.entries.write().await;
        entries.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::MemoryBackend;
    use crate::config::CacheConfig;
    use std::time::Duration;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_pattern_strategy_wildcard() {
        let pattern = PatternStrategy::Wildcard("user:*".to_string());
        assert!(pattern.matches("user:123").unwrap());
        assert!(pattern.matches("user:abc").unwrap());
        assert!(!pattern.matches("post:123").unwrap());
        
        let pattern = PatternStrategy::Wildcard("user:?".to_string());
        assert!(pattern.matches("user:1").unwrap());
        assert!(!pattern.matches("user:12").unwrap());
        assert!(!pattern.matches("user:").unwrap());
    }
    
    #[tokio::test]
    async fn test_pattern_strategy_regex() {
        let pattern = PatternStrategy::Regex(r"user:\d+".to_string());
        assert!(pattern.matches("user:123").unwrap());
        assert!(!pattern.matches("user:abc").unwrap());
        assert!(!pattern.matches("post:123").unwrap());
    }
    
    #[tokio::test]
    async fn test_pattern_strategy_prefix_suffix_contains() {
        let prefix = PatternStrategy::Prefix("user:".to_string());
        assert!(prefix.matches("user:123").unwrap());
        assert!(!prefix.matches("post:123").unwrap());
        
        let suffix = PatternStrategy::Suffix(":active".to_string());
        assert!(suffix.matches("user:active").unwrap());
        assert!(!suffix.matches("user:inactive").unwrap());
        
        let contains = PatternStrategy::Contains("temp".to_string());
        assert!(contains.matches("temp_cache").unwrap());
        assert!(contains.matches("cache_temp").unwrap());
        assert!(!contains.matches("permanent").unwrap());
    }
    
    #[tokio::test]
    async fn test_wildcard_match() {
        // Test using PatternStrategy::Wildcard which now uses wildmatch
        let wildcard_all = PatternStrategy::Wildcard("*".to_string());
        assert!(wildcard_all.matches("anything").unwrap());
        
        let user_wildcard = PatternStrategy::Wildcard("user:*".to_string());
        assert!(user_wildcard.matches("user:123").unwrap());
        assert!(user_wildcard.matches("user:").unwrap());
        assert!(!user_wildcard.matches("post:123").unwrap());
        
        let user_question = PatternStrategy::Wildcard("user:?".to_string());
        assert!(user_question.matches("user:1").unwrap());
        assert!(!user_question.matches("user:12").unwrap());
        assert!(!user_question.matches("user:").unwrap());
        
        let temp_wildcard = PatternStrategy::Wildcard("*temp*".to_string());
        assert!(temp_wildcard.matches("temporary").unwrap());
        assert!(temp_wildcard.matches("temp").unwrap());
        assert!(temp_wildcard.matches("something_temp_else").unwrap());
        assert!(!temp_wildcard.matches("permanent").unwrap());
    }
    
    #[tokio::test]
    async fn test_time_based_policy() {
        let now = SystemTime::now();
        let past = now - Duration::from_secs(60);
        let future = now + Duration::from_secs(60);
        
        let expires_at_future = TimeBasedPolicy::ExpiresAt(future);
        assert!(!expires_at_future.should_expire(past));
        
        let expires_at_past = TimeBasedPolicy::ExpiresAt(past);
        assert!(expires_at_past.should_expire(past));
        
        let ttl_expired = TimeBasedPolicy::Ttl(Duration::from_secs(30));
        assert!(ttl_expired.should_expire(past));
        
        let ttl_valid = TimeBasedPolicy::Ttl(Duration::from_secs(120));
        assert!(!ttl_valid.should_expire(past));
    }
    
    #[tokio::test]
    async fn test_invalidatable_entry() {
        let mut entry = InvalidatableEntry::new("test:key".to_string());
        
        entry.add_tag("users".to_string());
        entry.add_tags(vec!["active".to_string(), "premium".to_string()]);
        
        assert!(entry.tags.contains("users"));
        assert!(entry.tags.contains("active"));
        assert!(entry.tags.contains("premium"));
        assert_eq!(entry.tags.len(), 3);
        
        entry.add_time_policy(TimeBasedPolicy::Ttl(Duration::from_millis(1)));
        sleep(Duration::from_millis(2)).await;
        assert!(entry.should_expire());
        
        entry.add_dependency("parent:key".to_string());
        assert!(entry.dependencies.contains("parent:key"));
    }
    
    #[tokio::test]
    async fn test_memory_invalidation_manager() {
        let backend = Arc::new(MemoryBackend::new(CacheConfig::default()));
        let manager = MemoryInvalidationManager::new(backend.clone());
        
        // Put some test data in cache
        backend.put("user:1", b"data1".to_vec(), None).await.unwrap();
        backend.put("user:2", b"data2".to_vec(), None).await.unwrap();
        backend.put("post:1", b"data3".to_vec(), None).await.unwrap();
        
        // Register entries with manager
        let mut entry1 = InvalidatableEntry::new("user:1".to_string());
        entry1.add_tag("users".to_string());
        manager.register_entry(entry1).await.unwrap();
        
        let mut entry2 = InvalidatableEntry::new("user:2".to_string());
        entry2.add_tag("users".to_string());
        manager.register_entry(entry2).await.unwrap();
        
        let mut entry3 = InvalidatableEntry::new("post:1".to_string());
        entry3.add_tag("posts".to_string());
        manager.register_entry(entry3).await.unwrap();
        
        // Test pattern invalidation
        let invalidated = manager
            .invalidate_pattern(PatternStrategy::Wildcard("user:*".to_string()))
            .await
            .unwrap();
        assert_eq!(invalidated.len(), 2);
        assert!(invalidated.contains(&"user:1".to_string()));
        assert!(invalidated.contains(&"user:2".to_string()));
        
        // Verify cache entries are gone
        assert_eq!(backend.get("user:1").await.unwrap(), None);
        assert_eq!(backend.get("user:2").await.unwrap(), None);
        assert_eq!(backend.get("post:1").await.unwrap(), Some(b"data3".to_vec()));
        
        // Test tag invalidation
        let invalidated = manager.invalidate_tags(&["posts"]).await.unwrap();
        assert_eq!(invalidated.len(), 1);
        assert!(invalidated.contains(&"post:1".to_string()));
        
        // Verify all cache entries are gone
        assert_eq!(backend.get("post:1").await.unwrap(), None);
    }
    
    #[tokio::test]
    async fn test_dependency_invalidation() {
        let backend = Arc::new(MemoryBackend::new(CacheConfig::default()));
        let manager = MemoryInvalidationManager::new(backend.clone());
        
        // Put some test data in cache
        backend.put("parent", b"parent_data".to_vec(), None).await.unwrap();
        backend.put("child1", b"child1_data".to_vec(), None).await.unwrap();
        backend.put("child2", b"child2_data".to_vec(), None).await.unwrap();
        backend.put("independent", b"independent_data".to_vec(), None).await.unwrap();
        
        // Register entries with dependencies
        let parent_entry = InvalidatableEntry::new("parent".to_string());
        manager.register_entry(parent_entry).await.unwrap();
        
        let mut child1_entry = InvalidatableEntry::new("child1".to_string());
        child1_entry.add_dependency("parent".to_string());
        manager.register_entry(child1_entry).await.unwrap();
        
        let mut child2_entry = InvalidatableEntry::new("child2".to_string());
        child2_entry.add_dependency("parent".to_string());
        manager.register_entry(child2_entry).await.unwrap();
        
        let independent_entry = InvalidatableEntry::new("independent".to_string());
        manager.register_entry(independent_entry).await.unwrap();
        
        // Test dependency invalidation
        let invalidated = manager
            .invalidate_dependencies(&["parent"])
            .await
            .unwrap();
        
        assert_eq!(invalidated.len(), 2);
        assert!(invalidated.contains(&"child1".to_string()));
        assert!(invalidated.contains(&"child2".to_string()));
        
        // Verify dependent entries are gone but parent and independent remain
        assert_eq!(backend.get("child1").await.unwrap(), None);
        assert_eq!(backend.get("child2").await.unwrap(), None);
        assert_eq!(backend.get("parent").await.unwrap(), Some(b"parent_data".to_vec()));
        assert_eq!(backend.get("independent").await.unwrap(), Some(b"independent_data".to_vec()));
    }
    
    #[tokio::test]
    async fn test_time_based_invalidation() {
        let backend = Arc::new(MemoryBackend::new(CacheConfig::default()));
        let manager = MemoryInvalidationManager::new(backend.clone());
        
        // Put some test data in cache
        backend.put("temp1", b"temp1_data".to_vec(), None).await.unwrap();
        backend.put("temp2", b"temp2_data".to_vec(), None).await.unwrap();
        backend.put("permanent", b"permanent_data".to_vec(), None).await.unwrap();
        
        // Register entries with time policies
        let mut temp1_entry = InvalidatableEntry::new("temp1".to_string());
        temp1_entry.add_time_policy(TimeBasedPolicy::Ttl(Duration::from_millis(1)));
        manager.register_entry(temp1_entry).await.unwrap();
        
        let mut temp2_entry = InvalidatableEntry::new("temp2".to_string());
        temp2_entry.add_time_policy(TimeBasedPolicy::Ttl(Duration::from_millis(1)));
        manager.register_entry(temp2_entry).await.unwrap();
        
        let permanent_entry = InvalidatableEntry::new("permanent".to_string());
        manager.register_entry(permanent_entry).await.unwrap();
        
        // Wait for expiration
        sleep(Duration::from_millis(2)).await;
        
        // Test time-based invalidation
        let invalidated = manager.invalidate_expired().await.unwrap();
        assert_eq!(invalidated.len(), 2);
        assert!(invalidated.contains(&"temp1".to_string()));
        assert!(invalidated.contains(&"temp2".to_string()));
        
        // Verify expired entries are gone but permanent remains
        assert_eq!(backend.get("temp1").await.unwrap(), None);
        assert_eq!(backend.get("temp2").await.unwrap(), None);
        assert_eq!(backend.get("permanent").await.unwrap(), Some(b"permanent_data".to_vec()));
    }
    
    #[tokio::test]
    async fn test_parallel_invalidation_performance() {
        let backend = Arc::new(MemoryBackend::new(CacheConfig::default()));
        let manager = MemoryInvalidationManager::new(backend.clone());
        
        // Put a large number of entries
        for i in 0..100 {
            backend.put(&format!("batch_key_{}", i), b"data".to_vec(), None).await.unwrap();
            
            let mut entry = InvalidatableEntry::new(format!("batch_key_{}", i));
            entry.add_tag("batch".to_string());
            manager.register_entry(entry).await.unwrap();
        }
        
        // Test parallel invalidation with pattern
        let start_time = std::time::Instant::now();
        let invalidated = manager
            .invalidate_pattern(PatternStrategy::Wildcard("batch_key_*".to_string()))
            .await
            .unwrap();
        let duration = start_time.elapsed();
        
        // All keys should be invalidated
        assert_eq!(invalidated.len(), 100);
        
        // Parallel execution should be reasonably fast (this is a rough check)
        // In practice, with network backends like Redis, the performance improvement would be more significant
        assert!(duration.as_millis() < 1000, "Parallel invalidation took too long: {:?}", duration);
        
        // Verify all keys are gone
        for i in 0..100 {
            assert_eq!(backend.get(&format!("batch_key_{}", i)).await.unwrap(), None);
        }
    }
}