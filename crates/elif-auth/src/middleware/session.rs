//! Session authentication middleware
//! 
//! This module provides session-based authentication middleware similar to JWT middleware.

use crate::{
    providers::session::{SessionId, SessionProvider, SessionData},
    traits::{Authenticatable, SessionStorage, UserContext},
    AuthError, AuthResult,
};
use service_builder::builder;

/// Session middleware configuration
#[derive(Debug, Clone)]
pub struct SessionMiddlewareConfig {
    /// Cookie name for session ID
    pub cookie_name: String,
    
    /// Session cookie domain
    pub cookie_domain: Option<String>,
    
    /// Session cookie path
    pub cookie_path: String,
    
    /// Whether session cookie is HTTP-only
    pub cookie_http_only: bool,
    
    /// Whether session cookie is secure (HTTPS only)
    pub cookie_secure: bool,
    
    /// Session cookie SameSite attribute
    pub cookie_same_site: CookieSameSite,
    
    /// Require CSRF protection for session
    pub require_csrf: bool,
    
    /// Paths to skip authentication
    pub skip_paths: Vec<String>,
    
    /// Whether authentication is optional (doesn't fail if no session)
    pub optional: bool,
}

/// Cookie SameSite attribute values
#[derive(Debug, Clone)]
pub enum CookieSameSite {
    Strict,
    Lax,
    None,
}

impl std::fmt::Display for CookieSameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookieSameSite::Strict => write!(f, "Strict"),
            CookieSameSite::Lax => write!(f, "Lax"),
            CookieSameSite::None => write!(f, "None"),
        }
    }
}

impl Default for SessionMiddlewareConfig {
    fn default() -> Self {
        Self {
            cookie_name: "session_id".to_string(),
            cookie_domain: None,
            cookie_path: "/".to_string(),
            cookie_http_only: true,
            cookie_secure: false, // Set to true in production
            cookie_same_site: CookieSameSite::Lax,
            require_csrf: true,
            skip_paths: vec!["/health".to_string(), "/metrics".to_string()],
            optional: false,
        }
    }
}

impl SessionMiddlewareConfig {
    /// Create a new configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set cookie name
    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }
    
    /// Set cookie domain
    pub fn cookie_domain(mut self, domain: impl Into<String>) -> Self {
        self.cookie_domain = Some(domain.into());
        self
    }
    
    /// Set cookie path
    pub fn cookie_path(mut self, path: impl Into<String>) -> Self {
        self.cookie_path = path.into();
        self
    }
    
    /// Set whether cookie is HTTP-only
    pub fn cookie_http_only(mut self, http_only: bool) -> Self {
        self.cookie_http_only = http_only;
        self
    }
    
    /// Set whether cookie is secure
    pub fn cookie_secure(mut self, secure: bool) -> Self {
        self.cookie_secure = secure;
        self
    }
    
    /// Set cookie SameSite attribute
    pub fn cookie_same_site(mut self, same_site: CookieSameSite) -> Self {
        self.cookie_same_site = same_site;
        self
    }
    
    /// Set whether CSRF protection is required
    pub fn require_csrf(mut self, require: bool) -> Self {
        self.require_csrf = require;
        self
    }
    
    /// Add paths to skip authentication
    pub fn skip_paths(mut self, paths: Vec<String>) -> Self {
        self.skip_paths = paths;
        self
    }
    
    /// Add a path to skip authentication
    pub fn skip_path(mut self, path: impl Into<String>) -> Self {
        self.skip_paths.push(path.into());
        self
    }
    
    /// Set whether authentication is optional
    pub fn optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
    
    /// Production-ready configuration
    pub fn production() -> Self {
        Self {
            cookie_secure: true,
            cookie_same_site: CookieSameSite::Strict,
            require_csrf: true,
            ..Default::default()
        }
    }
    
    /// Development configuration  
    pub fn development() -> Self {
        Self {
            cookie_secure: false,
            require_csrf: false, // Relaxed for development
            ..Default::default()
        }
    }
}

/// Session authentication middleware (placeholder for Phase 5.4)
/// 
/// This will be implemented in Phase 5.4: User Authentication Middleware
pub struct SessionMiddleware<S, U>
where
    S: SessionStorage<SessionId = SessionId, SessionData = SessionData>,
    U: Authenticatable,
{
    /// Session provider for session operations
    provider: SessionProvider<S, U>,
    
    /// Session extraction configuration
    config: SessionMiddlewareConfig,
}

