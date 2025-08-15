//! Authentication guards for HTTP requests
//! 
//! Provides authentication middleware that can work with any auth provider

use std::collections::HashSet;

use crate::{UserContext, AuthError, AuthResult};

/// Authentication guard configuration
#[derive(Debug, Clone)]
pub struct AuthGuardConfig {
    /// Paths that skip authentication
    pub skip_paths: HashSet<String>,
    
    /// Whether authentication is optional
    pub optional: bool,
    
    /// Required roles (any of these roles grants access)
    pub required_roles: Vec<String>,
    
    /// Required permissions (any of these permissions grants access)
    pub required_permissions: Vec<String>,
    
    /// Whether to require all roles (true) or any role (false)
    pub require_all_roles: bool,
    
    /// Whether to require all permissions (true) or any permission (false)
    pub require_all_permissions: bool,
}

impl Default for AuthGuardConfig {
    fn default() -> Self {
        Self {
            skip_paths: ["/health", "/metrics"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            optional: false,
            required_roles: Vec::new(),
            required_permissions: Vec::new(),
            require_all_roles: false,
            require_all_permissions: false,
        }
    }
}

/// Base authentication guard trait
pub trait AuthGuard {
    /// Check if the path should skip authentication
    fn should_skip_path(&self, path: &str) -> bool;
    
    /// Check if authentication is optional
    fn is_optional(&self) -> bool;
    
    /// Validate user context against guard requirements
    fn validate_user(&self, user: &UserContext) -> AuthResult<()>;
    
    /// Get guard configuration
    fn config(&self) -> &AuthGuardConfig;
}

/// Required authentication guard - authentication must succeed
#[derive(Debug, Clone)]
pub struct RequireAuth {
    config: AuthGuardConfig,
}

impl RequireAuth {
    /// Create a new required authentication guard
    pub fn new() -> Self {
        Self {
            config: AuthGuardConfig::default(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: AuthGuardConfig) -> Self {
        Self { config }
    }
    
    /// Add a path to skip authentication
    pub fn skip_path<S: Into<String>>(mut self, path: S) -> Self {
        self.config.skip_paths.insert(path.into());
        self
    }
    
    /// Add multiple paths to skip authentication
    pub fn skip_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for path in paths {
            self.config.skip_paths.insert(path.into());
        }
        self
    }
    
    /// Require specific role
    pub fn require_role<S: Into<String>>(mut self, role: S) -> Self {
        self.config.required_roles.push(role.into());
        self
    }
    
    /// Require specific roles
    pub fn require_roles<I, S>(mut self, roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.required_roles.extend(roles.into_iter().map(|r| r.into()));
        self
    }
    
    /// Require specific permission
    pub fn require_permission<S: Into<String>>(mut self, permission: S) -> Self {
        self.config.required_permissions.push(permission.into());
        self
    }
    
    /// Require specific permissions
    pub fn require_permissions<I, S>(mut self, permissions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.required_permissions.extend(permissions.into_iter().map(|p| p.into()));
        self
    }
    
    /// Require ALL specified roles instead of ANY
    pub fn require_all_roles(mut self) -> Self {
        self.config.require_all_roles = true;
        self
    }
    
    /// Require ALL specified permissions instead of ANY
    pub fn require_all_permissions(mut self) -> Self {
        self.config.require_all_permissions = true;
        self
    }
}

impl Default for RequireAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthGuard for RequireAuth {
    fn should_skip_path(&self, path: &str) -> bool {
        self.config.skip_paths.contains(path)
    }
    
    fn is_optional(&self) -> bool {
        false // Required authentication is never optional
    }
    
    fn validate_user(&self, user: &UserContext) -> AuthResult<()> {
        // Check role requirements
        if !self.config.required_roles.is_empty() {
            let has_required_roles = if self.config.require_all_roles {
                // Check if user has ALL required roles
                self.config.required_roles.iter().all(|role| user.roles.contains(role))
            } else {
                // Check if user has ANY required role
                self.config.required_roles.iter().any(|role| user.roles.contains(role))
            };
            
            if !has_required_roles {
                return Err(AuthError::insufficient_permissions(&format!(
                    "User lacks required roles: {:?}", 
                    self.config.required_roles
                )));
            }
        }
        
        // Check permission requirements
        if !self.config.required_permissions.is_empty() {
            let has_required_permissions = if self.config.require_all_permissions {
                // Check if user has ALL required permissions
                self.config.required_permissions.iter().all(|perm| user.permissions.contains(perm))
            } else {
                // Check if user has ANY required permission
                self.config.required_permissions.iter().any(|perm| user.permissions.contains(perm))
            };
            
            if !has_required_permissions {
                return Err(AuthError::insufficient_permissions(&format!(
                    "User lacks required permissions: {:?}", 
                    self.config.required_permissions
                )));
            }
        }
        
        Ok(())
    }
    
    fn config(&self) -> &AuthGuardConfig {
        &self.config
    }
}

/// Optional authentication guard - authentication failure is allowed
#[derive(Debug, Clone)]
pub struct OptionalAuth {
    config: AuthGuardConfig,
}

impl OptionalAuth {
    /// Create a new optional authentication guard
    pub fn new() -> Self {
        let mut config = AuthGuardConfig::default();
        config.optional = true;
        
        Self { config }
    }
    
