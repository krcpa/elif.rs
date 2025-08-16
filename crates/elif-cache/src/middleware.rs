//! HTTP response caching utilities
//!
//! This module provides HTTP response caching utilities that can be used
//! with the elif-http framework. Applications can use these helpers to 
//! implement response caching in their handlers.

use crate::{Cache, CacheBackend};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use blake3::Hasher;

#[cfg(feature = "http-cache")]
use elif_http::{
    request::ElifRequest,
    response::{ElifResponse, ElifStatusCode},
};

/// HTTP cache configuration
#[derive(Debug, Clone)]
pub struct HttpCacheConfig {
    /// Default cache duration
    pub default_ttl: Duration,
    
    /// Cache key generation strategy
    pub key_strategy: CacheKeyStrategy,
    
    /// Headers to include in cache key
    pub vary_by_headers: Vec<String>,
    
    /// Query parameters to include in cache key
    pub vary_by_query: Vec<String>,
    
    /// Skip caching for these paths (patterns supported)
    pub skip_paths: Vec<String>,
    
    /// Skip caching for these methods
    pub skip_methods: Vec<String>,
    
    /// Maximum response size to cache (in bytes)
    pub max_response_size: usize,
    
    /// Cache only successful responses (2xx status codes)
    pub only_success: bool,
    
    /// Include user-specific data in cache key (requires auth)
    pub per_user: bool,
    
    /// Custom cache key prefix
    pub key_prefix: String,
}

impl Default for HttpCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300), // 5 minutes
            key_strategy: CacheKeyStrategy::UrlBased,
            vary_by_headers: vec!["accept".to_string(), "accept-encoding".to_string()],
            vary_by_query: vec![],
            skip_paths: vec!["/admin".to_string(), "/api/auth".to_string()],
            skip_methods: vec!["POST".to_string(), "PUT".to_string(), "DELETE".to_string(), "PATCH".to_string()],
            max_response_size: 1024 * 1024, // 1MB
            only_success: true,
            per_user: false,
            key_prefix: "http_cache:".to_string(),
        }
    }
}

/// Cache key generation strategies
#[derive(Debug, Clone)]
pub enum CacheKeyStrategy {
    /// Use URL + method as base key
    UrlBased,
    
    /// Use custom key generation function
    Custom(String),
}

/// Cached response metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedResponse {
    /// HTTP status code
    pub status: u16,
    
    /// Response headers
    pub headers: HashMap<String, String>,
    
    /// Response body
    pub body: Vec<u8>,
    
    /// When this response was cached
    pub cached_at: u64,
    
    /// Cache TTL in seconds
    pub ttl: u64,
    
    /// ETag for cache validation
    pub etag: Option<String>,
    
    /// Last-Modified timestamp
    pub last_modified: Option<String>,
}

impl CachedResponse {
    /// Create a new cached response
    pub fn new(status: u16, body: &[u8]) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            status,
            headers: HashMap::new(),
            body: body.to_vec(),
            cached_at: now,
            ttl: 300, // Default 5 minutes
            etag: None,
            last_modified: None,
        }
    }
    
    /// Create cached response with headers
    pub fn with_headers(status: u16, headers: HashMap<String, String>, body: &[u8]) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            status,
            headers: headers.clone(),
            body: body.to_vec(),
            cached_at: now,
            ttl: 300, // Default 5 minutes
            etag: headers.get("etag").cloned(),
            last_modified: headers.get("last-modified").cloned(),
        }
    }
    
    /// Set TTL for the cached response
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl.as_secs();
        self
    }
    
    /// Check if the cached response is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now < self.cached_at + self.ttl
    }
    
    /// Get the response body
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    
    /// Get the response status
    pub fn status(&self) -> u16 {
        self.status
    }
    
    /// Get the response headers
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

/// HTTP response caching helper
/// 
/// This helper provides utilities for caching HTTP responses in elif-http applications.
/// Applications should integrate this into their handlers rather than using it as middleware.
pub struct HttpCacheHelper<B: CacheBackend> {
    cache: Cache<B>,
    config: HttpCacheConfig,
}

impl<B: CacheBackend> HttpCacheHelper<B> {
    /// Create new HTTP cache helper
    pub fn new(cache: Cache<B>, config: HttpCacheConfig) -> Self {
        Self { cache, config }
    }
    
    /// Create with default configuration
    pub fn with_defaults(cache: Cache<B>) -> Self {
        Self::new(cache, HttpCacheConfig::default())
    }
    