impl<S, U> SessionMiddleware<S, U>
where
    S: SessionStorage<SessionId = SessionId, SessionData = SessionData>,
    U: Authenticatable + Clone,
{
    /// Create new session middleware
    pub fn new(
        provider: SessionProvider<S, U>,
        config: SessionMiddlewareConfig,
    ) -> Self {
        Self {
            provider,
            config,
        }
    }
    
    /// Create session middleware with default config
    pub fn with_default_config(provider: SessionProvider<S, U>) -> Self {
        Self::new(provider, SessionMiddlewareConfig::default())
    }
    
    /// Get middleware name
    pub fn name(&self) -> &str {
        "session"
    }
    
    /// Extract session ID from cookie header string
    pub fn extract_session_id_from_cookie(&self, cookie_header: &str) -> Option<SessionId> {
        for cookie in cookie_header.split(';') {
            let cookie = cookie.trim();
            if let Some(value) = cookie.strip_prefix(&format!("{}=", self.config.cookie_name)) {
                if let Ok(session_id) = SessionId::from_string(value.to_string()) {
                    return Some(session_id);
                }
            }
        }
        None
    }
    
    /// Check if path should be skipped
    pub fn should_skip_path(&self, path: &str) -> bool {
        self.config.skip_paths.iter().any(|skip_path| {
            path.starts_with(skip_path)
        })
    }
    
    /// Create session cookie string
    pub fn create_cookie_header(&self, session_id: &SessionId, max_age: Option<i64>) -> String {
        let mut cookie = format!("{}={}", self.config.cookie_name, session_id);
        
        if let Some(domain) = &self.config.cookie_domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }
        
        cookie.push_str(&format!("; Path={}", self.config.cookie_path));
        
        if self.config.cookie_http_only {
            cookie.push_str("; HttpOnly");
        }
        
        if self.config.cookie_secure {
            cookie.push_str("; Secure");
        }
        
        cookie.push_str(&format!("; SameSite={}", self.config.cookie_same_site));
        
        if let Some(max_age) = max_age {
            cookie.push_str(&format!("; Max-Age={}", max_age));
        }
        
        cookie
    }
    
    /// Validate session and get session data
    pub async fn validate_session(&self, session_id: &SessionId) -> AuthResult<SessionData> {
        self.provider.validate_session(session_id).await
    }
    
    /// Create user context from session data
    pub fn create_user_context(&self, session_data: &SessionData) -> UserContext {
        UserContext {
            user_id: session_data.user_id.clone(),
            username: session_data.username.clone(),
            roles: session_data.roles.clone(),
            permissions: session_data.permissions.clone(),
            auth_provider: "session".to_string(),
            authenticated_at: session_data.created_at,
            expires_at: Some(session_data.expires_at),
            additional_data: session_data.metadata.clone(),
        }
    }
    
    /// Get the session provider
    pub fn provider(&self) -> &SessionProvider<S, U> {
        &self.provider
    }
    
    /// Get the configuration
    pub fn config(&self) -> &SessionMiddlewareConfig {
        &self.config
    }
}

/// Configuration for Session middleware builder
#[derive(Debug, Clone)]
#[builder]
pub struct SessionMiddlewareBuilderConfig {
    #[builder(default = "String::from(\"session_id\")")]
    pub cookie_name: String,
    
    #[builder(optional)]
    pub cookie_domain: Option<String>,
    
    #[builder(default = "String::from(\"/\")")]
    pub cookie_path: String,
    
    #[builder(default = "true")]
    pub cookie_http_only: bool,
    
    #[builder(default)]
    pub cookie_secure: bool,
    
    #[builder(default = "CookieSameSite::Lax")]
    pub cookie_same_site: CookieSameSite,
    
    #[builder(default = "true")]
    pub require_csrf: bool,
    
    #[builder(default = "vec![String::from(\"/health\"), String::from(\"/metrics\")]")]
    pub skip_paths: Vec<String>,
    
    #[builder(default)]
    pub optional: bool,
}

