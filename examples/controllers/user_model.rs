//! User Model Example - Demonstrates ORM integration with timestamps and validation
//! 
//! Example implementation of a User model using the elif-orm Model trait
//! with full CRUD support, timestamps, and field validation.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres, Row};

use elif_orm::{Model, ModelResult, ModelError};

/// User model with timestamps and validation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Create a new User instance
    pub fn new(name: String, email: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            email,
            age: None,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a User with age
    pub fn with_age(name: String, email: String, age: i32) -> Self {
        let mut user = Self::new(name, email);
        user.age = Some(age);
        user
    }

    /// Validate email format (basic validation)
    pub fn is_valid_email(&self) -> bool {
        self.email.contains('@') && self.email.contains('.')
    }

    /// Validate required fields
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Name cannot be empty".to_string());
        }

        if self.email.trim().is_empty() {
            return Err("Email cannot be empty".to_string());
        }

        if !self.is_valid_email() {
            return Err("Invalid email format".to_string());
        }

        if let Some(age) = self.age {
            if age < 0 || age > 150 {
                return Err("Age must be between 0 and 150".to_string());
            }
        }

        Ok(())
    }
}

impl Model for User {
    type PrimaryKey = Uuid;

    fn table_name() -> &'static str {
        "users"
    }

    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        Some(self.id)
    }

    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = key;
    }

    fn uses_timestamps() -> bool {
        true
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        Some(self.created_at)
    }

    fn set_created_at(&mut self, timestamp: DateTime<Utc>) {
        self.created_at = timestamp;
    }

    fn updated_at(&self) -> Option<DateTime<Utc>> {
        Some(self.updated_at)
    }

    fn set_updated_at(&mut self, timestamp: DateTime<Utc>) {
        self.updated_at = timestamp;
    }

    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        Ok(User {
            id: row.try_get("id").map_err(|e| ModelError::DatabaseError(e.to_string()))?,
            name: row.try_get("name").map_err(|e| ModelError::DatabaseError(e.to_string()))?,
            email: row.try_get("email").map_err(|e| ModelError::DatabaseError(e.to_string()))?,
            age: row.try_get("age").map_err(|e| ModelError::DatabaseError(e.to_string()))?,
            is_active: row.try_get("is_active").map_err(|e| ModelError::DatabaseError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| ModelError::DatabaseError(e.to_string()))?,
            updated_at: row.try_get("updated_at").map_err(|e| ModelError::DatabaseError(e.to_string()))?,
        })
    }

    fn to_fields(&self) -> HashMap<String, serde_json::Value> {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), serde_json::json!(self.id));
        fields.insert("name".to_string(), serde_json::json!(self.name));
        fields.insert("email".to_string(), serde_json::json!(self.email));
        fields.insert("age".to_string(), serde_json::json!(self.age));
        fields.insert("is_active".to_string(), serde_json::json!(self.is_active));
        fields.insert("created_at".to_string(), serde_json::json!(self.created_at));
        fields.insert("updated_at".to_string(), serde_json::json!(self.updated_at));
        fields
    }
}

/// DTO for creating new users
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub age: Option<i32>,
}

impl CreateUserRequest {
    pub fn into_user(self) -> User {
        match self.age {
            Some(age) => User::with_age(self.name, self.email, age),
            None => User::new(self.name, self.email),
        }
    }
}

/// DTO for updating users
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub age: Option<i32>,
    pub is_active: Option<bool>,
}

impl UpdateUserRequest {
    pub fn apply_to_user(&self, user: &mut User) {
        if let Some(name) = &self.name {
            user.name = name.clone();
        }
        if let Some(email) = &self.email {
            user.email = email.clone();
        }
        if let Some(age) = self.age {
            user.age = Some(age);
        }
        if let Some(is_active) = self.is_active {
            user.is_active = is_active;
        }
        user.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("John Doe".to_string(), "john@example.com".to_string());
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
        assert_eq!(user.age, None);
        assert!(user.is_active);
        assert!(user.created_at <= Utc::now());
        assert!(user.updated_at <= Utc::now());
    }

    #[test]
    fn test_user_with_age() {
        let user = User::with_age("Jane Doe".to_string(), "jane@example.com".to_string(), 30);
        assert_eq!(user.age, Some(30));
    }

    #[test]
    fn test_email_validation() {
        let user = User::new("Test".to_string(), "test@example.com".to_string());
        assert!(user.is_valid_email());

        let invalid_user = User::new("Test".to_string(), "invalid-email".to_string());
        assert!(!invalid_user.is_valid_email());
    }

    #[test]
    fn test_user_validation() {
        let valid_user = User::new("John".to_string(), "john@example.com".to_string());
        assert!(valid_user.validate().is_ok());

        let empty_name = User::new("".to_string(), "john@example.com".to_string());
        assert!(empty_name.validate().is_err());

        let invalid_email = User::new("John".to_string(), "invalid".to_string());
        assert!(invalid_email.validate().is_err());

        let invalid_age = User::with_age("John".to_string(), "john@example.com".to_string(), 200);
        assert!(invalid_age.validate().is_err());
    }

    #[test]
    fn test_create_user_request() {
        let request = CreateUserRequest {
            name: "John".to_string(),
            email: "john@example.com".to_string(),
            age: Some(25),
        };

        let user = request.into_user();
        assert_eq!(user.name, "John");
        assert_eq!(user.email, "john@example.com");
        assert_eq!(user.age, Some(25));
    }

    #[test]
    fn test_update_user_request() {
        let mut user = User::new("John".to_string(), "john@example.com".to_string());
        let original_updated_at = user.updated_at;

        let update_request = UpdateUserRequest {
            name: Some("Jane".to_string()),
            email: None,
            age: Some(30),
            is_active: Some(false),
        };

        update_request.apply_to_user(&mut user);
        
        assert_eq!(user.name, "Jane");
        assert_eq!(user.email, "john@example.com"); // Unchanged
        assert_eq!(user.age, Some(30));
        assert!(!user.is_active);
        assert!(user.updated_at > original_updated_at);
    }

    #[test]
    fn test_model_traits() {
        let user = User::new("Test".to_string(), "test@example.com".to_string());
        assert_eq!(User::table_name(), "users");
        assert!(user.primary_key().is_some());
        assert!(User::uses_timestamps());
        assert!(user.created_at().is_some());
        assert!(user.updated_at().is_some());
    }
}