    /// Generate cache key for request
    /// 
    /// This method takes request information and generates a unique cache key.
    /// Applications can call this directly or use the higher-level caching methods.
    pub fn generate_cache_key_for(&self, method: &str, uri: &str, headers: &HashMap<String, String>) -> String {
        let mut hasher = Hasher::new();
        
        // Add method and URL
        hasher.update(method.as_bytes());
        hasher.update(uri.as_bytes());
        
        // Add headers if configured
        for header_name in &self.config.vary_by_headers {
            if let Some(header_value) = headers.get(header_name) {
                hasher.update(header_name.as_bytes());
                hasher.update(header_value.as_bytes());
            }
        }
        
        // Add user ID if per-user caching is enabled
        if self.config.per_user {
            if let Some(user_id) = headers.get("x-user-id") {
                hasher.update(b"user:");
                hasher.update(user_id.as_bytes());
            }
        }
        
        let hash = hasher.finalize();
        format!("{}{}", self.config.key_prefix, hex::encode(hash.as_bytes()))
    }
    
    /// Check if request should be cached based on method and path
    pub fn should_cache(&self, method: &str, path: &str, headers: &HashMap<String, String>) -> bool {
        // Check method
        if self.config.skip_methods.contains(&method.to_uppercase()) {
            return false;
        }
        
        // Check paths (simple pattern matching)
        for skip_path in &self.config.skip_paths {
            if path.starts_with(skip_path) {
                return false;
            }
        }
        
        // Check for cache control headers
        if let Some(cache_control) = headers.get("cache-control") {
            if cache_control.contains("no-cache") || cache_control.contains("no-store") {
                return false;
            }
        }
        
        true
    }
    
    /// Check if response should be cached based on status and headers
    pub fn should_cache_response(&self, status: u16, response_headers: &HashMap<String, String>) -> bool {
        // Check status code
        if self.config.only_success && !(200..300).contains(&status) {
            return false;
        }
        
        // Check cache control headers
        if let Some(cache_control) = response_headers.get("cache-control") {
            if cache_control.contains("no-cache") 
                || cache_control.contains("no-store") 
                || cache_control.contains("private") {
                return false;
            }
        }
        
        true
    }
    
    /// Get or set cached response using the remember pattern
    /// 
    /// This is the main method applications should use for HTTP response caching.
    pub async fn remember_response<F, Fut>(
        &self,
        cache_key: &str,
        ttl: Duration,
        compute: F,
    ) -> Result<CachedResponse, crate::CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = CachedResponse>,
    {
        self.cache.remember(cache_key, ttl, compute).await
    }
    
    /// Extract TTL from response headers or use default
    pub fn extract_ttl_from_headers(&self, response_headers: &HashMap<String, String>) -> Duration {
        // Try to extract from Cache-Control max-age
        if let Some(cache_control) = response_headers.get("cache-control") {
            for directive in cache_control.split(',') {
                let directive = directive.trim();
                if directive.starts_with("max-age=") {
                    if let Ok(seconds) = directive[8..].parse::<u64>() {
                        return Duration::from_secs(seconds);
                    }
                }
            }
        }
        
        self.config.default_ttl
    }
}

/// Example usage for HTTP handlers
/// 
/// ```rust,ignore
/// use elif_cache::{Cache, MemoryBackend, HttpCacheHelper, HttpCacheConfig};
/// use std::collections::HashMap;
/// use std::time::Duration;
/// 
/// async fn my_handler(cache_helper: &HttpCacheHelper<impl CacheBackend>) -> Result<String, Box<dyn std::error::Error>> {
///     let method = "GET";
///     let uri = "/api/users";
///     let headers = HashMap::new();
///     
///     if !cache_helper.should_cache(method, uri, &headers) {
///         return Ok("Not cached".to_string());
///     }
///     
///     let cache_key = cache_helper.generate_cache_key_for(method, uri, &headers);
///     
///     let response = cache_helper.remember_response(
///         &cache_key,
///         Duration::from_secs(300),
///         || async {
///             // Generate the response
///             CachedResponse::new(200, "Hello, World!".as_bytes())
///         }
///     ).await?;
///     
///     Ok(String::from_utf8_lossy(&response.body).to_string())
/// }
/// ```

/// Builder for HTTP cache helper configuration
pub struct HttpCacheBuilder {
    config: HttpCacheConfig,
}

