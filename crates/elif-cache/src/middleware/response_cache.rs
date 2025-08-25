//! HTTP Response Caching Middleware for elif.rs
//!
//! This middleware implements HTTP response caching with comprehensive support for
//! HTTP caching standards including ETags, conditional requests, Vary headers,
//! and intelligent cache key generation.
//!
//! Unlike traditional middleware that works at the axum level, this middleware
//! integrates directly with elif-http framework types and provides a seamless
//! caching experience for elif.rs applications.

use crate::{Cache, CacheBackend, CacheResult};
use blake3::Hasher;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error, warn};

// Re-export HTTP types when the feature is enabled
#[cfg(feature = "http-cache")]
use elif_http::{
    middleware::{BoxFuture, Middleware},
    request::ElifRequest,
    response::{ElifResponse, ElifStatusCode},
};

// Use axum types for internal middleware processing only
#[cfg(feature = "http-cache")]
use axum::{
    extract::Request,
    http::HeaderValue,
    response::{IntoResponse, Response},
};

/// HTTP Response Caching Configuration
#[derive(Debug, Clone)]
pub struct HttpCacheConfig {
    /// Default TTL for cached responses
    pub default_ttl: Duration,

    /// Maximum response size to cache (bytes)
    pub max_response_size: usize,

    /// Headers to include in Vary response and cache key
    pub vary_headers: Vec<String>,

    /// Cache key prefix
    pub key_prefix: String,

    /// Paths to exclude from caching (supports glob patterns like /admin/*)
    pub exclude_paths: Vec<String>,

    /// HTTP methods to exclude from caching
    pub exclude_methods: Vec<String>,

    /// Only cache successful responses (2xx status codes)
    pub only_success_responses: bool,

    /// Enable automatic ETag generation for responses
    pub enable_etag: bool,

    /// Enable conditional request handling (If-None-Match, If-Modified-Since)
    pub enable_conditional_requests: bool,

    /// Cache private responses (otherwise only public responses)
    pub cache_private_responses: bool,
}

impl Default for HttpCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_response_size: 1024 * 1024,        // 1MB
            vary_headers: vec![
                "Accept".to_string(),
                "Accept-Encoding".to_string(),
                "Accept-Language".to_string(),
            ],
            key_prefix: "http_response:".to_string(),
            exclude_paths: vec!["/admin/*".to_string(), "/api/auth/*".to_string()],
            exclude_methods: vec![
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
            ],
            only_success_responses: true,
            enable_etag: true,
            enable_conditional_requests: true,
            cache_private_responses: false,
        }
    }
}

/// Cached HTTP response data structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedHttpResponse {
    /// HTTP status code
    pub status: u16,

    /// Response headers as string map
    pub headers: HashMap<String, String>,

    /// Response body content
    pub body: Vec<u8>,

    /// Unix timestamp when cached
    pub cached_at: u64,

    /// TTL in seconds
    pub ttl_seconds: u64,

    /// Generated or original ETag
    pub etag: Option<String>,

    /// Last-Modified header value
    pub last_modified: Option<String>,

    /// Content-Type header value
    pub content_type: Option<String>,
}

impl CachedHttpResponse {
    /// Create new cached response
    pub fn new(
        status: u16,
        headers: HashMap<String, String>,
        body: Vec<u8>,
        ttl: Duration,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            status,
            etag: headers.get("etag").cloned(),
            last_modified: headers.get("last-modified").cloned(),
            content_type: headers.get("content-type").cloned(),
            headers,
            body,
            cached_at: now,
            ttl_seconds: ttl.as_secs(),
        }
    }

    /// Check if cached response is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now < self.cached_at + self.ttl_seconds
    }

    /// Generate ETag for response content
    pub fn generate_etag(&mut self) {
        if self.etag.is_none() {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&self.body);
            hasher.update(&self.status.to_be_bytes());

            // Use first 16 chars of hex for shorter ETag
            let hash_hex = hex::encode(&hasher.finalize().as_bytes()[..8]);
            let etag = format!("\"{}\"", hash_hex);

            self.etag = Some(etag.clone());
            self.headers.insert("etag".to_string(), etag);
        }
    }

    /// Convert to elif Response
    pub fn to_elif_response(&self) -> CacheResult<ElifResponse> {
        let mut response = ElifResponse::with_status(
            ElifStatusCode::from_u16(self.status)
                .map_err(|e| crate::CacheError::Backend(format!("Invalid status code: {}", e)))?,
        );

        // Add all cached headers
        for (name, value) in &self.headers {
            response = response
                .header(name, value)
                .map_err(|e| crate::CacheError::Backend(format!("Invalid header: {}", e)))?;
        }

        // Add cache-specific headers
        response = response.header("X-Cache", "HIT").map_err(|e| {
            crate::CacheError::Backend(format!("Failed to add cache header: {}", e))
        })?;

        // Set body based on content
        if !self.body.is_empty() {
            if let Some(ref content_type) = self.content_type {
                if content_type.contains("application/json") {
                    // Parse JSON and set as JSON response
                    let json_value: serde_json::Value = serde_json::from_slice(&self.body)
                        .map_err(|e| crate::CacheError::Serialization(e))?;
                    response = response.json_value(json_value);
                } else {
                    // Set as text
                    let text = String::from_utf8_lossy(&self.body).to_string();
                    response = response.text(text);
                }
            } else {
                // Default to text
                let text = String::from_utf8_lossy(&self.body).to_string();
                response = response.text(text);
            }
        }

        Ok(response)
    }
}

