//! Authentication extension for HTTP requests
//! 
//! Provides utilities for accessing authenticated user context from HTTP requests

#[cfg(feature = "auth")]
pub use elif_auth::{UserContext, middleware::{RequireAuth, OptionalAuth, AuthGuard}};

/// Extension trait for accessing user context from requests
pub trait RequestAuthExt {
    /// Get the authenticated user context from the request
    /// 
    /// Returns None if the user is not authenticated or authentication is optional
    #[cfg(feature = "auth")]
    fn user(&self) -> Option<&UserContext>;
    
    /// Get the authenticated user context, returning an error if not present
    /// 
    /// This is useful for required authentication scenarios
    #[cfg(feature = "auth")]
    fn require_user(&self) -> Result<&UserContext, crate::HttpError>;
    
    /// Check if the request has an authenticated user
    #[cfg(feature = "auth")]
    fn is_authenticated(&self) -> bool;
    
    /// Get the user ID from the authenticated user context
    #[cfg(feature = "auth")]
    fn user_id(&self) -> Option<&str>;
    
    /// Get the username from the authenticated user context
    #[cfg(feature = "auth")]
    fn username(&self) -> Option<&str>;
    
    /// Check if the authenticated user has a specific role
    #[cfg(feature = "auth")]
    fn has_role(&self, role: &str) -> bool;
    
    /// Check if the authenticated user has a specific permission
    #[cfg(feature = "auth")]
    fn has_permission(&self, permission: &str) -> bool;
    
    /// Check if the authenticated user has any of the specified roles
    #[cfg(feature = "auth")]
    fn has_any_role(&self, roles: &[&str]) -> bool;
    
    /// Check if the authenticated user has all of the specified roles
    #[cfg(feature = "auth")]
    fn has_all_roles(&self, roles: &[&str]) -> bool;
    
    /// Check if the authenticated user has any of the specified permissions
    #[cfg(feature = "auth")]
    fn has_any_permission(&self, permissions: &[&str]) -> bool;
    
    /// Check if the authenticated user has all of the specified permissions
    #[cfg(feature = "auth")]
    fn has_all_permissions(&self, permissions: &[&str]) -> bool;
}

#[cfg(feature = "auth")]
impl RequestAuthExt for axum::extract::Request {
    fn user(&self) -> Option<&UserContext> {
        self.extensions().get::<UserContext>()
    }
    
    fn require_user(&self) -> Result<&UserContext, crate::HttpError> {
        self.user()
            .ok_or_else(|| crate::HttpError::unauthorized())
    }
    
    fn is_authenticated(&self) -> bool {
        self.user().is_some()
    }
    
    fn user_id(&self) -> Option<&str> {
        self.user().map(|user| user.user_id.as_str())
    }
    
    fn username(&self) -> Option<&str> {
        self.user().map(|user| user.username.as_str())
    }
    
    fn has_role(&self, role: &str) -> bool {
        self.user()
            .map(|user| user.roles.contains(&role.to_string()))
            .unwrap_or(false)
    }
    
    fn has_permission(&self, permission: &str) -> bool {
        self.user()
            .map(|user| user.permissions.contains(&permission.to_string()))
            .unwrap_or(false)
    }
    
    fn has_any_role(&self, roles: &[&str]) -> bool {
        self.user()
            .map(|user| {
                roles.iter().any(|role| user.roles.contains(&role.to_string()))
            })
            .unwrap_or(false)
    }
    
    fn has_all_roles(&self, roles: &[&str]) -> bool {
        self.user()
            .map(|user| {
                roles.iter().all(|role| user.roles.contains(&role.to_string()))
            })
            .unwrap_or(false)
    }
    
    fn has_any_permission(&self, permissions: &[&str]) -> bool {
        self.user()
            .map(|user| {
                permissions.iter().any(|perm| user.permissions.contains(&perm.to_string()))
            })
            .unwrap_or(false)
    }
    
    fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        self.user()
            .map(|user| {
                permissions.iter().all(|perm| user.permissions.contains(&perm.to_string()))
            })
            .unwrap_or(false)
    }
}

#[cfg(not(feature = "auth"))]
impl RequestAuthExt for axum::extract::Request {
    // Placeholder implementations when auth feature is disabled
    // These methods will not be available at runtime
}