impl HttpCacheBuilder {
    pub fn new() -> Self {
        Self {
            config: HttpCacheConfig::default(),
        }
    }
    
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.config.default_ttl = ttl;
        self
    }
    
    pub fn vary_by_headers(mut self, headers: Vec<String>) -> Self {
        self.config.vary_by_headers = headers;
        self
    }
    
    pub fn vary_by_query(mut self, params: Vec<String>) -> Self {
        self.config.vary_by_query = params;
        self
    }
    
    pub fn skip_paths(mut self, paths: Vec<String>) -> Self {
        self.config.skip_paths = paths;
        self
    }
    
    pub fn skip_methods(mut self, methods: Vec<String>) -> Self {
        self.config.skip_methods = methods;
        self
    }
    
    pub fn max_response_size(mut self, size: usize) -> Self {
        self.config.max_response_size = size;
        self
    }
    
    pub fn cache_all_responses(mut self) -> Self {
        self.config.only_success = false;
        self
    }
    
    pub fn per_user_cache(mut self) -> Self {
        self.config.per_user = true;
        self
    }
    
    pub fn key_prefix(mut self, prefix: String) -> Self {
        self.config.key_prefix = prefix;
        self
    }
    
    pub fn build<B: CacheBackend>(self, cache: Cache<B>) -> HttpCacheHelper<B> {
        HttpCacheHelper::new(cache, self.config)
    }
}

impl Default for HttpCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::MemoryBackend;
    use crate::config::CacheConfig;
    
    #[tokio::test]
    async fn test_cache_key_generation() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let helper = HttpCacheHelper::with_defaults(cache);
        
        let headers = HashMap::new();
        
        let key1 = helper.generate_cache_key_for("GET", "https://example.com/api/users", &headers);
        let key2 = helper.generate_cache_key_for("GET", "https://example.com/api/users", &headers);
        
        // Same request should generate same key
        assert_eq!(key1, key2);
        
        // Different request should generate different key
        let key3 = helper.generate_cache_key_for("GET", "https://example.com/api/posts", &headers);
        assert_ne!(key1, key3);
    }
    
    #[tokio::test]
    async fn test_should_cache() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let helper = HttpCacheHelper::with_defaults(cache);
        
        let headers = HashMap::new();
        
        // GET request should be cached
        assert!(helper.should_cache("GET", "/api/users", &headers));
        
        // POST request should not be cached
        assert!(!helper.should_cache("POST", "/api/users", &headers));
        
        // Admin paths should not be cached
        assert!(!helper.should_cache("GET", "/admin/users", &headers));
        
        // Requests with no-cache header should not be cached
        let mut no_cache_headers = HashMap::new();
        no_cache_headers.insert("cache-control".to_string(), "no-cache".to_string());
        assert!(!helper.should_cache("GET", "/api/users", &no_cache_headers));
    }
    
    #[tokio::test]
    async fn test_should_cache_response() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let helper = HttpCacheHelper::with_defaults(cache);
        
        let headers = HashMap::new();
        
        // 200 response should be cached
        assert!(helper.should_cache_response(200, &headers));
        
        // 404 response should not be cached (with only_success=true)
        assert!(!helper.should_cache_response(404, &headers));
        
        // Response with no-cache header should not be cached
        let mut no_cache_headers = HashMap::new();
        no_cache_headers.insert("cache-control".to_string(), "no-cache".to_string());
        assert!(!helper.should_cache_response(200, &no_cache_headers));
    }
    
    #[tokio::test]
    async fn test_cached_response_validity() {
        let cached_response = CachedResponse::new(200, b"test")
            .with_ttl(Duration::from_secs(300));
        
        assert!(cached_response.is_valid());
        
        // Create an expired response by manually setting old timestamp
        let mut expired_response = CachedResponse::new(200, b"test")
            .with_ttl(Duration::from_secs(300));
        expired_response.cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 400; // Cached 400 seconds ago
        
        assert!(!expired_response.is_valid());
    }
    
    #[tokio::test]
    async fn test_cached_response_constructor() {
        let response = CachedResponse::new(200, b"Hello, World!");
        
        assert_eq!(response.status(), 200);
        assert_eq!(response.body(), b"Hello, World!");
        assert!(response.headers().is_empty());
        
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain".to_string());
        
        let response_with_headers = CachedResponse::with_headers(201, headers.clone(), b"Created");
        assert_eq!(response_with_headers.status(), 201);
        assert_eq!(response_with_headers.body(), b"Created");
        assert_eq!(response_with_headers.headers(), &headers);
    }
    
    #[test]
    fn test_http_cache_builder() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        let helper = HttpCacheBuilder::new()
            .default_ttl(Duration::from_secs(600))
            .vary_by_headers(vec!["Authorization".to_string()])
            .skip_paths(vec!["/api/private".to_string()])
            .per_user_cache()
            .build(cache);
        
        assert_eq!(helper.config.default_ttl, Duration::from_secs(600));
        assert!(helper.config.vary_by_headers.contains(&"Authorization".to_string()));
        assert!(helper.config.skip_paths.contains(&"/api/private".to_string()));
        assert!(helper.config.per_user);
    }
}