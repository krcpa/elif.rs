//! HTTP response caching middleware

use crate::{Cache, CacheBackend, CacheError, CacheResult};
use elif_http::{
    middleware::{Middleware, BoxFuture},
};
use axum::{extract::Request, response::Response, http::{HeaderMap, StatusCode}};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use blake3::Hasher;

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
    /// Check if the cached response is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now < self.cached_at + self.ttl
    }
    
    /// Convert to HTTP response
    pub fn to_response(self) -> ElifResponse {
        let mut response = ResponseBuilder::new()
            .status(self.status)
            .body(self.body);
            
        // Add cached headers
        for (name, value) in self.headers {
            response = response.header(&name, &value);
        }
        
        // Add cache headers
        response = response.header("X-Cache", "HIT");
        
        response.build()
    }
}

/// HTTP response caching middleware
pub struct ResponseCacheMiddleware<B: CacheBackend> {
    cache: Cache<B>,
    config: HttpCacheConfig,
}

impl<B: CacheBackend> ResponseCacheMiddleware<B> {
    /// Create new HTTP cache middleware
    pub fn new(cache: Cache<B>, config: HttpCacheConfig) -> Self {
        Self { cache, config }
    }
    
    /// Create with default configuration
    pub fn with_defaults(cache: Cache<B>) -> Self {
        Self::new(cache, HttpCacheConfig::default())
    }
    
    /// Generate cache key for request
    fn generate_cache_key(&self, request: &ElifRequest) -> String {
        let mut hasher = Hasher::new();
        
        // Add method and URL
        hasher.update(request.method().as_bytes());
        hasher.update(request.uri().as_bytes());
        
        // Add headers if configured
        for header_name in &self.config.vary_by_headers {
            if let Some(header_value) = request.header(header_name) {
                hasher.update(header_name.as_bytes());
                hasher.update(header_value.as_bytes());
            }
        }
        
        // Add query parameters if configured
        for param_name in &self.config.vary_by_query {
            if let Some(param_value) = request.query(param_name) {
                hasher.update(param_name.as_bytes());
                hasher.update(param_value.as_bytes());
            }
        }
        
        // Add user ID if per-user caching is enabled
        if self.config.per_user {
            if let Some(user_id) = self.extract_user_id(request) {
                hasher.update(b"user:");
                hasher.update(user_id.as_bytes());
            }
        }
        
        let hash = hasher.finalize();
        format!("{}{}", self.config.key_prefix, hex::encode(hash.as_bytes()))
    }
    
    /// Extract user ID from request (placeholder implementation)
    fn extract_user_id(&self, request: &ElifRequest) -> Option<String> {
        // In a real implementation, this would extract user ID from JWT token,
        // session, or other authentication mechanism
        request.header("x-user-id").map(|s| s.to_string())
    }
    
    /// Check if request should be cached
    fn should_cache_request(&self, request: &ElifRequest) -> bool {
        // Check method
        if self.config.skip_methods.contains(&request.method().to_uppercase()) {
            return false;
        }
        
        // Check paths (simple pattern matching)
        let path = request.uri_path();
        for skip_path in &self.config.skip_paths {
            if path.starts_with(skip_path) {
                return false;
            }
        }
        
        // Check for cache control headers
        if let Some(cache_control) = request.header("cache-control") {
            if cache_control.contains("no-cache") || cache_control.contains("no-store") {
                return false;
            }
        }
        
        true
    }
    
    /// Check if response should be cached
    fn should_cache_response(&self, response: &ElifResponse) -> bool {
        // Check status code
        if self.config.only_success && !response.status().is_success() {
            return false;
        }
        
        // Check response size
        if let Some(body_size) = response.body().map(|b| b.len()) {
            if body_size > self.config.max_response_size {
                return false;
            }
        }
        
        // Check cache control headers
        if let Some(cache_control) = response.header("cache-control") {
            if cache_control.contains("no-cache") 
                || cache_control.contains("no-store") 
                || cache_control.contains("private") {
                return false;
            }
        }
        
        true
    }
    
