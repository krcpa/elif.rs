//! Factory system for test data generation
//!
//! Provides a powerful and type-safe factory system for generating
//! test data with support for relationships, custom attributes,
//! and database persistence.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{Value as JsonValue, json};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use service_builder::builder;
use crate::{TestResult, database::TestDatabase};

/// Factory trait for creating test data
#[async_trait]
pub trait Factory<T: Send>: Send + Sync {
    /// Create a single instance
    async fn create(&self) -> TestResult<T>;
    
    /// Create multiple instances
    async fn create_many(&self, count: usize) -> TestResult<Vec<T>> {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            results.push(self.create().await?);
        }
        Ok(results)
    }
    
    /// Build the data without persisting to database
    fn build(&self) -> TestResult<T>;
    
    /// Build multiple instances without persisting
    fn build_many(&self, count: usize) -> TestResult<Vec<T>> {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            results.push(self.build()?);
        }
        Ok(results)
    }
}

/// Configuration for Factory builder
#[derive(Clone)]
#[builder]
pub struct FactoryBuilderConfig {
    #[builder(default)]
    pub attributes: HashMap<String, JsonValue>,
    
    #[builder(optional)]
    pub database: Option<Arc<TestDatabase>>,
}

impl std::fmt::Debug for FactoryBuilderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FactoryBuilderConfig")
            .field("attributes_count", &self.attributes.len())
            .field("has_database", &self.database.is_some())
            .finish()
    }
}

impl FactoryBuilderConfig {
    /// Build a FactoryBuilder from the builder config
    pub fn build_factory<T>(self) -> FactoryBuilder<T> {
        FactoryBuilder {
            builder_config: self,
            _phantom: std::marker::PhantomData,
        }
    }
}

// Add convenience methods to the generated builder
impl FactoryBuilderConfigBuilder {
    /// Set an attribute value
    pub fn with_attr<V: serde::Serialize>(self, key: &str, value: V) -> Self {
        let mut attributes = self.attributes.clone().unwrap_or_default();
        if let Ok(json_value) = serde_json::to_value(value) {
            attributes.insert(key.to_string(), json_value);
        }
        self.attributes(attributes)
    }
    
    /// Set multiple attributes
    pub fn with_attrs(self, new_attributes: HashMap<String, JsonValue>) -> Self {
        let mut attributes = self.attributes.clone().unwrap_or_default();
        attributes.extend(new_attributes);
        self.attributes(attributes)
    }
    
    /// Add relationship data
    pub fn with_relation_data(self, name: &str, data: JsonValue) -> Self {
        let mut attributes = self.attributes.clone().unwrap_or_default();
        attributes.insert(format!("{}_data", name), data);
        self.attributes(attributes)
    }
    
    pub fn build_config(self) -> FactoryBuilderConfig {
        self.build_with_defaults().unwrap()
    }
}