/// HTTP Response Caching Middleware
pub struct HttpResponseCacheMiddleware<B: CacheBackend> {
    cache: Arc<Cache<B>>,
    config: HttpCacheConfig,
}

impl<B: CacheBackend> HttpResponseCacheMiddleware<B> {
    /// Create new HTTP response caching middleware
    pub fn new(cache: Cache<B>, config: HttpCacheConfig) -> Self {
        Self {
            cache: Arc::new(cache),
            config,
        }
    }

    /// Create middleware with default configuration
    pub fn with_defaults(cache: Cache<B>) -> Self {
        Self::new(cache, HttpCacheConfig::default())
    }

    /// Generate cache key for request
    pub fn generate_cache_key(&self, request: &ElifRequest) -> String {
        let mut hasher = Hasher::new();

        // Include method and path
        hasher.update(request.method.as_str().as_bytes());
        hasher.update(request.uri.path().as_bytes());

        // Include query string if present
        if let Some(query) = request.uri.query() {
            hasher.update(b"?");
            hasher.update(query.as_bytes());
        }

        // Include vary headers in sorted order for consistency
        let mut vary_values: Vec<(String, String)> = Vec::new();
        for header_name in &self.config.vary_headers {
            if let Ok(Some(header_value)) = request.header_string(header_name) {
                vary_values.push((header_name.to_lowercase(), header_value));
            }
        }

        // Sort for consistent keys
        vary_values.sort();

        for (name, value) in vary_values {
            hasher.update(b":");
            hasher.update(name.as_bytes());
            hasher.update(b"=");
            hasher.update(value.as_bytes());
        }

        let hash = hasher.finalize();
        format!("{}{}", self.config.key_prefix, hex::encode(hash.as_bytes()))
    }

    /// Check if request should be cached
    pub fn should_cache_request(&self, request: &ElifRequest) -> bool {
        // Check HTTP method
        let method_str = request.method.as_str().to_uppercase();
        if self.config.exclude_methods.contains(&method_str) {
            debug!("Skipping cache for excluded method: {}", method_str);
            return false;
        }

        // Check excluded paths
        let path = request.path();
        for exclude_pattern in &self.config.exclude_paths {
            if self.path_matches_glob(path, exclude_pattern) {
                debug!(
                    "Skipping cache for excluded path: {} (pattern: {})",
                    path, exclude_pattern
                );
                return false;
            }
        }

        // Check Cache-Control request header
        if let Ok(Some(cache_control)) = request.header_string("cache-control") {
            if cache_control.contains("no-cache") || cache_control.contains("no-store") {
                debug!(
                    "Skipping cache due to request Cache-Control: {}",
                    cache_control
                );
                return false;
            }
        }

        true
    }

    /// Simple glob pattern matching
    fn path_matches_glob(&self, path: &str, pattern: &str) -> bool {
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }

    /// Check conditional requests (If-None-Match, If-Modified-Since)
    pub fn handle_conditional_request(
        &self,
        request: &ElifRequest,
        cached: &CachedHttpResponse,
    ) -> Option<ElifResponse> {
        if !self.config.enable_conditional_requests {
            return None;
        }

        // Handle If-None-Match (ETag validation)
        if let Ok(Some(if_none_match)) = request.header_string("if-none-match") {
            if let Some(ref etag) = cached.etag {
                if if_none_match == "*" || if_none_match.contains(etag) {
                    debug!("Returning 304 Not Modified for ETag match: {}", etag);

                    let response = ElifResponse::with_status(ElifStatusCode::NOT_MODIFIED)
                        .header("etag", etag)
                        .unwrap_or_else(|_| ElifResponse::with_status(ElifStatusCode::NOT_MODIFIED))
                        .header("x-cache", "CONDITIONAL")
                        .unwrap_or_else(|_| {
                            ElifResponse::with_status(ElifStatusCode::NOT_MODIFIED)
                        });

                    return Some(response);
                }
            }
        }

        // Handle If-Modified-Since (Last-Modified validation)
        if let Ok(Some(if_modified_since)) = request.header_string("if-modified-since") {
            if let Some(ref last_modified) = cached.last_modified {
                // Simple string comparison (in production, should parse HTTP dates)
                if if_modified_since == *last_modified {
                    debug!(
                        "Returning 304 Not Modified for Last-Modified match: {}",
                        last_modified
                    );

                    let response = ElifResponse::with_status(ElifStatusCode::NOT_MODIFIED)
                        .header("last-modified", last_modified)
                        .unwrap_or_else(|_| ElifResponse::with_status(ElifStatusCode::NOT_MODIFIED))
                        .header("x-cache", "CONDITIONAL")
                        .unwrap_or_else(|_| {
                            ElifResponse::with_status(ElifStatusCode::NOT_MODIFIED)
                        });

                    return Some(response);
                }
            }
        }

        None
    }

    /// Extract response data for caching
    pub fn extract_response_data(
        &self,
        response: &Response,
    ) -> Option<(HashMap<String, String>, Vec<u8>)> {
        let status = response.status();

        // Check if response should be cached
        if self.config.only_success_responses && !status.is_success() {
            debug!("Not caching non-success response: {}", status);
            return None;
        }

        // Check Cache-Control response headers
        if let Some(cache_control) = response.headers().get("cache-control") {
            if let Ok(value) = cache_control.to_str() {
                if value.contains("no-cache") || value.contains("no-store") {
                    debug!("Not caching due to Cache-Control: {}", value);
                    return None;
                }

                if !self.config.cache_private_responses && value.contains("private") {
                    debug!("Not caching private response");
                    return None;
                }
            }
        }

        // Extract headers
        let mut headers = HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        // TODO: Extract response body
        // This is tricky with axum Response as the body is consumed when read
        // For now, return empty body - this would need framework support
        let body = Vec::new();

        Some((headers, body))
    }

    /// Extract TTL from response headers or use default
    pub fn extract_ttl(&self, headers: &HashMap<String, String>) -> Duration {
        if let Some(cache_control) = headers.get("cache-control") {
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

// Implement Middleware trait for axum integration
#[cfg(feature = "http-cache")]
impl<B: CacheBackend + 'static> Middleware for HttpResponseCacheMiddleware<B> {
    fn process_request<'a>(&'a self, request: Request) -> BoxFuture<'a, Result<Request, Response>> {
        Box::pin(async move {
            // Convert axum Request to ElifRequest for analysis
            let elif_request = ElifRequest::new(
                request.method().clone(),
                request.uri().clone(),
                request.headers().clone(),
            );

            // Check if we should attempt caching for this request
            if !self.should_cache_request(&elif_request) {
                return Ok(request);
            }

            // Generate cache key
            let cache_key = self.generate_cache_key(&elif_request);
            debug!("Checking cache for key: {}", cache_key);

            // Try to get cached response
            match self.cache.get::<CachedHttpResponse>(&cache_key).await {
                Ok(Some(cached_response)) => {
                    if cached_response.is_valid() {
                        // Check for conditional requests first
                        if let Some(conditional_response) =
                            self.handle_conditional_request(&elif_request, &cached_response)
                        {
                            debug!("Returning conditional response (304)");
                            // Convert ElifResponse to axum Response using into_response
                            return Err(conditional_response.into_response());
                        }

                        // Return full cached response
                        match cached_response.to_elif_response() {
                            Ok(elif_response) => {
                                debug!("Cache HIT for key: {}", cache_key);
                                // Convert ElifResponse to axum Response using into_response
                                return Err(elif_response.into_response());
                            }
                            Err(e) => {
                                warn!("Failed to convert cached response: {}", e);
                            }
                        }
                    } else {
                        debug!("Cached response expired for key: {}", cache_key);
                        // Remove expired entry asynchronously
                        let cache = self.cache.clone();
                        let key = cache_key.clone();
                        tokio::spawn(async move {
                            if let Err(e) = cache.forget(&key).await {
                                warn!("Failed to remove expired cache entry {}: {}", key, e);
                            }
                        });
                    }
                }
                Ok(None) => {
                    debug!("Cache MISS for key: {}", cache_key);
                }
                Err(e) => {
                    error!("Cache lookup error for key {}: {}", cache_key, e);
                }
            }

            // Cache miss - proceed with request
            // TODO: Store cache key in request for response processing
            Ok(request)
        })
    }