    /// Create cached response from HTTP response
    fn create_cached_response(&self, response: &ElifResponse, ttl: Duration) -> CachedResponse {
        let mut headers = HashMap::new();
        
        // Extract headers
        for (name, value) in response.headers() {
            headers.insert(
                name.to_lowercase(),
                value.to_str().unwrap_or_default().to_string(),
            );
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        CachedResponse {
            status: response.status().as_u16(),
            headers: headers.clone(),
            body: response.body().unwrap_or_default(),
            cached_at: now,
            ttl: ttl.as_secs(),
            etag: headers.get("etag").cloned(),
            last_modified: headers.get("last-modified").cloned(),
        }
    }
    
    /// Handle conditional requests (ETag, If-Modified-Since)
    fn handle_conditional_request(
        &self,
        request: &ElifRequest,
        cached_response: &CachedResponse,
    ) -> Option<ElifResponse> {
        // Handle If-None-Match (ETag)
        if let (Some(if_none_match), Some(etag)) = (
            request.header("if-none-match"),
            &cached_response.etag,
        ) {
            if if_none_match == etag {
                return Some(
                    ResponseBuilder::new()
                        .status(304)
                        .header("ETag", etag)
                        .header("X-Cache", "HIT-CONDITIONAL")
                        .build(),
                );
            }
        }
        
        // Handle If-Modified-Since
        if let (Some(if_modified_since), Some(last_modified)) = (
            request.header("if-modified-since"),
            &cached_response.last_modified,
        ) {
            if if_modified_since == last_modified {
                return Some(
                    ResponseBuilder::new()
                        .status(304)
                        .header("Last-Modified", last_modified)
                        .header("X-Cache", "HIT-CONDITIONAL")
                        .build(),
                );
            }
        }
        
        None
    }
    
    /// Extract TTL from response headers or use default
    fn extract_ttl(&self, response: &ElifResponse) -> Duration {
        // Try to extract from Cache-Control max-age
        if let Some(cache_control) = response.header("cache-control") {
            for directive in cache_control.split(',') {
                let directive = directive.trim();
                if directive.starts_with("max-age=") {
                    if let Ok(seconds) = directive[8..].parse::<u64>() {
                        return Duration::from_secs(seconds);
                    }
                }
            }
        }
        
        // Try to extract from Expires header
        if let Some(expires) = response.header("expires") {
            // In a real implementation, you'd parse the HTTP date
            // For now, just use default
        }
        
        self.config.default_ttl
    }
}

#[async_trait]
impl<B: CacheBackend> Middleware for ResponseCacheMiddleware<B> {
    async fn handle(
        &self,
        request: ElifRequest,
        next: Box<dyn Fn(ElifRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ElifResponse, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>,
    ) -> Result<ElifResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Skip caching if not appropriate
        if !self.should_cache_request(&request) {
            return next(request).await;
        }
        
        let cache_key = self.generate_cache_key(&request);
        
        // Try to get from cache
        if let Ok(Some(cached_bytes)) = self.cache.get::<Vec<u8>>(&cache_key).await {
            if let Ok(cached_response) = serde_json::from_slice::<CachedResponse>(&cached_bytes) {
                if cached_response.is_valid() {
                    // Handle conditional requests
                    if let Some(conditional_response) = 
                        self.handle_conditional_request(&request, &cached_response) {
                        return Ok(conditional_response);
                    }
                    
                    // Return cached response
                    return Ok(cached_response.to_response());
                }
            }
        }
        
        // Not in cache or expired, call next middleware
        let response = next(request).await?;
        
        // Cache the response if appropriate
        if self.should_cache_response(&response) {
            let ttl = self.extract_ttl(&response);
            let cached_response = self.create_cached_response(&response, ttl);
            
            // Store in cache (fire and forget)
            if let Ok(cached_bytes) = serde_json::to_vec(&cached_response) {
                let _ = self.cache.put(&cache_key, &cached_bytes, ttl).await;
            }
            
            // Add cache headers to response
            let mut response_with_headers = response;
            response_with_headers = response_with_headers
                .header("X-Cache", "MISS")
                .header("Cache-Control", &format!("public, max-age={}", ttl.as_secs()));
            
            return Ok(response_with_headers);
        }
        
        Ok(response)
    }
}

/// Builder for HTTP cache middleware configuration
pub struct ResponseCacheBuilder {
    config: HttpCacheConfig,
}

impl ResponseCacheBuilder {
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
    
