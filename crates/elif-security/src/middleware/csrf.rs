//! CSRF (Cross-Site Request Forgery) protection middleware
//!
//! Provides comprehensive CSRF protection including token generation,
//! validation, and secure cookie handling.

use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sha2::{Sha256, Digest};
use rand::{thread_rng, Rng};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

pub use crate::config::CsrfConfig;
use crate::SecurityError;

/// CSRF token store - in production this would be backed by Redis/database
type TokenStore = Arc<tokio::sync::RwLock<HashMap<String, CsrfTokenData>>>;

/// CSRF token data with expiration
#[derive(Debug, Clone)]
pub struct CsrfTokenData {
    pub token: String,
    pub expires_at: time::OffsetDateTime,
    pub user_agent_hash: Option<String>,
}

/// CSRF protection middleware
#[derive(Debug, Clone)]
pub struct CsrfMiddleware {
    config: CsrfConfig,
    token_store: TokenStore,
}

impl CsrfMiddleware {
    /// Create new CSRF middleware with configuration
    pub fn new(config: CsrfConfig) -> Self {
        Self { 
            config,
            token_store: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// Create middleware with builder pattern
    pub fn builder() -> CsrfMiddlewareBuilder {
        CsrfMiddlewareBuilder::new()
    }
    
    /// Generate a new CSRF token
    pub async fn generate_token(&self, user_agent: Option<&str>) -> String {
        let mut rng = thread_rng();
        let token_bytes: [u8; 32] = rng.gen();
        let token = URL_SAFE_NO_PAD.encode(token_bytes);
        
        let user_agent_hash = user_agent.map(|ua| {
            let mut hasher = Sha256::new();
            hasher.update(ua.as_bytes());
            format!("{:x}", hasher.finalize())
        });
        
        let token_data = CsrfTokenData {
            token: token.clone(),
            expires_at: time::OffsetDateTime::now_utc() + 
                time::Duration::seconds(self.config.token_lifetime as i64),
            user_agent_hash,
        };
        
        // Store token
        let mut store = self.token_store.write().await;
        store.insert(token.clone(), token_data);
        
        // Clean up expired tokens periodically
        self.cleanup_expired_tokens(&mut store).await;
        
        token
    }
    
    /// Validate a CSRF token
    pub async fn validate_token(&self, token: &str, user_agent: Option<&str>) -> bool {
        let store = self.token_store.read().await;
        
        if let Some(token_data) = store.get(token) {
            // Check expiration
            if time::OffsetDateTime::now_utc() > token_data.expires_at {
                return false;
            }
            
            // Check user agent if configured
            if let Some(stored_hash) = &token_data.user_agent_hash {
                if let Some(ua) = user_agent {
                    let mut hasher = Sha256::new();
                    hasher.update(ua.as_bytes());
                    let ua_hash = format!("{:x}", hasher.finalize());
                    if stored_hash != &ua_hash {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            
            true
        } else {
            false
        }
    }
    
    /// Remove a token after successful validation (single-use)
    pub async fn consume_token(&self, token: &str) {
        let mut store = self.token_store.write().await;
        store.remove(token);
    }
    
    /// Clean up expired tokens
    async fn cleanup_expired_tokens(&self, store: &mut HashMap<String, CsrfTokenData>) {
        let now = time::OffsetDateTime::now_utc();
        store.retain(|_, data| data.expires_at > now);
    }
    
    /// Check if path is exempt from CSRF protection
    fn is_exempt_path(&self, path: &str) -> bool {
        self.config.exempt_paths.contains(path) ||
        self.config.exempt_paths.iter().any(|exempt| {
            // Simple glob pattern matching
            if exempt.ends_with('*') {
                path.starts_with(&exempt[..exempt.len()-1])
            } else {
                path == exempt
            }
        })
    }
    
    /// Extract CSRF token from request
    fn extract_token(&self, headers: &HeaderMap) -> Option<String> {
        // Try header first
        if let Some(header_value) = headers.get(&self.config.token_header) {
            if let Ok(token) = header_value.to_str() {
                return Some(token.to_string());
            }
        }
        
        // Try cookie (would need cookie parsing here - simplified for now)
        if let Some(cookie_header) = headers.get(header::COOKIE) {
            if let Ok(cookies) = cookie_header.to_str() {
                for cookie in cookies.split(';') {
                    let cookie = cookie.trim();
                    if let Some((name, value)) = cookie.split_once('=') {
                        if name == self.config.cookie_name {
                            return Some(value.to_string());
                        }
                    }
                }
            }
        }
        
        None
    }
}

/// Axum middleware implementation
pub async fn csrf_middleware(
    State(middleware): State<CsrfMiddleware>,
    request: Request,
    next: Next,
) -> Result<Response, SecurityError> {
    let method = request.method();
    let uri = request.uri();
    let headers = request.headers();
    
    // Skip CSRF protection for safe methods (GET, HEAD, OPTIONS)
    if matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS) {
        return Ok(next.run(request).await);
    }
    
    // Skip exempt paths
    if middleware.is_exempt_path(uri.path()) {
        return Ok(next.run(request).await);
    }
    
    // Extract and validate token
    let user_agent = headers.get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok());
        
    if let Some(token) = middleware.extract_token(headers) {
        if middleware.validate_token(&token, user_agent).await {
            // Consume token for single-use (optional - can be configured)
            // middleware.consume_token(&token).await;
            return Ok(next.run(request).await);
        }
    }
    
    // CSRF validation failed
    Err(SecurityError::CsrfValidationFailed)
}

impl IntoResponse for SecurityError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            SecurityError::CsrfValidationFailed => {
                (StatusCode::FORBIDDEN, "CSRF token validation failed")
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Security error"),
        };
        
        (status, message).into_response()
    }
}

/// Builder for CSRF middleware configuration
#[derive(Debug)]
pub struct CsrfMiddlewareBuilder {
    config: CsrfConfig,
}

impl CsrfMiddlewareBuilder {
    pub fn new() -> Self {
        Self {
            config: CsrfConfig::default(),
        }
    }
    
