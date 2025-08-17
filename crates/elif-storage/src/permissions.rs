//! File access control and permissions

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// File permissions and access control
#[cfg(feature = "access-control")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    /// Owner user ID
    pub owner: Option<String>,
    
    /// Allowed users (user IDs)
    pub allowed_users: Option<HashSet<String>>,
    
    /// Allowed roles
    pub allowed_roles: Option<HashSet<String>>,
    
    /// Public read access
    pub public_read: bool,
    
    /// Public write access
    pub public_write: bool,
    
    /// Expiration time for temporary access
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(feature = "access-control")]
impl Default for FilePermissions {
    fn default() -> Self {
        Self {
            owner: None,
            allowed_users: None,
            allowed_roles: None,
            public_read: false,
            public_write: false,
            expires_at: None,
        }
    }
}

#[cfg(feature = "access-control")]
impl FilePermissions {
    /// Create new file permissions
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set file owner
    pub fn owner(mut self, owner: String) -> Self {
        self.owner = Some(owner);
        self
    }
    
    /// Allow specific user
    pub fn allow_user(mut self, user_id: String) -> Self {
        self.allowed_users.get_or_insert_with(HashSet::new).insert(user_id);
        self
    }
    
    /// Allow specific role
    pub fn allow_role(mut self, role: String) -> Self {
        self.allowed_roles.get_or_insert_with(HashSet::new).insert(role);
        self
    }
    
    /// Enable public read access
    pub fn public_read(mut self) -> Self {
        self.public_read = true;
        self
    }
    
    /// Enable public write access
    pub fn public_write(mut self) -> Self {
        self.public_write = true;
        self
    }
    
    /// Set expiration time
    pub fn expires_at(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
    
    /// Check if user has read access
    pub fn can_read(&self, user_id: Option<&str>, user_roles: &[String]) -> bool {
        // Check if expired
        if let Some(expires) = self.expires_at {
            if chrono::Utc::now() > expires {
                return false;
            }
        }
        
        // Public read access
        if self.public_read {
            return true;
        }
        
        // Owner access
        if let (Some(owner), Some(user)) = (&self.owner, user_id) {
            if owner == user {
                return true;
            }
        }
        
        // User-specific access
        if let (Some(allowed_users), Some(user)) = (&self.allowed_users, user_id) {
            if allowed_users.contains(user) {
                return true;
            }
        }
        
        // Role-based access
        if let Some(allowed_roles) = &self.allowed_roles {
            if user_roles.iter().any(|role| allowed_roles.contains(role)) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if user has write access
    pub fn can_write(&self, user_id: Option<&str>, user_roles: &[String]) -> bool {
        // Check if expired
        if let Some(expires) = self.expires_at {
            if chrono::Utc::now() > expires {
                return false;
            }
        }
        
        // Public write access
        if self.public_write {
            return true;
        }
        
        // Owner always has write access
        if let (Some(owner), Some(user)) = (&self.owner, user_id) {
            if owner == user {
                return true;
            }
        }
        
        // For write access, we need explicit permission (not just read access)
        // This is more restrictive than read access
        false
    }
}

#[cfg(test)]
#[cfg(feature = "access-control")]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_permissions() {
        let permissions = FilePermissions::new()
            .owner("user123".to_string())
            .allow_user("user456".to_string())
            .allow_role("editor".to_string())
            .public_read();
        
        // Owner can read and write
        assert!(permissions.can_read(Some("user123"), &[]));
        assert!(permissions.can_write(Some("user123"), &[]));
        
        // Allowed user can read
        assert!(permissions.can_read(Some("user456"), &[]));
        assert!(!permissions.can_write(Some("user456"), &[])); // No write access
        
        // User with editor role can read
        assert!(permissions.can_read(Some("user789"), &["editor".to_string()]));
        
        // Public can read
        assert!(permissions.can_read(None, &[]));
        
        // Random user cannot write
        assert!(!permissions.can_write(Some("random"), &[]));
    }
    
    #[test]
    fn test_expired_permissions() {
        let expired_time = chrono::Utc::now() - chrono::Duration::hours(1);
        let permissions = FilePermissions::new()
            .public_read()
            .expires_at(expired_time);
        
        // Should not have access after expiration
        assert!(!permissions.can_read(None, &[]));
        assert!(!permissions.can_write(None, &[]));
    }
}