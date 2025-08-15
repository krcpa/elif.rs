//! Core authentication and authorization traits

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::{AuthError, AuthResult};

/// Trait for types that can be authenticated
#[async_trait]
pub trait Authenticatable: Send + Sync + Clone {
    type Id: Clone + Send + Sync + std::fmt::Debug + PartialEq;
    type Credentials: Send + Sync;

    /// Get the user's unique identifier
    fn id(&self) -> &Self::Id;

    /// Get the user's username/email for authentication
    fn username(&self) -> &str;

    /// Check if the user account is active
    fn is_active(&self) -> bool {
        true
    }

    /// Check if the user account is locked
    fn is_locked(&self) -> bool {
        false
    }

    /// Get the user's role names
    fn roles(&self) -> Vec<String> {
        vec![]
    }

    /// Get the user's direct permissions
    fn permissions(&self) -> Vec<String> {
        vec![]
    }

    /// Verify credentials against this user
    async fn verify_credentials(&self, credentials: &Self::Credentials) -> AuthResult<bool>;

    /// Get additional user data for token/session storage
    fn additional_data(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }
}

/// Authentication provider trait for different auth mechanisms
#[async_trait]
pub trait AuthProvider<User>: Send + Sync
where
    User: Authenticatable,
{
    type Token: Clone + Send + Sync + std::fmt::Debug;
    type Credentials: Send + Sync;

    /// Authenticate user with credentials and return a token
    async fn authenticate(
        &self,
        credentials: &Self::Credentials,
    ) -> AuthResult<AuthenticationResult<User, Self::Token>>;

    /// Validate an existing token and return user information
    async fn validate_token(&self, token: &Self::Token) -> AuthResult<User>;

    /// Refresh a token if supported
    async fn refresh_token(&self, _token: &Self::Token) -> AuthResult<Self::Token> {
        Err(AuthError::token_error("Token refresh not supported"))
    }

    /// Revoke a token
    async fn revoke_token(&self, _token: &Self::Token) -> AuthResult<()> {
        Ok(()) // Default implementation does nothing
    }

    /// Get provider name for identification
    fn provider_name(&self) -> &str;
}

/// Result of authentication attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationResult<User, Token> {
    /// The authenticated user
    pub user: User,
    
    /// The authentication token
    pub token: Token,
    
    /// Optional refresh token
    pub refresh_token: Option<Token>,
    
    /// Whether MFA is required
    pub requires_mfa: bool,
    
    /// MFA setup information if required
    pub mfa_setup: Option<MfaSetup>,
    
    /// Token expiration time
    pub expires_at: Option<DateTime<Utc>>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// MFA setup information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSetup {
    /// TOTP secret key
    pub secret: String,
    
    /// QR code URL for easy setup
    pub qr_code_url: String,
    
    /// Backup codes
    pub backup_codes: Vec<String>,
}

/// Authorization provider trait for role-based access control
#[async_trait]
pub trait AuthorizationProvider: Send + Sync {
    type User: Authenticatable;
    type Role: Send + Sync + Clone;
    type Permission: Send + Sync + Clone;

    /// Check if user has a specific role
    async fn has_role(&self, user: &Self::User, role: &str) -> AuthResult<bool>;

    /// Check if user has a specific permission
    async fn has_permission(&self, user: &Self::User, permission: &str) -> AuthResult<bool>;

    /// Check if user has a specific permission with context
    async fn has_permission_with_context(
        &self, 
        user: &Self::User, 
        resource: &str,
        action: &str,
        context: Option<&HashMap<String, serde_json::Value>>
    ) -> AuthResult<bool>;

