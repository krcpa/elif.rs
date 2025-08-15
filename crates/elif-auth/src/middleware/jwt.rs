//! JWT authentication middleware
//! 
//! Provides HTTP middleware for JWT token authentication

use crate::{
    providers::jwt::{JwtProvider, JwtToken, JwtClaims},
    traits::UserContext,
    AuthError, AuthResult,
};

/// JWT middleware for extracting and validating JWT tokens from HTTP requests
pub struct JwtMiddleware<User> {
    /// JWT provider for token operations
    provider: JwtProvider<User>,
    
    /// Token extraction configuration
    config: JwtMiddlewareConfig,
}

/// Configuration for JWT middleware
#[derive(Debug, Clone)]
pub struct JwtMiddlewareConfig {
    /// Header name for token extraction
    pub header_name: String,
    
    /// Token prefix (e.g., "Bearer ")
    pub token_prefix: String,
    
    /// Whether to skip authentication for certain paths
    pub skip_paths: Vec<String>,
    
    /// Whether authentication is optional
    pub optional: bool,
}

impl Default for JwtMiddlewareConfig {
    fn default() -> Self {
        Self {
            header_name: "Authorization".to_string(),
            token_prefix: "Bearer ".to_string(),
            skip_paths: vec!["/health".to_string(), "/metrics".to_string()],
            optional: false,
        }
    }
}

impl<User> JwtMiddleware<User> {
    /// Create a new JWT middleware
    pub fn new(provider: JwtProvider<User>) -> Self {
        Self::with_config(provider, JwtMiddlewareConfig::default())
    }
    
    /// Create a new JWT middleware with custom configuration
    pub fn with_config(provider: JwtProvider<User>, config: JwtMiddlewareConfig) -> Self {
        Self { provider, config }
    }
    
    /// Extract token from Authorization header
    pub fn extract_token(&self, auth_header: Option<&str>) -> AuthResult<Option<String>> {
        match auth_header {
            Some(header_value) => {
                if header_value.starts_with(&self.config.token_prefix) {
                    let token = header_value
                        .strip_prefix(&self.config.token_prefix)
                        .ok_or_else(|| AuthError::token_error("Invalid token format"))?
                        .trim();
                    
                    if token.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(token.to_string()))
                    }
                } else {
                    Err(AuthError::token_error(&format!(
                        "Token must start with '{}'", 
                        self.config.token_prefix
                    )))
                }
            }
            None => Ok(None),
        }
    }
    
    /// Validate JWT token and extract claims
    pub fn validate_token(&self, token_str: &str) -> AuthResult<JwtClaims> {
        let jwt_token = JwtToken {
            token: token_str.to_string(),
            expires_at: chrono::Utc::now(), // This will be validated from the token itself
            refresh_token: None,
        };
        
        self.provider.validate_token_claims(&jwt_token)
    }
    
    /// Create user context from validated JWT claims
    pub fn create_user_context(&self, claims: &JwtClaims) -> UserContext {
        self.provider.claims_to_user_context(claims)
    }
    
    /// Check if path should skip authentication
    pub fn should_skip_path(&self, path: &str) -> bool {
        self.config.skip_paths.iter().any(|skip_path| {
            path.starts_with(skip_path)
        })
    }
    
    /// Process authentication for a request
    /// Returns Ok(Some(UserContext)) for authenticated user
    /// Returns Ok(None) for unauthenticated but allowed requests
    /// Returns Err for authentication failures
    pub fn authenticate(&self, path: &str, auth_header: Option<&str>) -> AuthResult<Option<UserContext>> {
        // Skip authentication for configured paths
        if self.should_skip_path(path) {
            return Ok(None);
        }
        
        // Extract token from header
        let token = self.extract_token(auth_header)?;
        
        match token {
            Some(token_str) => {
                // Validate token and create user context
                let claims = self.validate_token(&token_str)?;
                let user_context = self.create_user_context(&claims);
                Ok(Some(user_context))
            }
            None => {
                if self.config.optional {
                    Ok(None)
                } else {
                    Err(AuthError::authentication_failed("Missing authorization token"))
                }
            }
        }
    }
}

/// Builder for JWT middleware configuration
pub struct JwtMiddlewareBuilder<User> {
    provider: Option<JwtProvider<User>>,
    config: JwtMiddlewareConfig,
}