    pub fn build<B: CacheBackend>(self, cache: Cache<B>) -> ResponseCacheMiddleware<B> {
        ResponseCacheMiddleware::new(cache, self.config)
    }
}

impl Default for ResponseCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::MemoryBackend;
    use crate::config::CacheConfig;
    use elif_http::{request::RequestBuilder, response::ResponseBuilder};
    
    #[tokio::test]
    async fn test_cache_key_generation() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let middleware = ResponseCacheMiddleware::with_defaults(cache);
        
        let request = RequestBuilder::new()
            .method("GET")
            .uri("https://example.com/api/users")
            .header("Accept", "application/json")
            .build();
        
        let key1 = middleware.generate_cache_key(&request);
        let key2 = middleware.generate_cache_key(&request);
        
        // Same request should generate same key
        assert_eq!(key1, key2);
        
        // Different request should generate different key
        let request2 = RequestBuilder::new()
            .method("GET")
            .uri("https://example.com/api/posts")
            .header("Accept", "application/json")
            .build();
            
        let key3 = middleware.generate_cache_key(&request2);
        assert_ne!(key1, key3);
    }
    
    #[tokio::test]
    async fn test_should_cache_request() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let middleware = ResponseCacheMiddleware::with_defaults(cache);
        
        // GET request should be cached
        let get_request = RequestBuilder::new()
            .method("GET")
            .uri("/api/users")
            .build();
        assert!(middleware.should_cache_request(&get_request));
        
        // POST request should not be cached
        let post_request = RequestBuilder::new()
            .method("POST")
            .uri("/api/users")
            .build();
        assert!(!middleware.should_cache_request(&post_request));
        
        // Admin paths should not be cached
        let admin_request = RequestBuilder::new()
            .method("GET")
            .uri("/admin/users")
            .build();
        assert!(!middleware.should_cache_request(&admin_request));
        
        // Requests with no-cache header should not be cached
        let no_cache_request = RequestBuilder::new()
            .method("GET")
            .uri("/api/users")
            .header("Cache-Control", "no-cache")
            .build();
        assert!(!middleware.should_cache_request(&no_cache_request));
    }
    
    #[tokio::test]
    async fn test_should_cache_response() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let middleware = ResponseCacheMiddleware::with_defaults(cache);
        
        // 200 response should be cached
        let success_response = ResponseBuilder::new()
            .status(200)
            .body(b"success".to_vec())
            .build();
        assert!(middleware.should_cache_response(&success_response));
        
        // 404 response should not be cached (with only_success=true)
        let error_response = ResponseBuilder::new()
            .status(404)
            .body(b"not found".to_vec())
            .build();
        assert!(!middleware.should_cache_response(&error_response));
        
        // Response with no-cache header should not be cached
        let no_cache_response = ResponseBuilder::new()
            .status(200)
            .header("Cache-Control", "no-cache")
            .body(b"no cache".to_vec())
            .build();
        assert!(!middleware.should_cache_response(&no_cache_response));
    }
    
    #[tokio::test]
    async fn test_cached_response_validity() {
        let cached_response = CachedResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"test".to_vec(),
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - 100, // Cached 100 seconds ago
            ttl: 300, // 5 minutes TTL
            etag: None,
            last_modified: None,
        };
        
        assert!(cached_response.is_valid());
        
        let expired_response = CachedResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"test".to_vec(),
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - 400, // Cached 400 seconds ago
            ttl: 300, // 5 minutes TTL
            etag: None,
            last_modified: None,
        };
        
        assert!(!expired_response.is_valid());
    }
    
    #[test]
    fn test_response_cache_builder() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        
        let middleware = ResponseCacheBuilder::new()
            .default_ttl(Duration::from_secs(600))
            .vary_by_headers(vec!["Authorization".to_string()])
            .skip_paths(vec!["/api/private".to_string()])
            .per_user_cache()
            .build(cache);
        
        assert_eq!(middleware.config.default_ttl, Duration::from_secs(600));
        assert!(middleware.config.vary_by_headers.contains(&"Authorization".to_string()));
        assert!(middleware.config.skip_paths.contains(&"/api/private".to_string()));
        assert!(middleware.config.per_user);
    }
}