use elif_orm::{ModelObserver, EventError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use super::models::{User, Post};

// Audit log entry
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuditLog {
    pub id: i64,
    pub table_name: String,
    pub record_id: i64,
    pub action: String,
    pub old_values: Option<Value>,
    pub new_values: Option<Value>,
    pub user_id: Option<i64>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// Audit service for logging all model changes
pub struct AuditService;

impl AuditService {
    pub async fn log_audit(entry: AuditLog) -> Result<(), Box<dyn std::error::Error>> {
        println!("Audit log: {} {} on {} record {}", 
                 entry.action, entry.table_name, entry.table_name, entry.record_id);
        
        if let Some(old_values) = &entry.old_values {
            println!("  Old values: {}", serde_json::to_string_pretty(old_values)?);
        }
        
        if let Some(new_values) = &entry.new_values {
            println!("  New values: {}", serde_json::to_string_pretty(new_values)?);
        }
        
        // In real implementation, this would save to an audit_logs table
        Ok(())
    }
    
    pub fn get_current_user_id() -> Option<i64> {
        // In real implementation, this would get the current authenticated user
        Some(1) // Mock user ID
    }
    
    pub fn get_client_ip() -> Option<String> {
        // In real implementation, this would get the client IP from request context
        Some("192.168.1.1".to_string())
    }
    
    pub fn get_user_agent() -> Option<String> {
        // In real implementation, this would get the user agent from request context
        Some("Mozilla/5.0 (example)".to_string())
    }
}

/// Generic audit observer that can be applied to any model
/// This observer logs all CRUD operations for audit trails
pub struct AuditObserver {
    table_name: String,
}

impl AuditObserver {
    pub fn new(table_name: &str) -> Self {
        Self {
            table_name: table_name.to_string(),
        }
    }
    
    #[allow(dead_code)]
    fn extract_id<T>(&self, _model: &T) -> i64 {
        // In real implementation, this would extract the primary key
        // This might use reflection or a trait to get the ID
        1 // Mock ID
    }
    
    #[allow(dead_code)]
    fn serialize_model<T>(&self, _model: &T) -> Result<Value, EventError> {
        // In real implementation, this would serialize the model to JSON
        // This might use serde or custom serialization
        Ok(serde_json::json!({
            "mock": "data"
        }))
    }
}

// We need to implement ModelObserver for specific types
// In a real implementation, this might be done with macros or generics

#[async_trait]
impl ModelObserver<User> for AuditObserver {
    async fn created(&self, model: &User) -> Result<(), EventError> {
        let audit_entry = AuditLog {
            id: 0, // Would be set by database
            table_name: self.table_name.clone(),
            record_id: model.id,
            action: "INSERT".to_string(),
            old_values: None,
            new_values: Some(serde_json::json!({
                "id": model.id,
                "name": model.name,
                "email": model.email,
            })),
            user_id: AuditService::get_current_user_id(),
            ip_address: AuditService::get_client_ip(),
            user_agent: AuditService::get_user_agent(),
            timestamp: Utc::now(),
        };
        
        if let Err(e) = AuditService::log_audit(audit_entry).await {
            return Err(EventError::observer(&format!("Failed to log audit: {}", e)));
        }
        
        Ok(())
    }
    
    async fn updated(&self, model: &User, original: &User) -> Result<(), EventError> {
        let audit_entry = AuditLog {
            id: 0,
            table_name: self.table_name.clone(),
            record_id: model.id,
            action: "UPDATE".to_string(),
            old_values: Some(serde_json::json!({
                "id": original.id,
                "name": original.name,
                "email": original.email,
            })),
            new_values: Some(serde_json::json!({
                "id": model.id,
                "name": model.name,
                "email": model.email,
            })),
            user_id: AuditService::get_current_user_id(),
            ip_address: AuditService::get_client_ip(),
            user_agent: AuditService::get_user_agent(),
            timestamp: Utc::now(),
        };
        
        if let Err(e) = AuditService::log_audit(audit_entry).await {
            return Err(EventError::observer(&format!("Failed to log audit: {}", e)));
        }
        
        Ok(())
    }
    
    async fn deleted(&self, model: &User) -> Result<(), EventError> {
        let audit_entry = AuditLog {
            id: 0,
            table_name: self.table_name.clone(),
            record_id: model.id,
            action: "DELETE".to_string(),
            old_values: Some(serde_json::json!({
                "id": model.id,
                "name": model.name,
                "email": model.email,
            })),
            new_values: None,
            user_id: AuditService::get_current_user_id(),
            ip_address: AuditService::get_client_ip(),
            user_agent: AuditService::get_user_agent(),
            timestamp: Utc::now(),
        };
        
        if let Err(e) = AuditService::log_audit(audit_entry).await {
            return Err(EventError::observer(&format!("Failed to log audit: {}", e)));
        }
        
        Ok(())
    }
}

// Implementation for Post model
#[async_trait]
impl ModelObserver<Post> for AuditObserver {
    async fn created(&self, model: &Post) -> Result<(), EventError> {
        let audit_entry = AuditLog {
            id: 0, // Would be set by database
            table_name: self.table_name.clone(),
            record_id: model.id,
            action: "INSERT".to_string(),
            old_values: None,
            new_values: Some(serde_json::json!({
                "id": model.id,
                "title": model.title,
                "content": model.content,
                "author_id": model.author_id,
                "published": model.published,
                "published_at": model.published_at,
                "slug": model.slug,
            })),
            user_id: AuditService::get_current_user_id(),
            ip_address: AuditService::get_client_ip(),
            user_agent: AuditService::get_user_agent(),
            timestamp: Utc::now(),
        };
        
        if let Err(e) = AuditService::log_audit(audit_entry).await {
            return Err(EventError::observer(&format!("Failed to log audit: {}", e)));
        }
        
        Ok(())
    }
    
    async fn updated(&self, model: &Post, original: &Post) -> Result<(), EventError> {
        let audit_entry = AuditLog {
            id: 0,
            table_name: self.table_name.clone(),
            record_id: model.id,
            action: "UPDATE".to_string(),
            old_values: Some(serde_json::json!({
                "id": original.id,
                "title": original.title,
                "content": original.content,
                "author_id": original.author_id,
                "published": original.published,
                "published_at": original.published_at,
                "slug": original.slug,
            })),
            new_values: Some(serde_json::json!({
                "id": model.id,
                "title": model.title,
                "content": model.content,
                "author_id": model.author_id,
                "published": model.published,
                "published_at": model.published_at,
                "slug": model.slug,
            })),
            user_id: AuditService::get_current_user_id(),
            ip_address: AuditService::get_client_ip(),
            user_agent: AuditService::get_user_agent(),
            timestamp: Utc::now(),
        };
        
        if let Err(e) = AuditService::log_audit(audit_entry).await {
            return Err(EventError::observer(&format!("Failed to log audit: {}", e)));
        }
        
        Ok(())
    }
    
    async fn deleted(&self, model: &Post) -> Result<(), EventError> {
        let audit_entry = AuditLog {
            id: 0,
            table_name: self.table_name.clone(),
            record_id: model.id,
            action: "DELETE".to_string(),
            old_values: Some(serde_json::json!({
                "id": model.id,
                "title": model.title,
                "content": model.content,
                "author_id": model.author_id,
                "published": model.published,
                "published_at": model.published_at,
                "slug": model.slug,
            })),
            new_values: None,
            user_id: AuditService::get_current_user_id(),
            ip_address: AuditService::get_client_ip(),
            user_agent: AuditService::get_user_agent(),
            timestamp: Utc::now(),
        };
        
        if let Err(e) = AuditService::log_audit(audit_entry).await {
            return Err(EventError::observer(&format!("Failed to log audit: {}", e)));
        }
        
        Ok(())
    }
}

// Security audit observer for sensitive operations
pub struct SecurityAuditObserver {
    #[allow(dead_code)]
    table_name: String,
}

impl SecurityAuditObserver {
    pub fn new(table_name: &str) -> Self {
        Self {
            table_name: table_name.to_string(),
        }
    }
}

#[async_trait]
impl ModelObserver<User> for SecurityAuditObserver {
    async fn creating(&self, model: &mut User) -> Result<(), EventError> {
        // Log security-sensitive operations
        println!("SECURITY AUDIT: Creating user {} with email {}", model.name, model.email);
        
        // Could add additional security checks here:
        // - Check for suspicious patterns
        // - Rate limiting
        // - Geographic restrictions
        // - Time-based restrictions
        
        Ok(())
    }
    
    async fn updating(&self, model: &mut User, original: &User) -> Result<(), EventError> {
        // Log sensitive field changes
        if model.email != original.email {
            println!("SECURITY AUDIT: Email change for user {} from {} to {}", 
                     model.id, original.email, model.email);
        }
        
        // Could trigger additional security measures:
        // - Require email verification
        // - Send security notification
        // - Log IP address changes
        
        Ok(())
    }
    
    async fn deleting(&self, model: &User) -> Result<(), EventError> {
        println!("SECURITY AUDIT: Deleting user {} ({})", model.name, model.email);
        
        // Could add deletion protections:
        // - Require admin approval
        // - Implement cooling-off period
        // - Archive instead of delete
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_observer_created() {
        let observer = AuditObserver::new("users");
        let user = User {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };
        
        let result = observer.created(&user).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_audit_observer_updated() {
        let observer = AuditObserver::new("users");
        let original = User {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };
        let updated = User {
            id: 1,
            name: "John Smith".to_string(),
            email: "john.smith@example.com".to_string(),
        };
        
        let result = observer.updated(&updated, &original).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_audit_observer_deleted() {
        let observer = AuditObserver::new("users");
        let user = User {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };
        
        let result = observer.deleted(&user).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_security_audit_observer() {
        let observer = SecurityAuditObserver::new("users");
        let mut user = User {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };
        
        let result = observer.creating(&mut user).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_audit_observer_post_created() {
        use chrono::Utc;
        
        let observer = AuditObserver::new("posts");
        let post = Post {
            id: 1,
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            author_id: 1,
            published: true,
            published_at: Some(Utc::now()),
            slug: "test-post".to_string(),
        };
        
        let result = observer.created(&post).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_audit_observer_post_updated() {
        use chrono::Utc;
        
        let observer = AuditObserver::new("posts");
        let original = Post {
            id: 1,
            title: "Original Title".to_string(),
            content: "Original content".to_string(),
            author_id: 1,
            published: false,
            published_at: None,
            slug: "original-title".to_string(),
        };
        let updated = Post {
            id: 1,
            title: "Updated Title".to_string(),
            content: "Updated content".to_string(),
            author_id: 1,
            published: true,
            published_at: Some(Utc::now()),
            slug: "updated-title".to_string(),
        };
        
        let result = observer.updated(&updated, &original).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_audit_observer_post_deleted() {
        use chrono::Utc;
        
        let observer = AuditObserver::new("posts");
        let post = Post {
            id: 1,
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            author_id: 1,
            published: true,
            published_at: Some(Utc::now()),
            slug: "test-post".to_string(),
        };
        
        let result = observer.deleted(&post).await;
        assert!(result.is_ok());
    }
}