impl SessionMiddlewareBuilderConfig {
    /// Build a SessionMiddlewareConfig from the builder config
    pub fn build_config(self) -> SessionMiddlewareConfig {
        SessionMiddlewareConfig {
            cookie_name: self.cookie_name,
            cookie_domain: self.cookie_domain,
            cookie_path: self.cookie_path,
            cookie_http_only: self.cookie_http_only,
            cookie_secure: self.cookie_secure,
            cookie_same_site: self.cookie_same_site,
            require_csrf: self.require_csrf,
            skip_paths: self.skip_paths,
            optional: self.optional,
        }
    }
}

// Add convenience methods to the generated builder
impl SessionMiddlewareBuilderConfigBuilder {
    /// Set cookie name
    pub fn cookie_name_str(self, name: impl Into<String>) -> Self {
        self.cookie_name(name.into())
    }
    
    /// Set cookie domain
    pub fn cookie_domain_str(self, domain: impl Into<String>) -> Self {
        self.cookie_domain(Some(domain.into()))
    }
    
    /// Set cookie path
    pub fn cookie_path_str(self, path: impl Into<String>) -> Self {
        self.cookie_path(path.into())
    }
    
    /// Make authentication optional
    pub fn make_optional(self) -> Self {
        self.optional(true)
    }
    
    /// Skip authentication for specific paths
    pub fn skip_paths_vec(self, paths: Vec<String>) -> Self {
        self.skip_paths(paths)
    }
    
    /// Add a path to skip
    pub fn skip_path<S: Into<String>>(self, path: S) -> Self {
        let mut paths = self.skip_paths.clone().unwrap_or_else(|| vec![String::from("/health"), String::from("/metrics")]);
        paths.push(path.into());
        self.skip_paths(paths)
    }
    
    /// Use production configuration
    pub fn production_config(self) -> Self {
        self.cookie_secure(true)
            .cookie_same_site(CookieSameSite::Strict)
            .require_csrf(true)
    }
    
    /// Use development configuration
    pub fn development_config(self) -> Self {
        self.cookie_secure(false)
            .require_csrf(false)
    }
    
    pub fn build_config(self) -> SessionMiddlewareBuilderConfig {
        self.build_with_defaults().unwrap()
    }
}

/// Builder for session middleware
pub struct SessionMiddlewareBuilder<S, U>
where
    S: SessionStorage<SessionId = SessionId, SessionData = SessionData>,
    U: Authenticatable,
{
    provider: Option<SessionProvider<S, U>>,
    builder_config: SessionMiddlewareBuilderConfigBuilder,
}

impl<S, U> SessionMiddlewareBuilder<S, U>
where
    S: SessionStorage<SessionId = SessionId, SessionData = SessionData>,
    U: Authenticatable,
{
    /// Create new builder
    pub fn new() -> Self {
        Self {
            provider: None,
            builder_config: SessionMiddlewareBuilderConfig::builder(),
        }
    }
    
    /// Set session provider
    pub fn provider(mut self, provider: SessionProvider<S, U>) -> Self {
        self.provider = Some(provider);
        self
    }
    
    /// Set cookie name
    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.builder_config = self.builder_config.cookie_name_str(name);
        self
    }
    
    /// Make authentication optional
    pub fn optional(mut self) -> Self {
        self.builder_config = self.builder_config.make_optional();
        self
    }
    
    /// Skip authentication for specific paths
    pub fn skip_paths(mut self, paths: Vec<String>) -> Self {
        self.builder_config = self.builder_config.skip_paths_vec(paths);
        self
    }
    
    /// Add a path to skip
    pub fn skip_path(mut self, path: impl Into<String>) -> Self {
        self.builder_config = self.builder_config.skip_path(path);
        self
    }
    
    /// Use production configuration
    pub fn production(mut self) -> Self {
        self.builder_config = self.builder_config.production_config();
        self
    }
    
    /// Use development configuration
    pub fn development(mut self) -> Self {
        self.builder_config = self.builder_config.development_config();
        self
    }
    
    /// Build the middleware
    pub fn build(self) -> AuthResult<SessionMiddleware<S, U>>
    where
        U: Clone,
    {
        let provider = self.provider
            .ok_or_else(|| AuthError::generic_error("Session provider is required"))?;
        
        let config = self.builder_config.build_config().build_config();
        Ok(SessionMiddleware::new(provider, config))
    }
}

