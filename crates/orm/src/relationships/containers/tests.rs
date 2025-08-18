//! Tests for relationship containers
#![cfg(test)]

use super::*;
use crate::model::Model;
use crate::relationships::metadata::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// Mock model for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestUser {
    id: Option<i64>,
    name: String,
    email: String,
}

impl Model for TestUser {
    type PrimaryKey = i64;
    
    fn table_name() -> &'static str {
        "users"
    }
    
    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        self.id
    }
    
    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = Some(key);
    }
    
    fn to_fields(&self) -> HashMap<String, serde_json::Value> {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), serde_json::json!(self.id));
        fields.insert("name".to_string(), serde_json::Value::String(self.name.clone()));
        fields.insert("email".to_string(), serde_json::Value::String(self.email.clone()));
        fields
    }
    
    fn from_row(row: &sqlx::postgres::PgRow) -> crate::error::ModelResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id").ok(),
            name: row.try_get("name").unwrap_or_default(),
            email: row.try_get("email").unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestPost {
    id: Option<i64>,
    title: String,
    content: String,
    user_id: Option<i64>,
}

impl Model for TestPost {
    type PrimaryKey = i64;
    
    fn table_name() -> &'static str {
        "posts"
    }
    
    fn primary_key(&self) -> Option<Self::PrimaryKey> {
        self.id
    }
    
    fn set_primary_key(&mut self, key: Self::PrimaryKey) {
        self.id = Some(key);
    }
    
    fn to_fields(&self) -> HashMap<String, serde_json::Value> {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), serde_json::json!(self.id));
        fields.insert("title".to_string(), serde_json::Value::String(self.title.clone()));
        fields.insert("content".to_string(), serde_json::Value::String(self.content.clone()));
        fields.insert("user_id".to_string(), serde_json::json!(self.user_id));
        fields
    }
    
    fn from_row(row: &sqlx::postgres::PgRow) -> crate::error::ModelResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id").ok(),
            title: row.try_get("title").unwrap_or_default(),
            content: row.try_get("content").unwrap_or_default(),
            user_id: row.try_get("user_id").ok(),
        })
    }
}

#[test]
fn test_type_safe_relationship_creation() {
    let metadata = RelationshipMetadata::new(
        RelationshipType::HasMany,
        "posts".to_string(),
        "posts".to_string(),
        "TestPost".to_string(),
        ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
    );
    
    let posts_rel: HasMany<TestPost> = TypeSafeRelationship::new(metadata);
    
    assert!(!posts_rel.is_loaded());
    assert_eq!(posts_rel.name(), "posts");
    assert!(posts_rel.get().is_none());
}

#[test]
fn test_relationship_loading_states() {
    let metadata = RelationshipMetadata::new(
        RelationshipType::BelongsTo,
        "user".to_string(),
        "users".to_string(),
        "TestUser".to_string(),
        ForeignKeyConfig::simple("user_id".to_string(), "users".to_string()),
    );
    
    let mut user_rel: BelongsTo<TestUser> = TypeSafeRelationship::new(metadata);
    
    // Initially not loaded
    assert!(!user_rel.is_loaded());
    assert!(matches!(user_rel.loading_state(), RelationshipLoadingState::NotLoaded));
    
    // Set to loading
    user_rel.set_loading();
    assert!(!user_rel.is_loaded());
    assert!(matches!(user_rel.loading_state(), RelationshipLoadingState::Loading));
    
    // Load data
    let test_user = TestUser {
        id: Some(1),
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };
    user_rel.set_loaded(Some(test_user));
    
    assert!(user_rel.is_loaded());
    assert!(matches!(user_rel.loading_state(), RelationshipLoadingState::Loaded));
    assert!(user_rel.get_typed().is_some());
    
    // Test try_get
    assert!(user_rel.try_get().is_ok());
    
    // Set failed
    user_rel.set_failed("Database connection failed".to_string());
    assert!(!user_rel.is_loaded());
    assert!(matches!(user_rel.loading_state(), RelationshipLoadingState::Failed(_)));
    assert!(user_rel.try_get().is_err());
}

