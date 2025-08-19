//! Authentication testing utilities
//!
//! Provides utilities for testing authentication and authorization
//! in elif.rs applications, including JWT token generation,
//! session management, and RBAC testing helpers.

use std::collections::HashMap;
use serde_json::Value as JsonValue;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use crate::{TestError, TestResult, factories::{User, UserFactory, Factory}};

/// Test authentication provider for generating test tokens and sessions
#[allow(dead_code)]
pub struct TestAuthProvider {
    jwt_secret: String,
    session_store: HashMap<String, TestSession>,
}

impl TestAuthProvider {
    /// Create a new test auth provider
    pub fn new() -> Self {
        Self {
            jwt_secret: "test_jwt_secret_key_for_testing_only".to_string(),
            session_store: HashMap::new(),
        }
    }

    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
    
    /// Create with custom JWT secret
    pub fn with_jwt_secret(secret: impl Into<String>) -> Self {
        Self {
            jwt_secret: secret.into(),
            session_store: HashMap::new(),
        }
    }
    
    /// Generate a test JWT token for a user
    pub fn generate_jwt_token(&self, user: &User) -> TestResult<String> {
        let claims = TestJwtClaims {
            sub: user.id.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
            roles: vec!["user".to_string()], // Default role
            permissions: vec![],
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };
        
        // In a real implementation, this would use a JWT library
        // For testing purposes, we'll create a mock token
        let token = format!(
            "test_jwt_token_{}_{}", 
            user.id.to_string().replace('-', ""), 
            claims.exp
        );
        
        Ok(token)
    }
    
    /// Generate a test JWT token with custom claims
    pub fn generate_jwt_with_claims(&self, claims: TestJwtClaims) -> TestResult<String> {
        let token = format!(
            "test_jwt_token_{}_{}", 
            claims.sub.replace('-', ""), 
            claims.exp
        );
        Ok(token)
    }
    
    /// Generate an admin JWT token
    pub fn generate_admin_token(&self, user: &User) -> TestResult<String> {
        let claims = TestJwtClaims {
            sub: user.id.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
            roles: vec!["admin".to_string()],
            permissions: vec![
                "users.create".to_string(),
                "users.read".to_string(),
                "users.update".to_string(),
                "users.delete".to_string(),
            ],
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };
        
        self.generate_jwt_with_claims(claims)
    }
    
    /// Create a test session
    pub fn create_session(&mut self, user: &User) -> TestResult<String> {
        let session_id = Uuid::new_v4().to_string();
        let session = TestSession {
            id: session_id.clone(),
            user_id: user.id,
            user_name: user.name.clone(),
            user_email: user.email.clone(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(2),
            data: HashMap::new(),
        };
        
        self.session_store.insert(session_id.clone(), session);
        Ok(session_id)
    }
    
    /// Create an admin session
    pub fn create_admin_session(&mut self, user: &User) -> TestResult<String> {
        let session_id = Uuid::new_v4().to_string();
        let session = TestSession {
            id: session_id.clone(),
            user_id: user.id,
            user_name: user.name.clone(),
            user_email: user.email.clone(),
            roles: vec!["admin".to_string()],
            permissions: vec![
                "users.create".to_string(),
                "users.read".to_string(),
                "users.update".to_string(),
                "users.delete".to_string(),
            ],
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(2),
            data: HashMap::new(),
        };
        
        self.session_store.insert(session_id.clone(), session);
        Ok(session_id)
    }
    
    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&TestSession> {
        self.session_store.get(session_id)
    }
    
    /// Validate JWT token (mock implementation)
    pub fn validate_jwt_token(&self, token: &str) -> TestResult<TestJwtClaims> {
        // Mock validation - in real implementation would decode and verify JWT
        if !token.starts_with("test_jwt_token_") {
            return Err(TestError::Authentication("Invalid token format".to_string()));
        }
        
        let parts: Vec<&str> = token.split('_').collect();
        if parts.len() < 4 {
            return Err(TestError::Authentication("Invalid token structure".to_string()));
        }
        
        // Extract user ID and expiration from token
        let user_id = parts[3];
        let exp_str = parts.get(4).copied().unwrap_or("0");
        let exp = exp_str.parse::<usize>().unwrap_or(0);
        
        if exp < Utc::now().timestamp() as usize {
            return Err(TestError::Authentication("Token expired".to_string()));
        }
        
        Ok(TestJwtClaims {
            sub: format!("{}-{}-{}-{}", &user_id[0..8], &user_id[8..12], &user_id[12..16], &user_id[16..20]),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            exp,
            iat: Utc::now().timestamp() as usize,
        })
    }
}

impl Default for TestAuthProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Test JWT claims structure
#[derive(Debug, Clone)]
pub struct TestJwtClaims {
    pub sub: String,        // Subject (user ID)
    pub name: String,       // User name
    pub email: String,      // User email
    pub roles: Vec<String>, // User roles
    pub permissions: Vec<String>, // User permissions
    pub exp: usize,         // Expiration time
    pub iat: usize,         // Issued at time
}

impl TestJwtClaims {
    /// Create claims for a regular user
    pub fn user(user_id: impl Into<String>) -> Self {
        Self {
            sub: user_id.into(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        }
    }
    
    /// Create claims for an admin user
    pub fn admin(user_id: impl Into<String>) -> Self {
        Self {
            sub: user_id.into(),
            name: "Admin User".to_string(),
            email: "admin@example.com".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec![
                "users.create".to_string(),
                "users.read".to_string(),
                "users.update".to_string(),
                "users.delete".to_string(),
            ],
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        }
    }
    
    /// Add a role to the claims
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }
    
