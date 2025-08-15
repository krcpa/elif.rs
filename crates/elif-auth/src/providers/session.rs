//! Session-based authentication provider
//! 
//! This module provides session-based authentication with multiple storage backends.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    traits::{AuthProvider, Authenticatable, AuthenticationResult, SessionStorage},
    utils::CryptoUtils,
    AuthError, AuthResult
};

/// Session ID type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Generate a new secure session ID
    pub fn generate() -> Self {
        Self(CryptoUtils::generate_token(32))
    }
    
    /// Create from string (for validation/parsing)
    pub fn from_string(s: String) -> AuthResult<Self> {
        if s.len() < 16 {
            return Err(AuthError::token_error("Session ID too short"));
        }
        Ok(Self(s))
    }
    
    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Session data stored for each session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// User ID
    pub user_id: String,
    
    /// Username for quick access
    pub username: String,
    
    /// User roles
    pub roles: Vec<String>,
    
    /// User permissions
    pub permissions: Vec<String>,
    
    /// Session creation time
    pub created_at: DateTime<Utc>,
    
    /// Last accessed time
    pub last_accessed: DateTime<Utc>,
    
    /// Session expiration time
    pub expires_at: DateTime<Utc>,
    
    /// CSRF token for this session
    pub csrf_token: Option<String>,
    
    /// Client IP address (for security)
    pub ip_address: Option<String>,
    
    /// User agent (for security)
    pub user_agent: Option<String>,
    
    /// Additional session metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionData {
    /// Create new session data
    pub fn new<U: Authenticatable>(
        user: &U,
        expires_in: Duration,
        csrf_token: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            user_id: format!("{:?}", user.id()),
            username: user.username().to_string(),
            roles: user.roles(),
            permissions: user.permissions(),
            created_at: now,
            last_accessed: now,
            expires_at: now + expires_in,
            csrf_token,
            ip_address,
            user_agent,
            metadata: user.additional_data(),
        }
    }
    
    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = Utc::now();
    }
    
    /// Extend session expiration
    pub fn extend(&mut self, duration: Duration) {
        self.expires_at = Utc::now() + duration;
        self.touch();
    }
}

// <<<ELIF:BEGIN agent-editable:session-storage-memory>>>
/// In-memory session storage implementation
/// 
/// **Warning**: This is for development/testing only. Sessions will be lost on restart.
#[derive(Debug)]
pub struct MemorySessionStorage {
    sessions: Arc<RwLock<HashMap<SessionId, (SessionData, DateTime<Utc>)>>>,
}

impl MemorySessionStorage {
    /// Create new memory session storage
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemorySessionStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStorage for MemorySessionStorage {
    type SessionId = SessionId;
    type SessionData = SessionData;
    
    async fn create_session(
        &self,
        data: Self::SessionData,
        expires_at: DateTime<Utc>,
    ) -> AuthResult<Self::SessionId> {
        let session_id = SessionId::generate();
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), (data, expires_at));
        Ok(session_id)
    }
    
    async fn get_session(&self, id: &Self::SessionId) -> AuthResult<Option<Self::SessionData>> {
        let sessions = self.sessions.read().await;
        if let Some((data, expires_at)) = sessions.get(id) {
            // Check if session is expired
            if Utc::now() > *expires_at {
                // Clean up expired session
                drop(sessions);
                let mut sessions = self.sessions.write().await;
                sessions.remove(id);
                return Ok(None);
            }
            Ok(Some(data.clone()))
        } else {
            Ok(None)
        }
    }
    
    async fn update_session(
        &self,
        id: &Self::SessionId,
        data: Self::SessionData,
        expires_at: DateTime<Utc>,
    ) -> AuthResult<()> {
        let mut sessions = self.sessions.write().await;
        if sessions.contains_key(id) {
            sessions.insert(id.clone(), (data, expires_at));
            Ok(())
        } else {
            Err(AuthError::token_error("Session not found"))
        }
    }
    
    async fn delete_session(&self, id: &Self::SessionId) -> AuthResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id);
        Ok(())
    }
    
    async fn cleanup_expired_sessions(&self) -> AuthResult<u64> {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        let initial_count = sessions.len();
        
        sessions.retain(|_, (_, expires_at)| now <= *expires_at);
        
        let cleaned = (initial_count - sessions.len()) as u64;
        Ok(cleaned)
    }
    
    async fn get_session_expiry(&self, id: &Self::SessionId) -> AuthResult<Option<DateTime<Utc>>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id).map(|(_, expires_at)| *expires_at))
    }
    
    async fn extend_session(&self, id: &Self::SessionId, expires_at: DateTime<Utc>) -> AuthResult<()> {
        let mut sessions = self.sessions.write().await;
        if let Some((data, _)) = sessions.get(id).cloned() {
            sessions.insert(id.clone(), (data, expires_at));
            Ok(())
        } else {
            Err(AuthError::token_error("Session not found"))
        }
    }
}
// <<<ELIF:END agent-editable:session-storage-memory>>>

