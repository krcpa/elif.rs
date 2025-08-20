use elif_orm::{ModelObserver, EventError};
use async_trait::async_trait;

// Example User model (simplified)
#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

// Email service example (simplified)
pub struct EmailService;

impl EmailService {
    pub async fn send_welcome_email(user: &User) -> Result<(), Box<dyn std::error::Error>> {
        println!("Sending welcome email to {}", user.email);
        // In real implementation, this would send an actual email
        Ok(())
    }
}

// Profile model example (simplified)
#[derive(Debug, Clone)]
pub struct Profile {
    pub user_id: i64,
    pub display_name: String,
}

impl Profile {
    pub async fn create(_pool: &str, profile: Profile) -> Result<(), Box<dyn std::error::Error>> {
        println!("Creating profile for user {}: {}", profile.user_id, profile.display_name);
        // In real implementation, this would create a database record
        Ok(())
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            user_id: 0,
            display_name: String::new(),
        }
    }
}

/// Observer for User model that handles email normalization and welcome emails
pub struct UserObserver;

#[async_trait]
impl ModelObserver<User> for UserObserver {
    async fn creating(&self, user: &mut User) -> Result<(), EventError> {
        // Normalize email before creating
        user.email = user.email.to_lowercase();
        
        // Validate email uniqueness (simplified check)
        if user.email == "duplicate@example.com" {
            return Err(EventError::validation("Email already exists"));
        }
        
        println!("User creating: normalized email to {}", user.email);
        Ok(())
    }
    
    async fn created(&self, user: &User) -> Result<(), EventError> {
        // Send welcome email (async operation)
        if let Err(e) = EmailService::send_welcome_email(user).await {
            return Err(EventError::observer(&format!("Failed to send welcome email: {}", e)));
        }
        
        // Create default user profile
        let profile = Profile {
            user_id: user.id,
            display_name: user.name.clone(),
            ..Default::default()
        };
        
        if let Err(e) = Profile::create("pool", profile).await {
            return Err(EventError::observer(&format!("Failed to create profile: {}", e)));
        }
        
        println!("User created: welcome email sent and profile created for {}", user.name);
        Ok(())
    }
    
    async fn updating(&self, user: &mut User, original: &User) -> Result<(), EventError> {
        // Log the change
        if user.email != original.email {
            user.email = user.email.to_lowercase();
            println!("User updating: email changed from {} to {}", original.email, user.email);
        }
        
        if user.name != original.name {
            println!("User updating: name changed from {} to {}", original.name, user.name);
        }
        
        Ok(())
    }
    
    async fn updated(&self, user: &User, original: &User) -> Result<(), EventError> {
        // Send email notification if email was changed
        if user.email != original.email {
            println!("User updated: sending email change notification to {}", user.email);
            // In real implementation, send notification email
        }
        
        Ok(())
    }
    
    async fn deleting(&self, user: &User) -> Result<(), EventError> {
        // Log deletion attempt
        println!("User deleting: preparing to delete {}", user.name);
        
        // Could add soft-delete logic here
        // Could check for dependencies and prevent deletion
        
        Ok(())
    }
    
    async fn deleted(&self, user: &User) -> Result<(), EventError> {
        // Clean up related data
        println!("User deleted: cleaning up data for {}", user.name);
        
        // In real implementation:
        // - Delete related profiles, posts, etc.
        // - Archive user data
        // - Send deletion confirmation email
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_observer_email_normalization() {
        let observer = UserObserver;
        let mut user = User {
            id: 1,
            name: "John Doe".to_string(),
            email: "JOHN@EXAMPLE.COM".to_string(),
        };
        
        let result = observer.creating(&mut user).await;
        assert!(result.is_ok());
        assert_eq!(user.email, "john@example.com");
    }
    
    #[tokio::test]
    async fn test_user_observer_duplicate_email_validation() {
        let observer = UserObserver;
        let mut user = User {
            id: 1,
            name: "Test User".to_string(),
            email: "duplicate@example.com".to_string(),
        };
        
        let result = observer.creating(&mut user).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            EventError::Validation { message, .. } => {
                assert_eq!(message, "Email already exists");
            }
            _ => panic!("Expected validation error"),
        }
    }
    
    #[tokio::test]
    async fn test_user_observer_created_flow() {
        let observer = UserObserver;
        let user = User {
            id: 1,
            name: "Jane Doe".to_string(),
            email: "jane@example.com".to_string(),
        };
        
        let result = observer.created(&user).await;
        assert!(result.is_ok());
    }
}