    fn process_response<'a>(&'a self, response: Response) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // TODO: Implement response caching
            // This requires access to the original request context to:
            // 1. Get the cache key
            // 2. Check if response should be cached
            // 3. Extract response body and store in cache
            // 4. Add appropriate cache headers

            // For now, just add cache miss header and return response
            let mut response = response;
            if let Ok(header_value) = HeaderValue::from_str("MISS") {
                response.headers_mut().insert("x-cache", header_value);
            }

            response
        })
    }

    fn name(&self) -> &'static str {
        "HttpResponseCache"
    }
}

/// Builder for HTTP response cache middleware configuration
pub struct HttpCacheBuilder {
    config: HttpCacheConfig,
}

impl HttpCacheBuilder {
    /// Create new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: HttpCacheConfig::default(),
        }
    }

    /// Set default TTL for cached responses
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.config.default_ttl = ttl;
        self
    }

    /// Set maximum response size to cache
    pub fn max_response_size(mut self, size: usize) -> Self {
        self.config.max_response_size = size;
        self
    }

    /// Set headers to vary by
    pub fn vary_by_headers(mut self, headers: Vec<String>) -> Self {
        self.config.vary_headers = headers;
        self
    }

    /// Add a header to vary by
    pub fn vary_by_header(mut self, header: impl Into<String>) -> Self {
        self.config.vary_headers.push(header.into());
        self
    }

    /// Set cache key prefix
    pub fn key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.key_prefix = prefix.into();
        self
    }

    /// Set paths to exclude from caching
    pub fn exclude_paths(mut self, paths: Vec<String>) -> Self {
        self.config.exclude_paths = paths;
        self
    }

    /// Add a path pattern to exclude
    pub fn exclude_path(mut self, path: impl Into<String>) -> Self {
        self.config.exclude_paths.push(path.into());
        self
    }

    /// Set HTTP methods to exclude
    pub fn exclude_methods(mut self, methods: Vec<String>) -> Self {
        self.config.exclude_methods = methods;
        self
    }

    /// Enable caching of non-success responses
    pub fn cache_all_responses(mut self) -> Self {
        self.config.only_success_responses = false;
        self
    }

    /// Disable ETag generation
    pub fn disable_etag(mut self) -> Self {
        self.config.enable_etag = false;
        self
    }

    /// Disable conditional request handling
    pub fn disable_conditional_requests(mut self) -> Self {
        self.config.enable_conditional_requests = false;
        self
    }

    /// Enable caching of private responses
    pub fn cache_private_responses(mut self) -> Self {
        self.config.cache_private_responses = true;
        self
    }

    /// Build the middleware with the given cache backend
    pub fn build<B: CacheBackend>(self, cache: Cache<B>) -> HttpResponseCacheMiddleware<B> {
        HttpResponseCacheMiddleware::new(cache, self.config)
    }
}

impl Default for HttpCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "http-cache"))]
mod tests {
    use super::*;
    use crate::{CacheConfig, MemoryBackend};
    use axum::http::{Method, Uri};

    fn create_test_request(method: Method, uri: &str, headers: Vec<(&str, &str)>) -> ElifRequest {
        let mut header_map = HeaderMap::new();
        for (name, value) in headers {
            header_map.insert(name, value.parse().unwrap());
        }

        ElifRequest::new(method, uri.parse().unwrap(), header_map)
    }

    #[test]
    fn test_cache_key_generation() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let middleware = HttpResponseCacheMiddleware::with_defaults(cache);

