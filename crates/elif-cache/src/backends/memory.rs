//! In-memory cache backend with LRU eviction

use crate::{CacheBackend, CacheConfig, CacheError, CacheResult, CacheStats};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::{RwLock, Mutex};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

/// Entry in the memory cache
#[derive(Debug)]
struct CacheEntry {
    data: Vec<u8>,
    created_at: Instant,
    expires_at: Option<Instant>,
    access_count: AtomicU64,
    last_accessed: RwLock<Instant>,
}

impl Clone for CacheEntry {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
            access_count: AtomicU64::new(self.access_count.load(Ordering::Relaxed)),
            last_accessed: RwLock::new(*self.last_accessed.read()),
        }
    }
}

impl CacheEntry {
    fn new(data: Vec<u8>, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            data,
            created_at: now,
            expires_at: ttl.map(|ttl| now + ttl),
            access_count: AtomicU64::new(1),
            last_accessed: RwLock::new(now),
        }
    }
    
    fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| Instant::now() > exp)
    }
    
    fn access(&self) -> Vec<u8> {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        *self.last_accessed.write() = Instant::now();
        self.data.clone()
    }
    
    fn size(&self) -> usize {
        self.data.len() + std::mem::size_of::<Self>()
    }
}

/// LRU eviction policy implementation
#[derive(Debug)]
struct LruTracker {
    access_order: RwLock<VecDeque<String>>,
    key_positions: DashMap<String, usize>,
}

impl LruTracker {
    fn new() -> Self {
        Self {
            access_order: RwLock::new(VecDeque::new()),
            key_positions: DashMap::new(),
        }
    }
    
    fn access(&self, key: &str) {
        let mut access_order = self.access_order.write();
        
        // Remove from current position if exists
        if let Some(pos) = self.key_positions.get(key).map(|p| *p) {
            access_order.remove(pos);
            // Update positions of keys after the removed one
            for (i, k) in access_order.iter().enumerate().skip(pos) {
                self.key_positions.insert(k.clone(), i);
            }
        }
        
        // Add to front (most recently used)
        access_order.push_front(key.to_string());
        self.key_positions.insert(key.to_string(), 0);
        
        // Update positions of other keys
        for (i, k) in access_order.iter().enumerate().skip(1) {
            self.key_positions.insert(k.clone(), i);
        }
    }
    
    fn remove(&self, key: &str) {
        let mut access_order = self.access_order.write();
        
        if let Some(pos) = self.key_positions.remove(key).map(|(_, p)| p) {
            access_order.remove(pos);
            // Update positions of keys after the removed one
            for (i, k) in access_order.iter().enumerate().skip(pos) {
                self.key_positions.insert(k.clone(), i);
            }
        }
    }
    
    fn least_recently_used(&self) -> Option<String> {
        self.access_order.read().back().cloned()
    }
}

/// In-memory cache backend with LRU eviction
pub struct MemoryBackend {
    entries: DashMap<String, CacheEntry>,
    lru: LruTracker,
    config: CacheConfig,
    stats: Arc<Mutex<CacheStats>>,
}