// <<<ELIF:BEGIN agent-editable:session-provider>>>
/// Session-based authentication provider
#[derive(Debug)]
pub struct SessionProvider<S, U> 
where
    S: SessionStorage<SessionId = SessionId, SessionData = SessionData>,
    U: Authenticatable,
{
    storage: Arc<S>,
    session_duration: Duration,
    cleanup_interval: Duration,
    _phantom: std::marker::PhantomData<U>,
}

impl<S, U> SessionProvider<S, U>
where
    S: SessionStorage<SessionId = SessionId, SessionData = SessionData>,
    U: Authenticatable + Clone,
{
    /// Create new session provider
    pub fn new(
        storage: S,
        session_duration: Duration,
        cleanup_interval: Duration,
    ) -> Self {
        Self {
            storage: Arc::new(storage),
            session_duration,
            cleanup_interval,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Create session provider with default settings
    pub fn with_default_config(storage: S) -> Self {
        Self::new(
            storage,
            Duration::hours(24), // 24 hour sessions
            Duration::hours(1),  // Cleanup every hour
        )
    }
    
    /// Get session storage reference
    pub fn storage(&self) -> &S {
        &self.storage
    }
    
    /// Create a new session for authenticated user
    pub async fn create_session(
        &self,
        user: &U,
        csrf_token: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> AuthResult<SessionId> {
        let session_data = SessionData::new(
            user,
            self.session_duration,
            csrf_token,
            ip_address,
            user_agent,
        );
        
        self.storage
            .create_session(session_data, Utc::now() + self.session_duration)
            .await
    }
    
    /// Validate session and return session data
    pub async fn validate_session(&self, session_id: &SessionId) -> AuthResult<SessionData> {
        match self.storage.get_session(session_id).await? {
            Some(mut session_data) => {
                if session_data.is_expired() {
                    // Clean up expired session
                    let _ = self.storage.delete_session(session_id).await;
                    return Err(AuthError::token_error("Session expired"));
                }
                
                // Update last accessed time
                session_data.touch();
                self.storage
                    .update_session(session_id, session_data.clone(), session_data.expires_at)
                    .await?;
                
                Ok(session_data)
            }
            None => Err(AuthError::token_error("Session not found")),
        }
    }
    
    /// Extend session expiration
    pub async fn extend_session(&self, session_id: &SessionId) -> AuthResult<()> {
        let new_expiry = Utc::now() + self.session_duration;
        self.storage.extend_session(session_id, new_expiry).await
    }
    
    /// Delete session (logout)
    pub async fn destroy_session(&self, session_id: &SessionId) -> AuthResult<()> {
        self.storage.delete_session(session_id).await
    }
    
    /// Clean up expired sessions
    pub async fn cleanup_expired(&self) -> AuthResult<u64> {
        self.storage.cleanup_expired_sessions().await
    }
}

/// Simple user finder trait for session authentication
#[async_trait]
pub trait UserFinder<U: Authenticatable>: Send + Sync {
    /// Find user by ID
    async fn find_by_id(&self, id: &str) -> AuthResult<Option<U>>;
}

/// Session credentials (just the session ID)
#[derive(Debug, Clone)]
pub struct SessionCredentials {
    pub session_id: SessionId,
}

#[async_trait]
impl<S, U> AuthProvider<U> for SessionProvider<S, U>
where
    S: SessionStorage<SessionId = SessionId, SessionData = SessionData>,
    U: Authenticatable + Clone,
{
    type Token = SessionId;
    type Credentials = SessionCredentials;
    
    async fn authenticate(
        &self,
        credentials: &Self::Credentials,
    ) -> AuthResult<AuthenticationResult<U, Self::Token>> {
        // For session authentication, we validate the existing session
        let _session_data = self.validate_session(&credentials.session_id).await?;
        
        // This is a simplified implementation - in practice you'd want to
        // reconstruct the full User object from the session data
        // For now, we'll return an error indicating this needs a user finder
        Err(AuthError::generic_error(
            "Session authentication requires a UserFinder implementation"
        ))
    }
    
    async fn validate_token(&self, _token: &Self::Token) -> AuthResult<U> {
        // Similar limitation - need UserFinder to reconstruct User from session
        Err(AuthError::generic_error(
            "Token validation requires a UserFinder implementation"
        ))
    }
    
    async fn revoke_token(&self, token: &Self::Token) -> AuthResult<()> {
        self.destroy_session(token).await
    }
    
    fn provider_name(&self) -> &str {
        "session"
    }
}
// <<<ELIF:END agent-editable:session-provider>>>

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock user for testing
    #[derive(Debug, Clone)]
    struct MockUser {
        id: String,
        username: String,
        roles: Vec<String>,
        permissions: Vec<String>,
        active: bool,
    }
    
    #[async_trait]
    impl Authenticatable for MockUser {
        type Id = String;
        type Credentials = String;
        
        fn id(&self) -> &Self::Id {
            &self.id
        }
        
        fn username(&self) -> &str {
            &self.username
        }
        
        fn is_active(&self) -> bool {
            self.active
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
    
    #[tokio::test]
    async fn test_session_id_generation() {
        let session_id = SessionId::generate();
        assert!(session_id.as_str().len() >= 32);
        
        let session_id2 = SessionId::generate();
        assert_ne!(session_id.as_str(), session_id2.as_str());
    }
    
    #[tokio::test]
    async fn test_session_id_from_string() {
        let valid_id = "a".repeat(32);
        let session_id = SessionId::from_string(valid_id.clone()).unwrap();
        assert_eq!(session_id.as_str(), &valid_id);
        
        let short_id = "short";
        let result = SessionId::from_string(short_id.to_string());
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_session_data_creation() {
        let user = MockUser {
            id: "123".to_string(),
            username: "test@example.com".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
            active: true,
        };
        
        let session_data = SessionData::new(
            &user,
            Duration::hours(24),
            Some("csrf_token".to_string()),
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        );
        
        assert_eq!(session_data.user_id, "\"123\"");
        assert_eq!(session_data.username, "test@example.com");
        assert_eq!(session_data.roles, vec!["admin"]);
        assert_eq!(session_data.permissions, vec!["read", "write"]);
        assert_eq!(session_data.csrf_token, Some("csrf_token".to_string()));
        assert!(!session_data.is_expired());
    }
    
    #[tokio::test]
    async fn test_session_data_expiration() {
        let user = MockUser {
            id: "123".to_string(),
            username: "test@example.com".to_string(),
            roles: vec![],
            permissions: vec![],
            active: true,
        };
        
        let mut session_data = SessionData::new(
            &user,
            Duration::milliseconds(-1), // Expired immediately
            None,
            None,
            None,
        );
        
        // Should be expired
        assert!(session_data.is_expired());
        
        // Extend it
        session_data.extend(Duration::hours(1));
        assert!(!session_data.is_expired());
    }
    
    #[tokio::test]
    async fn test_memory_session_storage() {
        let storage = MemorySessionStorage::new();
        let user = MockUser {
            id: "123".to_string(),
            username: "test@example.com".to_string(),
            roles: vec![],
            permissions: vec![],
            active: true,
        };
        
        let session_data = SessionData::new(
            &user,
            Duration::hours(24),
            None,
            None,
            None,
        );
        
        let expires_at = Utc::now() + Duration::hours(24);
        
        // Create session
        let session_id = storage.create_session(session_data.clone(), expires_at).await.unwrap();
        
        // Get session
        let retrieved = storage.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user_id, session_data.user_id);
        
        // Delete session
        storage.delete_session(&session_id).await.unwrap();
        
        // Verify deleted
        let retrieved = storage.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_none());
    }
    
    #[tokio::test]
    async fn test_memory_session_storage_expired_cleanup() {
        let storage = MemorySessionStorage::new();
        let user = MockUser {
            id: "123".to_string(),
            username: "test@example.com".to_string(),
            roles: vec![],
            permissions: vec![],
            active: true,
        };
        
        let session_data = SessionData::new(
            &user,
            Duration::milliseconds(-1), // Expired
            None,
            None,
            None,
        );
        
        let expires_at = Utc::now() - Duration::hours(1); // Expired
        
        // Create expired session
        let session_id = storage.create_session(session_data, expires_at).await.unwrap();
        
        // Try to get expired session - should return None and clean up
        let retrieved = storage.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_none());
    }
    
    #[tokio::test]
    async fn test_session_provider_creation() {
        let storage = MemorySessionStorage::new();
        let provider: SessionProvider<MemorySessionStorage, MockUser> = 
            SessionProvider::with_default_config(storage);
        
        assert_eq!(provider.provider_name(), "session");
    }
    
    #[tokio::test]
    async fn test_session_provider_session_lifecycle() {
        let storage = MemorySessionStorage::new();
        let provider = SessionProvider::with_default_config(storage);
        
        let user = MockUser {
            id: "123".to_string(),
            username: "test@example.com".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["read".to_string()],
            active: true,
        };
        
        // Create session
        let session_id = provider.create_session(
            &user,
            Some("csrf_token".to_string()),
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        ).await.unwrap();
        
        // Validate session
        let session_data = provider.validate_session(&session_id).await.unwrap();
        assert_eq!(session_data.username, "test@example.com");
        assert_eq!(session_data.csrf_token, Some("csrf_token".to_string()));
        
        // Extend session
        provider.extend_session(&session_id).await.unwrap();
        
        // Destroy session
        provider.destroy_session(&session_id).await.unwrap();
        
        // Verify destroyed
        let result = provider.validate_session(&session_id).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_session_cleanup() {
        let storage = MemorySessionStorage::new();
        let provider: SessionProvider<MemorySessionStorage, MockUser> = SessionProvider::with_default_config(storage);
        
        // The storage should start empty
        let cleaned = provider.cleanup_expired().await.unwrap();
        assert_eq!(cleaned, 0);
    }
}