impl<User> JwtMiddlewareBuilder<User> {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            provider: None,
            config: JwtMiddlewareConfig::default(),
        }
    }
    
    /// Set the JWT provider
    pub fn provider(mut self, provider: JwtProvider<User>) -> Self {
        self.provider = Some(provider);
        self
    }
    
    /// Set the authorization header name
    pub fn header_name<S: Into<String>>(mut self, name: S) -> Self {
        self.config.header_name = name.into();
        self
    }
    
    /// Set the token prefix
    pub fn token_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.config.token_prefix = prefix.into();
        self
    }
    
    /// Add a path to skip authentication
    pub fn skip_path<S: Into<String>>(mut self, path: S) -> Self {
        self.config.skip_paths.push(path.into());
        self
    }
    
    /// Set multiple paths to skip authentication
    pub fn skip_paths(mut self, paths: Vec<String>) -> Self {
        self.config.skip_paths = paths;
        self
    }
    
    /// Make authentication optional
    pub fn optional(mut self) -> Self {
        self.config.optional = true;
        self
    }
    
    /// Make authentication required (default)
    pub fn required(mut self) -> Self {
        self.config.optional = false;
        self
    }
    
    /// Build the middleware
    pub fn build(self) -> AuthResult<JwtMiddleware<User>> {
        let provider = self.provider.ok_or_else(|| {
            AuthError::configuration_error("JWT provider is required")
        })?;
        
        Ok(JwtMiddleware::with_config(provider, self.config))
    }
}

impl<User> Default for JwtMiddlewareBuilder<User> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::JwtConfig,
        providers::jwt::{JwtUser, JwtProvider},
    };
    
    fn create_test_provider() -> JwtProvider<JwtUser> {
        let config = JwtConfig {
            secret: "test-secret-key-that-is-long-enough-for-validation".to_string(),
            algorithm: "HS256".to_string(),
            access_token_expiry: 900,
            refresh_token_expiry: 604800,
            issuer: "test".to_string(),
            audience: Some("test-app".to_string()),
            allow_refresh: true,
        };
        
        JwtProvider::new(config).unwrap()
    }
    
    fn create_test_user() -> JwtUser {
        JwtUser {
            id: "123".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read".to_string()],
            is_active: true,
            is_locked: false,
        }
    }
    
    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_jwt_middleware_creation() {
        let provider = create_test_provider();
        let middleware = JwtMiddleware::new(provider);
        
        assert_eq!(middleware.config.header_name, "Authorization");
        assert_eq!(middleware.config.token_prefix, "Bearer ");
        assert!(!middleware.config.optional);
    }
    
    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_token_extraction() {
        let provider = create_test_provider();
        let middleware = JwtMiddleware::new(provider);
        
        // Valid token
        let result = middleware.extract_token(Some("Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_string()));
        
        // No header
        let result = middleware.extract_token(None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
        
        // Empty token
        let result = middleware.extract_token(Some("Bearer "));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
        
        // Invalid prefix
        let result = middleware.extract_token(Some("Basic token"));
        assert!(result.is_err());
    }
    
    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_path_skipping() {
        let provider = create_test_provider();
        let middleware = JwtMiddleware::new(provider);
        
        assert!(middleware.should_skip_path("/health"));
        assert!(middleware.should_skip_path("/health/check"));
        assert!(middleware.should_skip_path("/metrics"));
        assert!(!middleware.should_skip_path("/api/users"));
        assert!(!middleware.should_skip_path("/"));
    }
    
    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_authentication_with_valid_token() {
        let provider = create_test_provider();
        let user = create_test_user();
        
        // Generate a valid token
        let token = provider.generate_token(&user, "access").unwrap();
        let auth_header = format!("Bearer {}", token.token);
        
        let middleware = JwtMiddleware::new(provider);
        let result = middleware.authenticate("/api/users", Some(&auth_header));
        
        assert!(result.is_ok());
        let context = result.unwrap();
        assert!(context.is_some());
        
        let context = context.unwrap();
        assert_eq!(context.user_id, "123");
        assert_eq!(context.username, "testuser");
        assert_eq!(context.auth_provider, "jwt");
    }
    
    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_authentication_skip_path() {
        let provider = create_test_provider();
        let middleware = JwtMiddleware::new(provider);
        
        let result = middleware.authenticate("/health", None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
    
    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_authentication_missing_token() {
        let provider = create_test_provider();
        let middleware = JwtMiddleware::new(provider);
        
        let result = middleware.authenticate("/api/users", None);
        assert!(result.is_err());
    }
    
    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_optional_authentication() {
        let provider = create_test_provider();
        let config = JwtMiddlewareConfig {
            optional: true,
            ..Default::default()
        };
        let middleware = JwtMiddleware::with_config(provider, config);
        
        let result = middleware.authenticate("/api/users", None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
    
    #[tokio::test]
    async fn test_middleware_builder() {
        let provider = create_test_provider();
        
        #[cfg(feature = "jwt")]
        {
            let middleware = JwtMiddlewareBuilder::new()
                .provider(provider)
                .header_name("X-Auth-Token")
                .token_prefix("Token ")
                .skip_path("/public")
                .optional()
                .build();
            
            assert!(middleware.is_ok());
            let middleware = middleware.unwrap();
            
            assert_eq!(middleware.config.header_name, "X-Auth-Token");
            assert_eq!(middleware.config.token_prefix, "Token ");
            assert!(middleware.config.optional);
            assert!(middleware.config.skip_paths.contains(&"/public".to_string()));
        }
    }
    
    #[tokio::test]
    async fn test_builder_missing_provider() {
        let middleware = JwtMiddlewareBuilder::<JwtUser>::new().build();
        assert!(middleware.is_err());
    }
}