//! CSRF (Cross-Site Request Forgery) protection middleware
//!
//! Provides comprehensive CSRF protection including token generation,
//! validation, and secure cookie handling.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use elif_http::{
    middleware::v2::{Middleware, Next, NextFuture},
    request::{ElifMethod, ElifRequest},
    response::{ElifResponse, ElifStatusCode},
};
use rand::{thread_rng, Rng};
use serde_json;
use service_builder::builder;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

pub use crate::config::CsrfConfig;

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
    pub fn builder() -> CsrfMiddlewareConfigBuilder {
        CsrfMiddlewareConfig::builder()
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
            expires_at: time::OffsetDateTime::now_utc()
                + time::Duration::seconds(self.config.token_lifetime as i64),
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
        Self::validate_token_internal(&self.token_store, token, user_agent).await
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

    /// Internal token validation logic
    async fn validate_token_internal(
        store: &TokenStore,
        token: &str,
        user_agent: Option<&str>,
    ) -> bool {
        let store_guard = store.read().await;
        let Some(token_data) = store_guard.get(token) else {
            return false;
        };

        if time::OffsetDateTime::now_utc() > token_data.expires_at {
            return false;
        }

        if let Some(stored_hash) = &token_data.user_agent_hash {
            let Some(ua) = user_agent else {
                return false;
            };
            let mut hasher = sha2::Sha256::new();
            hasher.update(ua.as_bytes());
            let ua_hash = format!("{:x}", hasher.finalize());
            if stored_hash != &ua_hash {
                return false;
            }
        }

        true
    }

    /// Check if path is exempt from CSRF protection
    fn is_exempt_path(&self, path: &str) -> bool {
        self.config.exempt_paths.contains(path)
            || self.config.exempt_paths.iter().any(|exempt| {
                // Simple glob pattern matching
                if exempt.ends_with('*') {
                    path.starts_with(&exempt[..exempt.len() - 1])
                } else {
                    path == exempt
                }
            })
    }

    /// Extract CSRF token from request
    fn extract_token(&self, headers: &elif_http::response::ElifHeaderMap) -> Option<String> {
        // Try header first
        if let Some(header_value) = headers.get_str(&self.config.token_header) {
            if let Ok(token) = header_value.to_str() {
                return Some(token.to_string());
            }
        }

        // Try cookie (would need cookie parsing here - simplified for now)
        if let Some(cookie_header) = headers.get_str("cookie") {
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

/// Implementation of our Middleware trait for CSRF protection
impl Middleware for CsrfMiddleware {
    fn handle(&self, request: ElifRequest, next: Next) -> NextFuture<'static> {
        // Check if exempt before async block
        let is_exempt = self.is_exempt_path(request.uri.path());
        let token = self.extract_token(&request.headers);
        let store = self.token_store.clone();

        Box::pin(async move {
            // Skip CSRF protection for safe methods (GET, HEAD, OPTIONS)
            if matches!(
                request.method,
                ElifMethod::GET | ElifMethod::HEAD | ElifMethod::OPTIONS
            ) {
                return next.run(request).await;
            }

            // Skip exempt paths
            if is_exempt {
                return next.run(request).await;
            }

            // Extract and validate token
            let user_agent = request
                .headers
                .get_str("user-agent")
                .and_then(|h| h.to_str().ok());

            if let Some(token) = token {
                let is_valid = Self::validate_token_internal(&store, &token, user_agent).await;

                if is_valid {
                    // Consume token for single-use (optional - can be configured)
                    // self.consume_token(&token).await;
                    return next.run(request).await;
                }
            }

            // CSRF validation failed - return 403 Forbidden
            let error_data = serde_json::json!({
                "error": {
                    "code": "CSRF_VALIDATION_FAILED",
                    "message": "CSRF token validation failed"
                }
            });
            ElifResponse::with_status(ElifStatusCode::FORBIDDEN)
                .json(&error_data)
                .unwrap_or_else(|_| {
                    ElifResponse::with_status(ElifStatusCode::INTERNAL_SERVER_ERROR)
                        .text("Internal server error")
                })
        })
    }

    fn name(&self) -> &'static str {
        "CsrfMiddleware"
    }
}

/// Configuration for CSRF middleware builder  
#[derive(Debug, Clone)]
#[builder]
pub struct CsrfMiddlewareConfig {
    #[builder(default = "String::from(\"X-CSRF-Token\")")]
    pub token_header: String,
    #[builder(default = "String::from(\"_csrf_token\")")]
    pub cookie_name: String,
    #[builder(default = "3600")]
    pub token_lifetime: u64,
    #[builder(default)]
    pub secure_cookie: bool,
    #[builder(default)]
    pub exempt_paths: std::collections::HashSet<String>,
}

impl CsrfMiddlewareConfig {
    pub fn build_middleware(self) -> CsrfMiddleware {
        let config = CsrfConfig {
            token_header: self.token_header,
            cookie_name: self.cookie_name,
            token_lifetime: self.token_lifetime,
            secure_cookie: self.secure_cookie,
            exempt_paths: self.exempt_paths,
        };
        CsrfMiddleware::new(config)
    }
}

// Add convenience methods to the generated builder
impl CsrfMiddlewareConfigBuilder {
    pub fn token_header_str<S: Into<String>>(self, header: S) -> Self {
        self.token_header(header.into())
    }

    pub fn cookie_name_str<S: Into<String>>(self, name: S) -> Self {
        self.cookie_name(name.into())
    }

    pub fn exempt_path<S: Into<String>>(self, path: S) -> Self {
        let mut paths = self.exempt_paths.unwrap_or_default();
        paths.insert(path.into());
        CsrfMiddlewareConfigBuilder {
            token_header: self.token_header,
            cookie_name: self.cookie_name,
            token_lifetime: self.token_lifetime,
            secure_cookie: self.secure_cookie,
            exempt_paths: Some(paths),
        }
    }

    pub fn exempt_paths_vec<I, S>(self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut exempt_paths = self.exempt_paths.unwrap_or_default();
        for path in paths {
            exempt_paths.insert(path.into());
        }
        CsrfMiddlewareConfigBuilder {
            token_header: self.token_header,
            cookie_name: self.cookie_name,
            token_lifetime: self.token_lifetime,
            secure_cookie: self.secure_cookie,
            exempt_paths: Some(exempt_paths),
        }
    }

    pub fn build_config(self) -> CsrfMiddlewareConfig {
        self.build_with_defaults()
            .expect("Building CsrfMiddlewareConfig should not fail as all fields have defaults")
    }

    pub fn build_middleware(self) -> CsrfMiddleware {
        self.build_config().build_middleware()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elif_http::middleware::v2::MiddlewarePipelineV2;
    use elif_http::request::ElifRequest;
    use elif_http::response::ElifHeaderMap;
    use std::collections::HashSet;

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
        assert!(
            !middleware
                .validate_token(&token, Some("Different Agent"))
                .await
        );
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
            .token_header_str("X-Custom-CSRF-Token")
            .cookie_name_str("_custom_csrf")
            .token_lifetime(7200)
            .secure_cookie(true)
            .exempt_path("/api/public")
            .exempt_paths_vec(vec!["/webhook", "/status"])
            .build_middleware();

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
        let pipeline = MiddlewarePipelineV2::new().add(middleware);

        // Create GET request
        let headers = ElifHeaderMap::new();
        let request = ElifRequest::new(ElifMethod::GET, "/test".parse().unwrap(), headers);

        // GET requests should pass without CSRF token
        let response = pipeline
            .execute(request, |_req| {
                Box::pin(async move { ElifResponse::ok().text("Success") })
            })
            .await;

        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_middleware_post_without_token() {
        let middleware = create_test_middleware();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);

        // Create POST request without CSRF token
        let headers = ElifHeaderMap::new();
        let request = ElifRequest::new(ElifMethod::POST, "/test".parse().unwrap(), headers);

        // POST without CSRF token should fail
        let response = pipeline
            .execute(request, |_req| {
                Box::pin(async move { ElifResponse::ok().text("Should not reach handler") })
            })
            .await;

        // Check that it returns 403 Forbidden
        assert_eq!(response.status_code(), ElifStatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_csrf_middleware_post_with_valid_token() {
        let middleware = create_test_middleware();
        let token = middleware.generate_token(Some("TestAgent")).await;
        let pipeline = MiddlewarePipelineV2::new().add(middleware);

        // Create POST request with valid CSRF token
        let mut headers = ElifHeaderMap::new();
        headers.insert("x-csrf-token".parse().unwrap(), token.parse().unwrap());
        headers.insert("user-agent".parse().unwrap(), "TestAgent".parse().unwrap());

        let request = ElifRequest::new(ElifMethod::POST, "/test".parse().unwrap(), headers);

        // POST with valid CSRF token should pass
        let response = pipeline
            .execute(request, |_req| {
                Box::pin(async move { ElifResponse::ok().text("Success") })
            })
            .await;

        assert_eq!(response.status_code(), ElifStatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_middleware_exempt_paths() {
        let middleware = create_test_middleware();
        let pipeline = MiddlewarePipelineV2::new().add(middleware);

        // Test exempt exact path
        let headers1 = ElifHeaderMap::new();
        let request1 =
            ElifRequest::new(ElifMethod::POST, "/api/webhook".parse().unwrap(), headers1);

        let response1 = pipeline
            .execute(request1, |_req| {
                Box::pin(async move { ElifResponse::ok().text("Success") })
            })
            .await;

        assert_eq!(response1.status_code(), ElifStatusCode::OK);

        // Test exempt glob path
        let headers2 = ElifHeaderMap::new();
        let request2 = ElifRequest::new(
            ElifMethod::POST,
            "/public/upload".parse().unwrap(),
            headers2,
        );

        let response2 = pipeline
            .execute(request2, |_req| {
                Box::pin(async move { ElifResponse::ok().text("Success") })
            })
            .await;

        assert_eq!(response2.status_code(), ElifStatusCode::OK);
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
        let mut headers = ElifHeaderMap::new();

        // Test cookie extraction
        headers.insert(
            "cookie".parse().unwrap(),
            "_csrf_token=test_token_123; other_cookie=value"
                .parse()
                .unwrap(),
        );

        let token = middleware.extract_token(&headers);
        assert_eq!(token, Some("test_token_123".to_string()));

        // Test header extraction (should take precedence)
        headers.insert(
            "X-CSRF-Token".parse().unwrap(),
            "header_token_456".parse().unwrap(),
        );

        let token = middleware.extract_token(&headers);
        assert_eq!(token, Some("header_token_456".to_string()));
    }

    #[tokio::test]
    async fn test_csrf_user_agent_binding() {
        let middleware = create_test_middleware();

        let token = middleware.generate_token(Some("SpecificAgent")).await;

        // Same user agent should work
        assert!(
            middleware
                .validate_token(&token, Some("SpecificAgent"))
                .await
        );

        // Different user agent should fail
        assert!(
            !middleware
                .validate_token(&token, Some("DifferentAgent"))
                .await
        );

        // No user agent should fail when token was created with one
        assert!(!middleware.validate_token(&token, None).await);
    }
}
