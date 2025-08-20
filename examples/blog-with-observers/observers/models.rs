use chrono::{DateTime, Utc};

/// User model - represents a user in the system
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: 1,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        }
    }
}

impl User {
    #[allow(dead_code)]
    pub fn new(email: &str) -> Self {
        Self {
            id: 1, // In real app, this would be set by database
            name: "New User".to_string(),
            email: email.to_string(),
        }
    }
}

/// Post model - represents a blog post
#[derive(Debug, Clone, PartialEq)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub slug: String,
}

impl Post {
    #[allow(dead_code)]
    pub fn new(title: &str) -> Self {
        Self {
            id: 1, // In real app, this would be set by database
            title: title.to_string(),
            content: "Default content".to_string(),
            author_id: 1,
            published: false,
            published_at: None,
            slug: String::new(),
        }
    }
}

/// Profile model - represents a user profile
#[derive(Debug, Clone, PartialEq)]
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

    #[allow(dead_code)]
    pub async fn create_default(user_id: i64) -> Result<Self, Box<dyn std::error::Error>> {
        let profile = Self {
            user_id,
            display_name: format!("User {}", user_id),
        };
        Self::create("pool", profile.clone()).await?;
        Ok(profile)
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