impl MemoryBackend {
    /// Create a new memory backend with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: DashMap::new(),
            lru: LruTracker::new(),
            config,
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }
    
    /// Get current memory usage in bytes
    fn memory_usage(&self) -> usize {
        self.entries.iter().map(|entry| entry.value().size()).sum()
    }
    
    /// Check if we need to evict entries
    fn should_evict(&self) -> bool {
        if let Some(max_entries) = self.config.max_entries {
            if self.entries.len() >= max_entries {
                return true;
            }
        }
        
        if let Some(max_memory) = self.config.max_memory {
            if self.memory_usage() >= max_memory {
                return true;
            }
        }
        
        false
    }
    
    /// Evict expired and least recently used entries
    async fn evict(&self) -> CacheResult<()> {
        // First, remove expired entries
        let expired_keys: Vec<String> = self.entries
            .iter()
            .filter_map(|entry| {
                if entry.value().is_expired() {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();
        
        for key in expired_keys {
            self.entries.remove(&key);
            self.lru.remove(&key);
        }
        
        // Then, evict LRU entries if still over limits
        while self.should_evict() {
            if let Some(lru_key) = self.lru.least_recently_used() {
                self.entries.remove(&lru_key);
                self.lru.remove(&lru_key);
            } else {
                break;
            }
        }
        
        Ok(())
    }
    
    /// Clean up expired entries (background task)
    async fn cleanup_expired(&self) {
        let expired_keys: Vec<String> = self.entries
            .iter()
            .filter_map(|entry| {
                if entry.value().is_expired() {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();
        
        for key in expired_keys {
            self.entries.remove(&key);
            self.lru.remove(&key);
        }
    }
}

#[async_trait]
impl CacheBackend for MemoryBackend {
    async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
        // Clean up expired entries occasionally
        if rand::random::<f64>() < 0.01 { // 1% chance
            self.cleanup_expired().await;
        }
        
        if let Some(entry) = self.entries.get(key) {
            if entry.is_expired() {
                // Remove expired entry
                drop(entry);
                self.entries.remove(key);
                self.lru.remove(key);
                
                // Update stats
                self.stats.lock().misses += 1;
                
                return Ok(None);
            }
            
            // Access the entry (updates LRU and access count)
            let data = entry.access();
            self.lru.access(key);
            
            // Update stats
            self.stats.lock().hits += 1;
            
            Ok(Some(data))
        } else {
            // Update stats
            self.stats.lock().misses += 1;
            
            Ok(None)
        }
    }
    
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> CacheResult<()> {
        // Evict if necessary before adding
        if self.should_evict() {
            self.evict().await?;
        }
        
        let entry = CacheEntry::new(value, ttl);
        
        // Insert or update entry
        if self.entries.insert(key.to_string(), entry).is_none() {
            // New entry, update total count
            let mut stats = self.stats.lock();
            stats.total_keys += 1;
            stats.memory_usage = self.memory_usage() as u64;
        } else {
            // Update memory usage for existing entry
            self.stats.lock().memory_usage = self.memory_usage() as u64;
        }
        
        // Update LRU
        self.lru.access(key);
        
        Ok(())
    }
    
    async fn forget(&self, key: &str) -> CacheResult<bool> {
        let removed = self.entries.remove(key).is_some();
        
        if removed {
            self.lru.remove(key);
            
            // Update stats
            let mut stats = self.stats.lock();
            stats.total_keys = stats.total_keys.saturating_sub(1);
            stats.memory_usage = self.memory_usage() as u64;
        }
        
        Ok(removed)
    }
    
    async fn exists(&self, key: &str) -> CacheResult<bool> {
        if let Some(entry) = self.entries.get(key) {
            if entry.is_expired() {
                // Clean up expired entry
                drop(entry);
                self.entries.remove(key);
                self.lru.remove(key);
                
                let mut stats = self.stats.lock();
                stats.total_keys = stats.total_keys.saturating_sub(1);
                stats.memory_usage = self.memory_usage() as u64;
                
                return Ok(false);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    async fn flush(&self) -> CacheResult<()> {
        self.entries.clear();
        
        // Reset LRU tracker
        self.lru.access_order.write().clear();
        self.lru.key_positions.clear();
        
        // Reset stats
        let mut stats = self.stats.lock();
        stats.total_keys = 0;
        stats.memory_usage = 0;
        
        Ok(())
    }
    
    async fn get_many(&self, keys: &[&str]) -> CacheResult<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::with_capacity(keys.len());
        
        for key in keys {
            results.push(self.get(key).await?);
        }
        
        Ok(results)
    }
    
    async fn put_many(&self, entries: &[(&str, Vec<u8>, Option<Duration>)]) -> CacheResult<()> {
        for (key, value, ttl) in entries {
            self.put(key, value.clone(), *ttl).await?;
        }
        
        Ok(())
    }
    
    async fn stats(&self) -> CacheResult<CacheStats> {
        let stats = self.stats.lock();
        Ok(CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            total_keys: self.entries.len() as u64,
            memory_usage: self.memory_usage() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_memory_backend_basic_operations() {
        let backend = MemoryBackend::new(CacheConfig::default());
        
        // Test put and get
        backend.put("test", b"value".to_vec(), Some(Duration::from_secs(60))).await.unwrap();
        let result = backend.get("test").await.unwrap();
        assert_eq!(result, Some(b"value".to_vec()));
        
        // Test exists
        assert!(backend.exists("test").await.unwrap());
        assert!(!backend.exists("nonexistent").await.unwrap());
        
        // Test forget
        assert!(backend.forget("test").await.unwrap());
        assert!(!backend.exists("test").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_memory_backend_ttl() {
        let backend = MemoryBackend::new(CacheConfig::default());
        
        // Put with very short TTL
        backend.put("ttl_test", b"value".to_vec(), Some(Duration::from_millis(50))).await.unwrap();
        
        // Should exist initially
        assert!(backend.exists("ttl_test").await.unwrap());
        
        // Wait for expiration
        sleep(Duration::from_millis(100)).await;
        
        // Should be expired
        assert!(!backend.exists("ttl_test").await.unwrap());
        let result = backend.get("ttl_test").await.unwrap();
        assert_eq!(result, None);
    }
    
    #[tokio::test]
    async fn test_memory_backend_lru_eviction() {
        let config = CacheConfig::builder()
            .max_entries(2)
            .build();
        let backend = MemoryBackend::new(config);
        
        // Fill cache to capacity
        backend.put("key1", b"value1".to_vec(), None).await.unwrap();
        backend.put("key2", b"value2".to_vec(), None).await.unwrap();
        
        // Access key1 to make it more recently used
        backend.get("key1").await.unwrap();
        
        // Add third key, should evict key2 (least recently used)
        backend.put("key3", b"value3".to_vec(), None).await.unwrap();
        
        // key1 and key3 should exist, key2 should be evicted
        assert!(backend.exists("key1").await.unwrap());
        assert!(!backend.exists("key2").await.unwrap());
        assert!(backend.exists("key3").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_memory_backend_stats() {
        let backend = MemoryBackend::new(CacheConfig::default());
        
        // Initial stats
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_keys, 0);
        
        // Add some data
        backend.put("test1", b"value1".to_vec(), None).await.unwrap();
        backend.put("test2", b"value2".to_vec(), None).await.unwrap();
        
        // Check stats after puts
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.total_keys, 2);
        assert!(stats.memory_usage > 0);
        
        // Test cache hit
        backend.get("test1").await.unwrap();
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.hits, 1);
        
        // Test cache miss
        backend.get("nonexistent").await.unwrap();
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.misses, 1);
        
        // Check hit ratio
        assert_eq!(stats.hit_ratio(), 0.5);
    }
    
    #[tokio::test]
    async fn test_memory_backend_flush() {
        let backend = MemoryBackend::new(CacheConfig::default());
        
        // Add some data
        backend.put("test1", b"value1".to_vec(), None).await.unwrap();
        backend.put("test2", b"value2".to_vec(), None).await.unwrap();
        
        // Verify data exists
        assert!(backend.exists("test1").await.unwrap());
        assert!(backend.exists("test2").await.unwrap());
        
        // Flush cache
        backend.flush().await.unwrap();
        
        // Verify cache is empty
        assert!(!backend.exists("test1").await.unwrap());
        assert!(!backend.exists("test2").await.unwrap());
        
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.total_keys, 0);
        assert_eq!(stats.memory_usage, 0);
    }
}