    /// Create with custom configuration
    pub fn with_config(mut config: AuthGuardConfig) -> Self {
        config.optional = true;
        Self { config }
    }
    
    /// Add a path to skip authentication
    pub fn skip_path<S: Into<String>>(mut self, path: S) -> Self {
        self.config.skip_paths.insert(path.into());
        self
    }
    
    /// Add multiple paths to skip authentication
    pub fn skip_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for path in paths {
            self.config.skip_paths.insert(path.into());
        }
        self
    }
}

impl Default for OptionalAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthGuard for OptionalAuth {
    fn should_skip_path(&self, path: &str) -> bool {
        self.config.skip_paths.contains(path)
    }
    
    fn is_optional(&self) -> bool {
        true // Optional authentication allows failures
    }
    
    fn validate_user(&self, _user: &UserContext) -> AuthResult<()> {
        // Optional auth doesn't validate - any authenticated user is allowed
        Ok(())
    }
    
    fn config(&self) -> &AuthGuardConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use chrono::Utc;

    fn create_test_user() -> UserContext {
        UserContext {
            user_id: "123".to_string(),
            username: "test@example.com".to_string(),
            roles: vec!["user".to_string(), "moderator".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
            auth_provider: "test".to_string(),
            authenticated_at: Utc::now(),
            expires_at: None,
            additional_data: HashMap::new(),
        }
    }

    #[test]
    fn test_require_auth_creation() {
        let guard = RequireAuth::new();
        assert!(!guard.is_optional());
        assert!(guard.should_skip_path("/health"));
        assert!(guard.should_skip_path("/metrics"));
        assert!(!guard.should_skip_path("/protected"));
    }

    #[test]
    fn test_require_auth_custom_skip_paths() {
        let guard = RequireAuth::new()
            .skip_path("/public")
            .skip_paths(["/docs", "/swagger"]);
        
        assert!(guard.should_skip_path("/public"));
        assert!(guard.should_skip_path("/docs"));
        assert!(guard.should_skip_path("/swagger"));
        assert!(!guard.should_skip_path("/private"));
    }

    #[test]
    fn test_require_auth_role_validation_any() {
        let user = create_test_user();
        
        // Should pass - user has 'user' role
        let guard = RequireAuth::new().require_role("user");
        assert!(guard.validate_user(&user).is_ok());
        
        // Should pass - user has 'moderator' role
        let guard = RequireAuth::new().require_role("moderator");
        assert!(guard.validate_user(&user).is_ok());
        
        // Should fail - user doesn't have 'admin' role
        let guard = RequireAuth::new().require_role("admin");
        assert!(guard.validate_user(&user).is_err());
        
        // Should pass - user has one of the required roles
        let guard = RequireAuth::new().require_roles(["admin", "moderator"]);
        assert!(guard.validate_user(&user).is_ok());
    }

    #[test]
    fn test_require_auth_role_validation_all() {
        let user = create_test_user();
        
        // Should pass - user has both required roles
        let guard = RequireAuth::new()
            .require_roles(["user", "moderator"])
            .require_all_roles();
        assert!(guard.validate_user(&user).is_ok());
        
        // Should fail - user doesn't have 'admin' role
        let guard = RequireAuth::new()
            .require_roles(["user", "admin"])
            .require_all_roles();
        assert!(guard.validate_user(&user).is_err());
    }

    #[test]
    fn test_require_auth_permission_validation_any() {
        let user = create_test_user();
        
        // Should pass - user has 'read' permission
        let guard = RequireAuth::new().require_permission("read");
        assert!(guard.validate_user(&user).is_ok());
        
        // Should fail - user doesn't have 'delete' permission
        let guard = RequireAuth::new().require_permission("delete");
        assert!(guard.validate_user(&user).is_err());
        
        // Should pass - user has one of the required permissions
        let guard = RequireAuth::new().require_permissions(["delete", "write"]);
        assert!(guard.validate_user(&user).is_ok());
    }

    #[test]
    fn test_require_auth_permission_validation_all() {
        let user = create_test_user();
        
        // Should pass - user has both required permissions
        let guard = RequireAuth::new()
            .require_permissions(["read", "write"])
            .require_all_permissions();
        assert!(guard.validate_user(&user).is_ok());
        
        // Should fail - user doesn't have 'delete' permission
        let guard = RequireAuth::new()
            .require_permissions(["read", "delete"])
            .require_all_permissions();
        assert!(guard.validate_user(&user).is_err());
    }

    #[test]
    fn test_optional_auth_creation() {
        let guard = OptionalAuth::new();
        assert!(guard.is_optional());
        assert!(guard.should_skip_path("/health"));
        assert!(guard.should_skip_path("/metrics"));
    }

    #[test]
    fn test_optional_auth_validation() {
        let user = create_test_user();
        let guard = OptionalAuth::new();
        
        // Optional auth always passes validation
        assert!(guard.validate_user(&user).is_ok());
    }

    #[test]
    fn test_optional_auth_custom_skip_paths() {
        let guard = OptionalAuth::new()
            .skip_path("/api")
            .skip_paths(["/v1", "/v2"]);
        
        assert!(guard.should_skip_path("/api"));
        assert!(guard.should_skip_path("/v1"));
        assert!(guard.should_skip_path("/v2"));
        assert!(!guard.should_skip_path("/protected"));
    }
}