        let request1 = create_test_request(Method::GET, "/api/users", vec![]);
        let request2 = create_test_request(Method::GET, "/api/users", vec![]);

        let key1 = middleware.generate_cache_key(&request1);
        let key2 = middleware.generate_cache_key(&request2);

        // Same requests should generate same keys
        assert_eq!(key1, key2);

        let request3 = create_test_request(Method::GET, "/api/posts", vec![]);
        let key3 = middleware.generate_cache_key(&request3);

        // Different requests should generate different keys
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_vary_header_cache_keys() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let middleware = HttpResponseCacheMiddleware::with_defaults(cache);

        let request1 = create_test_request(
            Method::GET,
            "/api/users",
            vec![("accept", "application/json")],
        );

        let request2 = create_test_request(
            Method::GET,
            "/api/users",
            vec![("accept", "application/xml")],
        );

        let key1 = middleware.generate_cache_key(&request1);
        let key2 = middleware.generate_cache_key(&request2);

        // Different Accept headers should generate different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_should_cache_request() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let middleware = HttpResponseCacheMiddleware::with_defaults(cache);

        // GET should be cached
        let get_request = create_test_request(Method::GET, "/api/users", vec![]);
        assert!(middleware.should_cache_request(&get_request));

        // POST should not be cached
        let post_request = create_test_request(Method::POST, "/api/users", vec![]);
        assert!(!middleware.should_cache_request(&post_request));

        // Admin paths should not be cached
        let admin_request = create_test_request(Method::GET, "/admin/users", vec![]);
        assert!(!middleware.should_cache_request(&admin_request));

        // No-cache header should prevent caching
        let no_cache_request = create_test_request(
            Method::GET,
            "/api/users",
            vec![("cache-control", "no-cache")],
        );
        assert!(!middleware.should_cache_request(&no_cache_request));
    }

    #[test]
    fn test_path_glob_matching() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);
        let middleware = HttpResponseCacheMiddleware::with_defaults(cache);

        // Exact match
        assert!(middleware.path_matches_glob("/admin", "/admin"));

        // Wildcard match
        assert!(middleware.path_matches_glob("/admin/users", "/admin/*"));
        assert!(middleware.path_matches_glob("/admin/posts/123", "/admin/*"));

        // No match
        assert!(!middleware.path_matches_glob("/api/users", "/admin/*"));
        assert!(!middleware.path_matches_glob("/admin", "/api"));
    }

    #[test]
    fn test_cached_response_validity() {
        let headers = HashMap::new();
        let body = b"test response".to_vec();

        // Valid response (5 minute TTL)
        let valid_response =
            CachedHttpResponse::new(200, headers.clone(), body.clone(), Duration::from_secs(300));
        assert!(valid_response.is_valid());

        // Expired response (created in the past)
        let mut expired_response =
            CachedHttpResponse::new(200, headers, body, Duration::from_secs(300));
        expired_response.cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 400; // 400 seconds ago
        assert!(!expired_response.is_valid());
    }

    #[test]
    fn test_etag_generation() {
        let headers = HashMap::new();
        let body = b"Hello, World!".to_vec();

        let mut response = CachedHttpResponse::new(200, headers, body, Duration::from_secs(300));
        assert!(response.etag.is_none());

        response.generate_etag();
        assert!(response.etag.is_some());

        let etag = response.etag.unwrap();
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert!(response.headers.contains_key("etag"));
    }

    #[test]
    fn test_builder_configuration() {
        let backend = MemoryBackend::new(CacheConfig::default());
        let cache = Cache::new(backend);

        let middleware = HttpCacheBuilder::new()
            .default_ttl(Duration::from_secs(600))
            .max_response_size(2 * 1024 * 1024)
            .vary_by_header("Authorization")
            .exclude_path("/api/private")
            .cache_all_responses()
            .disable_etag()
            .build(cache);

        assert_eq!(middleware.config.default_ttl, Duration::from_secs(600));
        assert_eq!(middleware.config.max_response_size, 2 * 1024 * 1024);
        assert!(middleware
            .config
            .vary_headers
            .contains(&"Authorization".to_string()));
        assert!(middleware
            .config
            .exclude_paths
            .contains(&"/api/private".to_string()));
        assert!(!middleware.config.only_success_responses);
        assert!(!middleware.config.enable_etag);
    }
}