    /// Add multiple roles
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles.extend(roles);
        self
    }
    
    /// Add a permission to the claims
    pub fn with_permission(mut self, permission: impl Into<String>) -> Self {
        self.permissions.push(permission.into());
        self
    }
    
    /// Add multiple permissions
    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions.extend(permissions);
        self
    }
    
    /// Set custom expiration time
    pub fn expires_in(mut self, duration: Duration) -> Self {
        self.exp = (Utc::now() + duration).timestamp() as usize;
        self
    }
}

/// Test session structure
#[derive(Debug, Clone)]
pub struct TestSession {
    pub id: String,
    pub user_id: Uuid,
    pub user_name: String,
    pub user_email: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub data: HashMap<String, JsonValue>,
}

impl TestSession {
    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
    
    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
    
    /// Set session data
    pub fn set_data(&mut self, key: impl Into<String>, value: JsonValue) {
        self.data.insert(key.into(), value);
    }
    
    /// Get session data
    pub fn get_data(&self, key: &str) -> Option<&JsonValue> {
        self.data.get(key)
    }
}

/// Test user builder for authentication testing
pub struct TestUserBuilder {
    user: User,
    roles: Vec<String>,
    permissions: Vec<String>,
}

impl TestUserBuilder {
    /// Create a new test user builder
    pub fn new() -> TestResult<Self> {
        let user = UserFactory::new().build()?;
        Ok(Self {
            user,
            roles: vec!["user".to_string()],
            permissions: vec![],
        })
    }
    
    /// Create an admin user
    pub fn admin() -> TestResult<Self> {
        let user = UserFactory::new().build()?;
        Ok(Self {
            user,
            roles: vec!["admin".to_string()],
            permissions: vec![
                "users.create".to_string(),
                "users.read".to_string(),
                "users.update".to_string(),
                "users.delete".to_string(),
            ],
        })
    }
    
    /// Set user name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.user.name = name.into();
        self
    }
    
    /// Set user email
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.user.email = email.into();
        self
    }
    
    /// Add a role
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }
    
    /// Add multiple roles
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles.extend(roles);
        self
    }
    
    /// Add a permission
    pub fn with_permission(mut self, permission: impl Into<String>) -> Self {
        self.permissions.push(permission.into());
        self
    }
    
    /// Add multiple permissions
    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions.extend(permissions);
        self
    }
    
    /// Build the user
    pub fn build(self) -> (User, Vec<String>, Vec<String>) {
        (self.user, self.roles, self.permissions)
    }
    
    /// Generate JWT token for this user
    pub fn generate_jwt_token(self) -> TestResult<(User, String)> {
        let auth_provider = TestAuthProvider::new();
        let (user, roles, permissions) = self.build();
        
        let claims = TestJwtClaims {
            sub: user.id.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
            roles,
            permissions,
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };
        
        let token = auth_provider.generate_jwt_with_claims(claims)?;
        Ok((user, token))
    }
}

impl Default for TestUserBuilder {
    fn default() -> Self {
        Self::new().expect("Failed to create default test user")
    }
}