/// Authentication middleware that integrates auth guards with the HTTP pipeline
#[cfg(feature = "auth")]
pub struct AuthMiddleware<G> {
    guard: G,
}

#[cfg(feature = "auth")]
impl<G> AuthMiddleware<G>
where
    G: AuthGuard + Send + Sync + 'static,
{
    /// Create new authentication middleware with the given guard
    pub fn new(guard: G) -> Self {
        Self { guard }
    }
    
    /// Create middleware that requires authentication
    pub fn require() -> AuthMiddleware<RequireAuth> {
        AuthMiddleware::new(RequireAuth::new())
    }
    
    /// Create middleware that allows optional authentication
    pub fn optional() -> AuthMiddleware<OptionalAuth> {
        AuthMiddleware::new(OptionalAuth::new())
    }
    
    /// Create middleware that requires specific role
    pub fn require_role<S: Into<String>>(role: S) -> AuthMiddleware<RequireAuth> {
        AuthMiddleware::new(RequireAuth::new().require_role(role))
    }
    
    /// Create middleware that requires specific permission
    pub fn require_permission<S: Into<String>>(permission: S) -> AuthMiddleware<RequireAuth> {
        AuthMiddleware::new(RequireAuth::new().require_permission(permission))
    }
}

#[cfg(feature = "auth")]
impl<G> crate::middleware::Middleware for AuthMiddleware<G>
where
    G: AuthGuard + Send + Sync + 'static,
{
    fn process_request<'a>(
        &'a self,
        mut request: axum::extract::Request,
    ) -> crate::middleware::BoxFuture<'a, Result<axum::extract::Request, axum::response::Response>> {
        Box::pin(async move {
            let path = request.uri().path();
            
            // Check if we should skip authentication for this path
            if self.guard.should_skip_path(path) {
                return Ok(request);
            }
            
            // Try to extract user context from existing middleware
            // This could come from JWT or Session middleware
            let user_context = request.extensions().get::<UserContext>().cloned();
            
            match user_context {
                Some(user) => {
                    // We have a user context, validate against guard requirements
                    if let Err(auth_error) = self.guard.validate_user(&user) {
                        // User doesn't meet requirements
                        let response = axum::response::Response::builder()
                            .status(axum::http::StatusCode::FORBIDDEN)
                            .header("content-type", "application/json")
                            .body(axum::body::Body::from(format!(
                                r#"{{"error": {{"code": "FORBIDDEN", "message": "{}"}}}}\"#,
                                auth_error
                            )))
                            .unwrap();
                        return Err(response);
                    }
                    // User context is valid, keep it in extensions
                    request.extensions_mut().insert(user);
                    Ok(request)
                }
                None => {
                    // No user context found
                    if self.guard.is_optional() {
                        // Optional authentication - allow request to proceed
                        Ok(request)
                    } else {
                        // Required authentication - reject request
                        let response = axum::response::Response::builder()
                            .status(axum::http::StatusCode::UNAUTHORIZED)
                            .header("content-type", "application/json")
                            .body(axum::body::Body::from(r#"{"error": {"code": "UNAUTHORIZED", "message": "Authentication required"}}"#))
                            .unwrap();
                        Err(response)
                    }
                }
            }
        })
    }
    
    fn name(&self) -> &'static str {
        "AuthMiddleware"
    }
}

#[cfg(test)]
#[cfg(feature = "auth")]
mod tests {
    use super::*;
    use crate::middleware::Middleware;
    use axum::http::Method;
    use std::collections::HashMap;
    use chrono::Utc;