impl<S, U> Default for SessionMiddlewareBuilder<S, U>
where
    S: SessionStorage<SessionId = SessionId, SessionData = SessionData>,
    U: Authenticatable,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::session::MemorySessionStorage;
    
    // Mock user for testing
    #[derive(Debug, Clone)]
    struct MockUser {
        id: String,
        username: String,
        roles: Vec<String>,
        permissions: Vec<String>,
    }
    
    #[async_trait::async_trait]
    impl Authenticatable for MockUser {
        type Id = String;
        type Credentials = String;
        
        fn id(&self) -> &Self::Id {
            &self.id
        }
        
        fn username(&self) -> &str {
            &self.username
        }
        
        fn roles(&self) -> Vec<String> {
            self.roles.clone()
        }
        
        fn permissions(&self) -> Vec<String> {
            self.permissions.clone()
        }
        
        async fn verify_credentials(&self, _credentials: &Self::Credentials) -> AuthResult<bool> {
            Ok(true)
        }
    }
    
    #[test]
    fn test_session_middleware_config() {
        let config = SessionMiddlewareConfig::new()
            .cookie_name("test_session")
            .cookie_domain("example.com")
            .cookie_secure(true)
            .require_csrf(false)
            .optional(true);
        
        assert_eq!(config.cookie_name, "test_session");
        assert_eq!(config.cookie_domain, Some("example.com".to_string()));
        assert!(config.cookie_secure);
        assert!(!config.require_csrf);
        assert!(config.optional);
    }
    
    #[test]
    fn test_cookie_same_site_display() {
        assert_eq!(CookieSameSite::Strict.to_string(), "Strict");
        assert_eq!(CookieSameSite::Lax.to_string(), "Lax");
        assert_eq!(CookieSameSite::None.to_string(), "None");
    }
    
    #[test]
    fn test_session_middleware_builder() {
        let storage = MemorySessionStorage::new();
        let provider: SessionProvider<MemorySessionStorage, MockUser> = SessionProvider::with_default_config(storage);
        
        let builder = SessionMiddlewareBuilder::new()
            .provider(provider)
            .cookie_name("test_session")
            .optional()
            .skip_path("/health");
        
        let middleware = builder.build().unwrap();
        assert_eq!(middleware.name(), "session");
    }
    
    #[tokio::test]
    async fn test_cookie_header_creation() {
        let storage = MemorySessionStorage::new();
        let provider: SessionProvider<MemorySessionStorage, MockUser> = SessionProvider::with_default_config(storage);
        let config = SessionMiddlewareConfig::production();
        let middleware = SessionMiddleware::new(provider, config);
        
        let session_id = SessionId::generate();
        let cookie = middleware.create_cookie_header(&session_id, Some(3600));
        
        assert!(cookie.contains(&format!("session_id={}", session_id)));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Max-Age=3600"));
    }
    
    #[test]
    fn test_session_id_extraction() {
        let storage = MemorySessionStorage::new();
        let provider: SessionProvider<MemorySessionStorage, MockUser> = SessionProvider::with_default_config(storage);
        let middleware = SessionMiddleware::with_default_config(provider);
        
        let cookie_header = "other_cookie=value; session_id=short; another=value";
        let extracted = middleware.extract_session_id_from_cookie(cookie_header);
        
        // This will fail since "short" is too short for validation
        assert!(extracted.is_none()); // Because it fails validation
        
        // Test with a valid length session ID
        let valid_session_id = "a".repeat(32);
        let cookie_header = format!("session_id={}", valid_session_id);
        let extracted = middleware.extract_session_id_from_cookie(&cookie_header);
        assert!(extracted.is_some());
        
        // Test extraction from complex cookie header
        let complex_cookie = format!("first=value1; session_id={}; last=value2", valid_session_id);
        let extracted = middleware.extract_session_id_from_cookie(&complex_cookie);
        assert!(extracted.is_some());
    }
    
    #[test]
    fn test_path_skipping() {
        let storage = MemorySessionStorage::new();
        let provider: SessionProvider<MemorySessionStorage, MockUser> = SessionProvider::with_default_config(storage);
        let middleware = SessionMiddleware::with_default_config(provider);
        
        assert!(middleware.should_skip_path("/health"));
        assert!(middleware.should_skip_path("/metrics"));
        assert!(!middleware.should_skip_path("/api/users"));
    }
}