/// Authorization test helpers
pub struct AuthTestHelpers;

impl AuthTestHelpers {
    /// Create a test user with JWT token
    pub fn user_with_jwt() -> TestResult<(User, String)> {
        TestUserBuilder::new()?.generate_jwt_token()
    }
    
    /// Create an admin user with JWT token
    pub fn admin_with_jwt() -> TestResult<(User, String)> {
        TestUserBuilder::admin()?.generate_jwt_token()
    }
    
    /// Create a user with specific roles and JWT token
    pub fn user_with_roles_and_jwt(roles: Vec<String>) -> TestResult<(User, String)> {
        TestUserBuilder::new()?.with_roles(roles).generate_jwt_token()
    }
    
    /// Create a user with specific permissions and JWT token
    pub fn user_with_permissions_and_jwt(permissions: Vec<String>) -> TestResult<(User, String)> {
        TestUserBuilder::new()?.with_permissions(permissions).generate_jwt_token()
    }
    
    /// Validate that a token has specific roles
    pub fn assert_token_has_roles(token: &str, expected_roles: &[String]) -> TestResult<()> {
        let auth_provider = TestAuthProvider::new();
        let claims = auth_provider.validate_jwt_token(token)?;
        
        for role in expected_roles {
            if !claims.roles.contains(role) {
                return Err(TestError::Assertion {
                    message: format!("Token does not contain required role: {}", role),
                });
            }
        }
        
        Ok(())
    }
    
    /// Validate that a token has specific permissions
    pub fn assert_token_has_permissions(token: &str, expected_permissions: &[String]) -> TestResult<()> {
        let auth_provider = TestAuthProvider::new();
        let claims = auth_provider.validate_jwt_token(token)?;
        
        for permission in expected_permissions {
            if !claims.permissions.contains(permission) {
                return Err(TestError::Assertion {
                    message: format!("Token does not contain required permission: {}", permission),
                });
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_jwt_claims_creation() {
        let claims = TestJwtClaims::user("user123");
        assert_eq!(claims.sub, "user123");
        assert!(claims.roles.contains(&"user".to_string()));
        assert!(claims.exp > Utc::now().timestamp() as usize);
        
        let admin_claims = TestJwtClaims::admin("admin123");
        assert_eq!(admin_claims.sub, "admin123");
        assert!(admin_claims.roles.contains(&"admin".to_string()));
        assert!(!admin_claims.permissions.is_empty());
    }
    
    #[test]
    fn test_jwt_claims_modification() {
        let claims = TestJwtClaims::user("user123")
            .with_role("moderator")
            .with_permission("posts.delete");
            
        assert!(claims.roles.contains(&"moderator".to_string()));
        assert!(claims.permissions.contains(&"posts.delete".to_string()));
    }
    
    #[test]
    fn test_test_auth_provider() -> TestResult<()> {
        let provider = TestAuthProvider::new();
        let user = UserFactory::new().build()?;
        
        let token = provider.generate_jwt_token(&user)?;
        assert!(token.starts_with("test_jwt_token_"));
        
        let claims = provider.validate_jwt_token(&token)?;
        assert!(!claims.sub.is_empty());
        
        Ok(())
    }
    
    #[test]
    fn test_session_functionality() {
        let mut provider = TestAuthProvider::new();
        let user = UserFactory::new().build().unwrap();
        
        let session_id = provider.create_session(&user).unwrap();
        let session = provider.get_session(&session_id).unwrap();
        
        assert_eq!(session.user_id, user.id);
        assert!(session.has_role("user"));
        assert!(!session.is_expired());
    }
    
    #[test]
    fn test_user_builder() -> TestResult<()> {
        let builder = TestUserBuilder::new()?;
        let (user, roles, permissions) = builder
            .with_name("John Doe")
            .with_role("moderator")
            .with_permission("posts.create")
            .build();
            
        assert_eq!(user.name, "John Doe");
        assert!(roles.contains(&"moderator".to_string()));
        assert!(permissions.contains(&"posts.create".to_string()));
        
        Ok(())
    }
    
    #[test]
    fn test_auth_helpers() -> TestResult<()> {
        let (_user, token) = AuthTestHelpers::user_with_jwt()?;
        assert!(!token.is_empty());
        
        let (_admin, admin_token) = AuthTestHelpers::admin_with_jwt()?;
        assert!(!admin_token.is_empty());
        
        Ok(())
    }
}