    /// Check if user has any of the specified roles
    async fn has_any_role(&self, user: &Self::User, roles: &[String]) -> AuthResult<bool> {
        for role in roles {
            if self.has_role(user, role).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if user has all of the specified roles
    async fn has_all_roles(&self, user: &Self::User, roles: &[String]) -> AuthResult<bool> {
        for role in roles {
            if !self.has_role(user, role).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Check if user has any of the specified permissions
    async fn has_any_permission(&self, user: &Self::User, permissions: &[String]) -> AuthResult<bool> {
        for permission in permissions {
            if self.has_permission(user, permission).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if user has all of the specified permissions
    async fn has_all_permissions(&self, user: &Self::User, permissions: &[String]) -> AuthResult<bool> {
        for permission in permissions {
            if !self.has_permission(user, permission).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Get all roles for a user
    async fn get_user_roles(&self, user: &Self::User) -> AuthResult<Vec<Self::Role>>;

    /// Get all permissions for a user (direct and through roles)
    async fn get_user_permissions(&self, user: &Self::User) -> AuthResult<Vec<Self::Permission>>;
}

/// Session storage trait for session-based authentication
#[async_trait]
pub trait SessionStorage: Send + Sync {
    type SessionId: Clone + Send + Sync + std::fmt::Debug + PartialEq;
    type SessionData: Clone + Send + Sync;

    /// Create a new session
    async fn create_session(
        &self,
        data: Self::SessionData,
        expires_at: DateTime<Utc>,
    ) -> AuthResult<Self::SessionId>;

    /// Get session data by ID
    async fn get_session(&self, id: &Self::SessionId) -> AuthResult<Option<Self::SessionData>>;

    /// Update session data
    async fn update_session(
        &self,
        id: &Self::SessionId,
        data: Self::SessionData,
        expires_at: DateTime<Utc>,
    ) -> AuthResult<()>;

    /// Delete a session
    async fn delete_session(&self, id: &Self::SessionId) -> AuthResult<()>;

    /// Clean up expired sessions
    async fn cleanup_expired_sessions(&self) -> AuthResult<u64>;

    /// Get session expiration time
    async fn get_session_expiry(&self, id: &Self::SessionId) -> AuthResult<Option<DateTime<Utc>>>;

    /// Extend session expiration
    async fn extend_session(&self, id: &Self::SessionId, expires_at: DateTime<Utc>) -> AuthResult<()>;
}

/// Multi-factor authentication provider
#[async_trait]
pub trait MfaProvider: Send + Sync {
    type User: Authenticatable;
    type Secret: Clone + Send + Sync;
    type Code: Send + Sync;

    /// Generate MFA setup information for a user
    async fn setup_mfa(&self, user: &Self::User) -> AuthResult<MfaSetup>;

    /// Verify MFA code
    async fn verify_code(
        &self,
        user: &Self::User,
        code: &Self::Code,
        secret: &Self::Secret,
    ) -> AuthResult<bool>;

    /// Generate backup codes
    async fn generate_backup_codes(&self, user: &Self::User) -> AuthResult<Vec<String>>;

    /// Verify backup code
    async fn verify_backup_code(
        &self,
        user: &Self::User,
        code: &str,
    ) -> AuthResult<bool>;

    /// Check if user has MFA enabled
    async fn is_mfa_enabled(&self, user: &Self::User) -> AuthResult<bool>;

    /// Disable MFA for a user
    async fn disable_mfa(&self, user: &Self::User) -> AuthResult<()>;
}

/// Password hasher trait for different hashing algorithms
pub trait PasswordHasher: Send + Sync {
    /// Hash a password
    fn hash_password(&self, password: &str) -> AuthResult<String>;

    /// Verify a password against its hash
    fn verify_password(&self, password: &str, hash: &str) -> AuthResult<bool>;

    /// Get the hasher name
    fn hasher_name(&self) -> &str;
}

/// User context extracted from authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User ID
    pub user_id: String,
    
    /// Username/email
    pub username: String,
    
    /// User roles
    pub roles: Vec<String>,
    
    /// User permissions
    pub permissions: Vec<String>,
    
    /// Authentication provider used
    pub auth_provider: String,
    
    /// Authentication timestamp
    pub authenticated_at: DateTime<Utc>,
    
    /// Token expiration (if applicable)
    pub expires_at: Option<DateTime<Utc>>,
    
    /// Additional user data
    pub additional_data: HashMap<String, serde_json::Value>,
}

impl UserContext {
    /// Create a new user context
    pub fn new(
        user_id: String,
        username: String,
        auth_provider: String,
    ) -> Self {
        Self {
            user_id,
            username,
            roles: vec![],
            permissions: vec![],
            auth_provider,
            authenticated_at: Utc::now(),
            expires_at: None,
            additional_data: HashMap::new(),
        }
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[String]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if user has all of the specified roles
    pub fn has_all_roles(&self, roles: &[String]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }

    /// Check if authentication has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| Utc::now() > exp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_context_creation() {
        let context = UserContext::new(
            "123".to_string(),
            "user@example.com".to_string(),
            "jwt".to_string(),
        );
        
        assert_eq!(context.user_id, "123");
        assert_eq!(context.username, "user@example.com");
        assert_eq!(context.auth_provider, "jwt");
        assert!(context.roles.is_empty());
        assert!(context.permissions.is_empty());
    }

    #[test]
    fn test_user_context_role_checking() {
        let mut context = UserContext::new(
            "123".to_string(),
            "user@example.com".to_string(),
            "jwt".to_string(),
        );
        
        context.roles = vec!["admin".to_string(), "editor".to_string()];
        
        assert!(context.has_role("admin"));
        assert!(context.has_role("editor"));
        assert!(!context.has_role("viewer"));
        
        assert!(context.has_any_role(&["admin".to_string(), "viewer".to_string()]));
        assert!(!context.has_any_role(&["viewer".to_string(), "guest".to_string()]));
        
        assert!(context.has_all_roles(&["admin".to_string(), "editor".to_string()]));
        assert!(!context.has_all_roles(&["admin".to_string(), "viewer".to_string()]));
    }

    #[test]
    fn test_user_context_permission_checking() {
        let mut context = UserContext::new(
            "123".to_string(),
            "user@example.com".to_string(),
            "jwt".to_string(),
        );
        
        context.permissions = vec!["read".to_string(), "write".to_string()];
        
        assert!(context.has_permission("read"));
        assert!(context.has_permission("write"));
        assert!(!context.has_permission("delete"));
    }

    #[test]
    fn test_user_context_expiration() {
        let mut context = UserContext::new(
            "123".to_string(),
            "user@example.com".to_string(),
            "jwt".to_string(),
        );
        
        // No expiration set
        assert!(!context.is_expired());
        
        // Set expiration in the past
        context.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(context.is_expired());
        
        // Set expiration in the future
        context.expires_at = Some(Utc::now() + chrono::Duration::hours(1));
        assert!(!context.is_expired());
    }
}