/// Factory builder for fluent API
#[derive(Clone)]
pub struct FactoryBuilder<T> {
    builder_config: FactoryBuilderConfig,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> FactoryBuilder<T> {
    /// Create a new factory builder
    pub fn new() -> Self {
        Self {
            builder_config: FactoryBuilderConfig::builder().build_config(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Set an attribute value
    pub fn with<V: serde::Serialize>(self, key: &str, value: V) -> Self {
        let config_builder = FactoryBuilderConfig::builder()
            .attributes(self.builder_config.attributes.clone())
            .database(self.builder_config.database.clone())
            .with_attr(key, value)
            .build_config();
        
        Self {
            builder_config: config_builder,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Set multiple attributes
    pub fn with_attributes(self, attributes: HashMap<String, JsonValue>) -> Self {
        let config_builder = FactoryBuilderConfig::builder()
            .attributes(self.builder_config.attributes.clone())
            .database(self.builder_config.database.clone())
            .with_attrs(attributes)
            .build_config();
        
        Self {
            builder_config: config_builder,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Set database connection for persistence
    pub fn with_database(self, database: Arc<TestDatabase>) -> Self {
        let config_builder = FactoryBuilderConfig::builder()
            .attributes(self.builder_config.attributes.clone())
            .database(Some(database))
            .build_config();
        
        Self {
            builder_config: config_builder,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Add a relationship (simplified version)
    pub fn with_relationship_data(self, name: &str, data: JsonValue) -> Self {
        let config_builder = FactoryBuilderConfig::builder()
            .attributes(self.builder_config.attributes.clone())
            .database(self.builder_config.database.clone())
            .with_relation_data(name, data)
            .build_config();
        
        Self {
            builder_config: config_builder,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Get the current attributes
    pub fn attributes(&self) -> &HashMap<String, JsonValue> {
        &self.builder_config.attributes
    }
}

impl<T> Default for FactoryBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}


/// Trait for models that have an ID
pub trait HasId {
    fn id(&self) -> JsonValue;
}

/// Common factory implementations

/// User factory
#[derive(Clone)]
pub struct UserFactory {
    builder: FactoryBuilder<User>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl HasId for User {
    fn id(&self) -> JsonValue {
        json!(self.id)
    }
}

impl UserFactory {
    pub fn new() -> Self {
        let builder = FactoryBuilder::new();
            
        Self { builder }
    }
    
    /// Create an admin user
    pub fn admin(self) -> Self {
        let mut new_self = self;
        new_self.builder = new_self.builder.with("role", "admin");
        new_self
    }
    
    /// Set custom name
    pub fn named(self, name: &str) -> Self {
        let mut new_self = self;
        new_self.builder = new_self.builder.with("name", name);
        new_self
    }
    
    /// Set custom email
    pub fn with_email(self, email: &str) -> Self {
        let mut new_self = self;
        new_self.builder = new_self.builder.with("email", email);
        new_self
    }
    
    /// Add posts relationship (simplified)
    pub fn with_posts(self, count: usize) -> Self {
        let mut new_self = self;
        new_self.builder = new_self.builder.with("posts_count", count);
        new_self
    }
}

#[async_trait]
impl Factory<User> for UserFactory {
    async fn create(&self) -> TestResult<User> {
        let user = self.build()?;
        
        // If database is available, persist the user
        if let Some(db) = &self.builder.builder_config.database {
            let insert_sql = r#"
                INSERT INTO users (id, name, email, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5)
            "#;
            
            sqlx::query(insert_sql)
                .bind(&user.id)
                .bind(&user.name)
                .bind(&user.email)
                .bind(&user.created_at)
                .bind(&user.updated_at)
                .execute(db.pool())
                .await?;
        }
        
        Ok(user)
    }
    
    fn build(&self) -> TestResult<User> {
        let attrs = &self.builder.builder_config.attributes;
        
        // Generate fresh values for each build
        let id = attrs.get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);
        
        let name = attrs.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Test User {}", crate::utils::random_string(None)));
        
        let email = attrs.get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| crate::utils::random_email());
        
        Ok(User {
            id,
            name,
            email,
            created_at: attrs.get("created_at")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            updated_at: attrs.get("updated_at")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        v.as_str()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&Utc))
                    }
                }),
        })
    }
}

impl Default for UserFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Post factory
#[derive(Clone)]
pub struct PostFactory {
    builder: FactoryBuilder<Post>,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl HasId for Post {
    fn id(&self) -> JsonValue {
        json!(self.id)
    }
}

impl PostFactory {
    pub fn new() -> Self {
        let builder = FactoryBuilder::new();
            
        Self { builder }
    }
    
    /// Set custom title
    pub fn with_title(self, title: &str) -> Self {
        let mut new_self = self;
        new_self.builder = new_self.builder.with("title", title);
        new_self
    }
    
    /// Set custom content
    pub fn with_content(self, content: &str) -> Self {
        let mut new_self = self;
        new_self.builder = new_self.builder.with("content", content);
        new_self
    }
    
    /// Set user relationship
    pub fn for_user(self, user_id: Uuid) -> Self {
        let mut new_self = self;
        new_self.builder = new_self.builder.with("user_id", user_id);
        new_self
    }
    
    /// Set user relationship using factory (simplified)
    pub fn with_user(self) -> Self {
        let mut new_self = self;
        // In real implementation, this would create a user and set user_id
        let user_id = Uuid::new_v4();
        new_self.builder = new_self.builder.with("user_id", user_id);
        new_self
    }
}

#[async_trait]
impl Factory<Post> for PostFactory {
    async fn create(&self) -> TestResult<Post> {
        let post = self.build()?;
        
        if let Some(db) = &self.builder.builder_config.database {
            let insert_sql = r#"
                INSERT INTO posts (id, title, content, user_id, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
            "#;
            
            sqlx::query(insert_sql)
                .bind(&post.id)
                .bind(&post.title)
                .bind(&post.content)
                .bind(&post.user_id)
                .bind(&post.created_at)
                .bind(&post.updated_at)
                .execute(db.pool())
                .await?;
        }
        
        Ok(post)
    }
    
    fn build(&self) -> TestResult<Post> {
        let attrs = &self.builder.builder_config.attributes;
        
        // Generate fresh values for each build
        let id = attrs.get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);
        
        let title = attrs.get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Test Post {}", crate::utils::random_string(None)));
        
        let user_id = attrs.get("user_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);
        
        Ok(Post {
            id,
            title,
            content: attrs.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("This is test content for the post.")
                .to_string(),
            user_id,
            created_at: attrs.get("created_at")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            updated_at: attrs.get("updated_at")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        v.as_str()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&Utc))
                    }
                }),
        })
    }
}

impl Default for PostFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Sequence generator for unique values
pub struct Sequence {
    current: std::sync::atomic::AtomicUsize,
}

impl Sequence {
    pub fn new() -> Self {
        Self {
            current: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    pub fn next(&self) -> usize {
        self.current.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
    
    pub fn next_string(&self, prefix: &str) -> String {
        format!("{}{}", prefix, self.next())
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_factory_build() -> TestResult<()> {
        let factory = UserFactory::new();
        let user = factory.build()?;
        
        assert!(!user.name.is_empty());
        assert!(user.email.contains("@"));
        assert!(user.created_at <= Utc::now());
        
        Ok(())
    }
    
    #[test]
    fn test_user_factory_with_custom_attributes() -> TestResult<()> {
        let factory = UserFactory::new()
            .named("John Doe")
            .with_email("john@example.com");
            
        let user = factory.build()?;
        
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
        
        Ok(())
    }
    
    #[test]
    fn test_post_factory_build() -> TestResult<()> {
        let factory = PostFactory::new();
        let post = factory.build()?;
        
        assert!(!post.title.is_empty());
        assert!(!post.content.is_empty());
        assert!(post.created_at <= Utc::now());
        
        Ok(())
    }
    
    #[test]
    fn test_sequence() {
        let seq = Sequence::new();
        
        assert_eq!(seq.next(), 0);
        assert_eq!(seq.next(), 1);
        assert_eq!(seq.next_string("user"), "user2");
    }
    
    #[test]
    fn test_factory_builder() {
        let builder = FactoryBuilder::<User>::new()
            .with("name", "Test User")
            .with("email", "test@example.com");
            
        assert_eq!(builder.attributes().get("name"), Some(&json!("Test User")));
        assert_eq!(builder.attributes().get("email"), Some(&json!("test@example.com")));
    }
    
    #[tokio::test]
    async fn test_factory_create_many() -> TestResult<()> {
        let factory = UserFactory::new();
        let users = factory.build_many(3)?;
        
        assert_eq!(users.len(), 3);
        
        // Ensure all users are unique
        for i in 0..users.len() {
            for j in (i + 1)..users.len() {
                assert_ne!(users[i].id, users[j].id);
            }
        }
        
        Ok(())
    }
}