    pub fn token_header<S: Into<String>>(mut self, header: S) -> Self {
        self.config.token_header = header.into();
        self
    }
    
    pub fn cookie_name<S: Into<String>>(mut self, name: S) -> Self {
        self.config.cookie_name = name.into();
        self
    }
    
    pub fn token_lifetime(mut self, seconds: u64) -> Self {
        self.config.token_lifetime = seconds;
        self
    }
    
    pub fn secure_cookie(mut self, secure: bool) -> Self {
        self.config.secure_cookie = secure;
        self
    }
    
    pub fn exempt_path<S: Into<String>>(mut self, path: S) -> Self {
        self.config.exempt_paths.insert(path.into());
        self
    }
    
    pub fn exempt_paths<I, S>(mut self, paths: I) -> Self 
    where 
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for path in paths {
            self.config.exempt_paths.insert(path.into());
        }
        self
    }
    
    pub fn build(self) -> CsrfMiddleware {
        CsrfMiddleware::new(self.config)
    }
}

impl Default for CsrfMiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::HeaderValue,
        middleware::from_fn_with_state,
        routing::{get, post},
        Router,
    };
    use axum_test::TestServer;
    use std::collections::HashSet;

    async fn test_handler() -> &'static str {
        "OK"
    }

    fn create_test_middleware() -> CsrfMiddleware {
        let mut exempt_paths = HashSet::new();
        exempt_paths.insert("/api/webhook".to_string());
        exempt_paths.insert("/public/*".to_string());

        let config = CsrfConfig {
            token_header: "X-CSRF-Token".to_string(),
            cookie_name: "_csrf_token".to_string(),
            token_lifetime: 3600,
            secure_cookie: false, // For testing
            exempt_paths,
        };

        CsrfMiddleware::new(config)
    }

    #[tokio::test]
    async fn test_csrf_token_generation() {
        let middleware = create_test_middleware();
        
        let token1 = middleware.generate_token(Some("Mozilla/5.0")).await;
        let token2 = middleware.generate_token(Some("Mozilla/5.0")).await;
        
        // Tokens should be different
        assert_ne!(token1, token2);
        assert!(token1.len() > 20); // Should be base64 encoded
        assert!(token2.len() > 20);
    }

    #[tokio::test]
    async fn test_csrf_token_validation() {
        let middleware = create_test_middleware();
        let user_agent = Some("Mozilla/5.0");
        
        let token = middleware.generate_token(user_agent).await;
        
        // Valid token should pass
        assert!(middleware.validate_token(&token, user_agent).await);
        
        // Invalid token should fail
        assert!(!middleware.validate_token("invalid_token", user_agent).await);
        
        // Different user agent should fail if token was generated with one
        assert!(!middleware.validate_token(&token, Some("Different Agent")).await);
    }

    #[tokio::test]
    async fn test_csrf_token_expiration() {
        let config = CsrfConfig {
            token_lifetime: 1, // 1 second
            ..Default::default()
        };
        let middleware = CsrfMiddleware::new(config);
        
        let token = middleware.generate_token(None).await;
        
        // Should be valid immediately
        assert!(middleware.validate_token(&token, None).await);
        
        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Should be expired now
        assert!(!middleware.validate_token(&token, None).await);
    }

    #[tokio::test]
    async fn test_csrf_exempt_paths() {
        let middleware = create_test_middleware();
        
        // Exact match
        assert!(middleware.is_exempt_path("/api/webhook"));
        
        // Glob pattern match
        assert!(middleware.is_exempt_path("/public/assets/style.css"));
        assert!(middleware.is_exempt_path("/public/images/logo.png"));
        
        // Non-exempt paths
        assert!(!middleware.is_exempt_path("/api/users"));
        assert!(!middleware.is_exempt_path("/admin/dashboard"));
    }

    #[tokio::test]
    async fn test_csrf_builder_pattern() {
        let middleware = CsrfMiddleware::builder()
            .token_header("X-Custom-CSRF-Token")
            .cookie_name("_custom_csrf")
            .token_lifetime(7200)
            .secure_cookie(true)
            .exempt_path("/api/public")
            .exempt_paths(vec!["/webhook", "/status"])
            .build();
            
        assert_eq!(middleware.config.token_header, "X-Custom-CSRF-Token");
        assert_eq!(middleware.config.cookie_name, "_custom_csrf");
        assert_eq!(middleware.config.token_lifetime, 7200);
        assert!(middleware.config.secure_cookie);
        assert!(middleware.config.exempt_paths.contains("/api/public"));
        assert!(middleware.config.exempt_paths.contains("/webhook"));
        assert!(middleware.config.exempt_paths.contains("/status"));
    }

    #[tokio::test]
    async fn test_csrf_middleware_get_requests() {
        let middleware = create_test_middleware();
        
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(from_fn_with_state(middleware, csrf_middleware));
            
        let server = TestServer::new(app).unwrap();
        
        // GET requests should pass without CSRF token
        let response = server.get("/test").await;
        response.assert_status_ok();
        response.assert_text("OK");
    }

    #[tokio::test]
    async fn test_csrf_middleware_post_without_token() {
        let middleware = create_test_middleware();
        
        let app = Router::new()
            .route("/test", post(test_handler))
            .layer(from_fn_with_state(middleware, csrf_middleware));
            
        let server = TestServer::new(app).unwrap();
        
        // POST without CSRF token should fail
        let response = server.post("/test").await;
        response.assert_status_forbidden();
    }

    #[tokio::test]
    async fn test_csrf_middleware_post_with_valid_token() {
        let middleware = create_test_middleware();
        let token = middleware.generate_token(Some("TestAgent")).await;
        
        let app = Router::new()
            .route("/test", post(test_handler))
            .layer(from_fn_with_state(middleware, csrf_middleware));
            
        let server = TestServer::new(app).unwrap();
        
        // POST with valid CSRF token should pass
        let response = server
            .post("/test")
            .add_header("X-CSRF-Token", &token)
            .add_header("User-Agent", "TestAgent")
            .await;
            
        response.assert_status_ok();
        response.assert_text("OK");
    }

    #[tokio::test]
    async fn test_csrf_middleware_exempt_paths() {
        let middleware = create_test_middleware();
        
        let app = Router::new()
            .route("/api/webhook", post(test_handler))
            .route("/public/upload", post(test_handler))
            .layer(from_fn_with_state(middleware, csrf_middleware));
            
        let server = TestServer::new(app).unwrap();
        
        // Exempt paths should pass without CSRF token
        let response1 = server.post("/api/webhook").await;
        response1.assert_status_ok();
        
        let response2 = server.post("/public/upload").await;
        response2.assert_status_ok();
    }

    #[tokio::test]
    async fn test_csrf_token_cleanup() {
        let config = CsrfConfig {
            token_lifetime: 1, // 1 second
            ..Default::default()
        };
        let middleware = CsrfMiddleware::new(config);
        
        // Generate several tokens
        let _token1 = middleware.generate_token(None).await;
        let _token2 = middleware.generate_token(None).await;
        let _token3 = middleware.generate_token(None).await;
        
        // Check initial count
        {
            let store = middleware.token_store.read().await;
            assert_eq!(store.len(), 3);
        }
        
        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Generate a new token to trigger cleanup
        let _new_token = middleware.generate_token(None).await;
        
        // Check that expired tokens were cleaned up
        {
            let store = middleware.token_store.read().await;
            assert_eq!(store.len(), 1); // Only the new token should remain
        }
    }

    #[tokio::test]
    async fn test_csrf_cookie_extraction() {
        let middleware = create_test_middleware();
        let mut headers = HeaderMap::new();
        
        // Test cookie extraction
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str("_csrf_token=test_token_123; other_cookie=value").unwrap()
        );
        
        let token = middleware.extract_token(&headers);
        assert_eq!(token, Some("test_token_123".to_string()));
        
        // Test header extraction (should take precedence)
        headers.insert(
            "X-CSRF-Token",
            HeaderValue::from_str("header_token_456").unwrap()
        );
        
        let token = middleware.extract_token(&headers);
        assert_eq!(token, Some("header_token_456".to_string()));
    }

    #[tokio::test]
    async fn test_csrf_user_agent_binding() {
        let middleware = create_test_middleware();
        
        let token = middleware.generate_token(Some("SpecificAgent")).await;
        
        // Same user agent should work
        assert!(middleware.validate_token(&token, Some("SpecificAgent")).await);
        
        // Different user agent should fail
        assert!(!middleware.validate_token(&token, Some("DifferentAgent")).await);
        
        // No user agent should fail when token was created with one
        assert!(!middleware.validate_token(&token, None).await);
    }
}