#[test]
fn test_has_many_relationship() {
    let metadata = RelationshipMetadata::new(
        RelationshipType::HasMany,
        "posts".to_string(),
        "posts".to_string(),
        "TestPost".to_string(),
        ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
    );
    
    let mut posts_rel: HasMany<TestPost> = TypeSafeRelationship::new(metadata);
    
    let posts = vec![
        TestPost {
            id: Some(1),
            title: "Post 1".to_string(),
            content: "Content 1".to_string(),
            user_id: Some(1),
        },
        TestPost {
            id: Some(2),
            title: "Post 2".to_string(),
            content: "Content 2".to_string(),
            user_id: Some(1),
        },
    ];
    
    posts_rel.set_loaded(posts.clone());
    
    assert!(posts_rel.is_loaded());
    if let Some(loaded_posts) = posts_rel.get_typed() {
        assert_eq!(loaded_posts.len(), 2);
        assert_eq!(loaded_posts[0].title, "Post 1");
        assert_eq!(loaded_posts[1].title, "Post 2");
    } else {
        panic!("Posts should be loaded");
    }
}

#[test]
fn test_polymorphic_relationships() {
    let metadata = RelationshipMetadata::new(
        RelationshipType::MorphOne,
        "commentable".to_string(),
        "comments".to_string(),
        "Comment".to_string(),
        ForeignKeyConfig::simple("commentable_id".to_string(), "comments".to_string()),
    );
    
    let mut morph_rel: MorphOne<TestPost> = MorphOne::new(metadata);
    
    // Set polymorphic info
    morph_rel.set_morph_info("Post".to_string(), "1".to_string());
    
    assert_eq!(morph_rel.morph_type(), Some("Post"));
    assert_eq!(morph_rel.morph_id(), Some("1"));
    
    // Load data
    let post = TestPost {
        id: Some(1),
        title: "Test Post".to_string(),
        content: "Test Content".to_string(),
        user_id: Some(1),
    };
    morph_rel.set_loaded(Some(post));
    
    assert!(morph_rel.is_loaded());
    if let Some(Some(loaded_post)) = morph_rel.get() {
        assert_eq!(loaded_post.title, "Test Post");
    } else {
        panic!("Post should be loaded");
    }
}

#[test]
fn test_relationship_mapping() {
    let metadata = RelationshipMetadata::new(
        RelationshipType::BelongsTo,
        "user".to_string(),
        "users".to_string(),
        "TestUser".to_string(),
        ForeignKeyConfig::simple("user_id".to_string(), "users".to_string()),
    );
    
    let mut user_rel: TypeSafeRelationship<Option<TestUser>> = TypeSafeRelationship::new(metadata);
    
    let test_user = TestUser {
        id: Some(1),
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };
    user_rel.set_loaded(Some(test_user));
    
    // Map to extract just the name
    let name_rel = user_rel.map(|user_opt| {
        user_opt.map(|user| user.name)
    });
    
    assert!(name_rel.is_loaded());
    if let Some(Some(name)) = name_rel.get_typed() {
        assert_eq!(name, "Test User");
    } else {
        panic!("Name should be loaded");
    }
}

#[test]
fn test_type_safe_utils() {
    let metadata1 = RelationshipMetadata::new(
        RelationshipType::HasOne,
        "profile".to_string(),
        "profiles".to_string(),
        "Profile".to_string(),
        ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
    );
    
    let metadata2 = RelationshipMetadata::new(
        RelationshipType::HasMany,
        "posts".to_string(),
        "posts".to_string(),
        "Post".to_string(),
        ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
    );
    
    let mut rel1: TypeSafeRelationship<Option<TestUser>> = TypeSafeRelationship::new(metadata1);
    let mut rel2: TypeSafeRelationship<Option<TestUser>> = TypeSafeRelationship::new(metadata2);
    
    // Load one, fail the other
    rel1.set_loaded(Some(TestUser {
        id: Some(1),
        name: "User 1".to_string(),
        email: "user1@example.com".to_string(),
    }));
    rel2.set_failed("Connection timeout".to_string());
    
    let relationships = vec![rel1, rel2];
    
    // Test utility functions
    assert_eq!(type_safe_utils::count_loaded(&relationships), 1);
    assert!(!type_safe_utils::all_loaded(&relationships));
    
    let failed = type_safe_utils::get_failed_relationships(&relationships);
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].1, "Connection timeout");
}