    fn create_test_request(path: &str) -> axum::extract::Request {
        axum::extract::Request::builder()
            .method(Method::GET)
            .uri(path)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    fn create_test_user() -> UserContext {
        UserContext {
            user_id: "123".to_string(),
            username: "test@example.com".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read".to_string()],
            auth_provider: "test".to_string(),
            authenticated_at: Utc::now(),
            expires_at: None,
            additional_data: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_auth_middleware_require_with_valid_user() {
        let middleware = AuthMiddleware::new(RequireAuth::new());
        let mut request = create_test_request("/protected");
        
        // Add user context to request extensions
        let user = create_test_user();
        request.extensions_mut().insert(user.clone());
        
        let result = middleware.process_request(request).await;
        assert!(result.is_ok());
        
        let processed_request = result.unwrap();
        assert!(processed_request.extensions().get::<UserContext>().is_some());
    }

    #[tokio::test]
    async fn test_auth_middleware_require_without_user() {
        let middleware = AuthMiddleware::new(RequireAuth::new());
        let request = create_test_request("/protected");
        
        let result = middleware.process_request(request).await;
        assert!(result.is_err());
        
        let response = result.unwrap_err();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_optional_without_user() {
        let middleware = AuthMiddleware::new(OptionalAuth::new());
        let request = create_test_request("/public");
        
        let result = middleware.process_request(request).await;
        assert!(result.is_ok());
        
        let processed_request = result.unwrap();
        assert!(processed_request.extensions().get::<UserContext>().is_none());
    }

    #[tokio::test]
    async fn test_auth_middleware_skip_paths() {
        let middleware = AuthMiddleware::new(RequireAuth::new());
        let request = create_test_request("/health");
        
        let result = middleware.process_request(request).await;
        assert!(result.is_ok()); // Should skip authentication for /health
    }

    #[tokio::test]
    async fn test_auth_middleware_require_role() {
        let middleware = AuthMiddleware::new(RequireAuth::new().require_role("admin"));
        let mut request = create_test_request("/admin");
        
        // User without admin role
        let user = create_test_user();
        request.extensions_mut().insert(user);
        
        let result = middleware.process_request(request).await;
        assert!(result.is_err());
        
        let response = result.unwrap_err();
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_auth_middleware_require_role_success() {
        let middleware = AuthMiddleware::new(RequireAuth::new().require_role("user"));
        let mut request = create_test_request("/user-area");
        
        // User with user role
        let user = create_test_user();
        request.extensions_mut().insert(user);
        
        let result = middleware.process_request(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_auth_ext_user_context() {
        let mut request = create_test_request("/test");
        let user = create_test_user();
        request.extensions_mut().insert(user.clone());
        
        assert!(request.is_authenticated());
        assert_eq!(request.user_id(), Some("123"));
        assert_eq!(request.username(), Some("test@example.com"));
        assert!(request.has_role("user"));
        assert!(!request.has_role("admin"));
        assert!(request.has_permission("read"));
        assert!(!request.has_permission("write"));
    }

    #[tokio::test]
    async fn test_request_auth_ext_no_user() {
        let request = create_test_request("/test");
        
        assert!(!request.is_authenticated());
        assert_eq!(request.user_id(), None);
        assert_eq!(request.username(), None);
        assert!(!request.has_role("user"));
        assert!(!request.has_permission("read"));
        assert!(request.require_user().is_err());
    }

    #[tokio::test]
    async fn test_request_auth_ext_role_checks() {
        let mut request = create_test_request("/test");
        let mut user = create_test_user();
        user.roles = vec!["user".to_string(), "moderator".to_string()];
        request.extensions_mut().insert(user);
        
        // Single role checks
        assert!(request.has_role("user"));
        assert!(request.has_role("moderator"));
        assert!(!request.has_role("admin"));
        
        // Multiple role checks
        assert!(request.has_any_role(&["user", "admin"]));
        assert!(request.has_any_role(&["moderator", "admin"]));
        assert!(!request.has_any_role(&["admin", "super_admin"]));
        
        assert!(request.has_all_roles(&["user", "moderator"]));
        assert!(!request.has_all_roles(&["user", "admin"]));
    }

    #[tokio::test]
    async fn test_request_auth_ext_permission_checks() {
        let mut request = create_test_request("/test");
        let mut user = create_test_user();
        user.permissions = vec!["read".to_string(), "write".to_string()];
        request.extensions_mut().insert(user);
        
        // Single permission checks
        assert!(request.has_permission("read"));
        assert!(request.has_permission("write"));
        assert!(!request.has_permission("delete"));
        
        // Multiple permission checks
        assert!(request.has_any_permission(&["read", "delete"]));
        assert!(request.has_any_permission(&["write", "delete"]));
        assert!(!request.has_any_permission(&["delete", "admin"]));
        
        assert!(request.has_all_permissions(&["read", "write"]));
        assert!(!request.has_all_permissions(&["read", "delete"]));
    }
}