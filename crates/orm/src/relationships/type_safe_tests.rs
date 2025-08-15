//! Comprehensive tests for type-safe relationship loading system

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::error::{ModelError, ModelResult};
use crate::model::Model;
use super::containers::*;
use super::hydration::*;
use super::inference::*;
use super::metadata::*;
use super::type_safe_eager_loading::*;

// Test models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Option<i64>,
    pub name: String,
    pub email: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    
    // Type-safe relationships
    #[serde(skip)]
    pub posts: HasMany<Post>,
    
    #[serde(skip)]
    pub profile: HasOne<Profile>,
    
    #[serde(skip)]
    pub roles: ManyToMany<Role>,
}

impl Model for User {
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
        if let Some(created_at) = self.created_at {
            fields.insert("created_at".to_string(), serde_json::json!(created_at));
        }
        fields
    }
    
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id").ok(),
            name: row.try_get("name").unwrap_or_default(),
            email: row.try_get("email").unwrap_or_default(),
            created_at: row.try_get("created_at").ok(),
            posts: HasMany::new(RelationshipMetadata::new(
                RelationshipType::HasMany,
                "posts".to_string(),
                "posts".to_string(),
                "Post".to_string(),
                ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
            )),
            profile: HasOne::new(RelationshipMetadata::new(
                RelationshipType::HasOne,
                "profile".to_string(),
                "profiles".to_string(),
                "Profile".to_string(),
                ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
            )),
            roles: ManyToMany::new(RelationshipMetadata::new(
                RelationshipType::ManyToMany,
                "roles".to_string(),
                "roles".to_string(),
                "Role".to_string(),
                ForeignKeyConfig::simple("user_id".to_string(), "user_roles".to_string()),
            ).with_pivot(PivotConfig::new(
                "user_roles".to_string(),
                "user_id".to_string(),
                "role_id".to_string(),
            ).with_additional_columns(vec!["created_at".to_string()]))),
        })
    }
}

impl InferableModel for User {
    fn relationship_hints() -> Vec<RelationshipHint> {
        vec![
            RelationshipHint {
                field_name: "posts".to_string(),
                relationship_type: RelationshipType::HasMany,
                related_model: "Post".to_string(),
                custom_foreign_key: Some("user_id".to_string()),
                eager_load: false,
            },
            RelationshipHint {
                field_name: "profile".to_string(),
                relationship_type: RelationshipType::HasOne,
                related_model: "Profile".to_string(),
                custom_foreign_key: Some("user_id".to_string()),
                eager_load: true,
            },
            RelationshipHint {
                field_name: "roles".to_string(),
                relationship_type: RelationshipType::ManyToMany,
                related_model: "Role".to_string(),
                custom_foreign_key: None,
                eager_load: false,
            },
        ]
    }
    
    fn foreign_key_convention() -> ForeignKeyConvention {
        ForeignKeyConvention::Underscore
    }
    
    fn table_naming_convention() -> TableNamingConvention {
        TableNamingConvention::Plural
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Post {
    pub id: Option<i64>,
    pub title: String,
    pub content: String,
    pub user_id: Option<i64>,
    pub published: bool,
    
    // Type-safe relationships
    #[serde(skip)]
    pub user: BelongsTo<User>,
    
    #[serde(skip)]
    pub comments: HasMany<Comment>,
    
    #[serde(skip)]
    pub tags: ManyToMany<Tag>,
}

impl Model for Post {
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
        fields.insert("published".to_string(), serde_json::json!(self.published));
        fields
    }
    
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id").ok(),
            title: row.try_get("title").unwrap_or_default(),
            content: row.try_get("content").unwrap_or_default(),
            user_id: row.try_get("user_id").ok(),
            published: row.try_get("published").unwrap_or(false),
            user: BelongsTo::new(RelationshipMetadata::new(
                RelationshipType::BelongsTo,
                "user".to_string(),
                "users".to_string(),
                "User".to_string(),
                ForeignKeyConfig::simple("user_id".to_string(), "users".to_string()),
            )),
            comments: HasMany::new(RelationshipMetadata::new(
                RelationshipType::HasMany,
                "comments".to_string(),
                "comments".to_string(),
                "Comment".to_string(),
                ForeignKeyConfig::simple("post_id".to_string(), "comments".to_string()),
            )),
            tags: ManyToMany::new(RelationshipMetadata::new(
                RelationshipType::ManyToMany,
                "tags".to_string(),
                "tags".to_string(),
                "Tag".to_string(),
                ForeignKeyConfig::simple("post_id".to_string(), "post_tags".to_string()),
            ).with_pivot(PivotConfig::new(
                "post_tags".to_string(),
                "post_id".to_string(),
                "tag_id".to_string(),
            ))),
        })
    }
}

impl InferableModel for Post {
    fn relationship_hints() -> Vec<RelationshipHint> {
        vec![
            RelationshipHint {
                field_name: "user".to_string(),
                relationship_type: RelationshipType::BelongsTo,
                related_model: "User".to_string(),
                custom_foreign_key: Some("user_id".to_string()),
                eager_load: true,
            },
            RelationshipHint {
                field_name: "comments".to_string(),
                relationship_type: RelationshipType::HasMany,
                related_model: "Comment".to_string(),
                custom_foreign_key: Some("post_id".to_string()),
                eager_load: false,
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub id: Option<i64>,
    pub user_id: Option<i64>,
    pub bio: String,
    pub avatar_url: Option<String>,
}

impl Model for Profile {
    type PrimaryKey = i64;
    
    fn table_name() -> &'static str {
        "profiles"
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
        fields.insert("user_id".to_string(), serde_json::json!(self.user_id));
        fields.insert("bio".to_string(), serde_json::Value::String(self.bio.clone()));
        fields.insert("avatar_url".to_string(), serde_json::json!(self.avatar_url));
        fields
    }
    
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id").ok(),
            user_id: row.try_get("user_id").ok(),
            bio: row.try_get("bio").unwrap_or_default(),
            avatar_url: row.try_get("avatar_url").ok(),
        })
    }
}

impl InferableModel for Profile {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Comment {
    pub id: Option<i64>,
    pub post_id: Option<i64>,
    pub author_name: String,
    pub content: String,
    pub approved: bool,
}

impl Model for Comment {
    type PrimaryKey = i64;
    
    fn table_name() -> &'static str {
        "comments"
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
        fields.insert("post_id".to_string(), serde_json::json!(self.post_id));
        fields.insert("author_name".to_string(), serde_json::Value::String(self.author_name.clone()));
        fields.insert("content".to_string(), serde_json::Value::String(self.content.clone()));
        fields.insert("approved".to_string(), serde_json::json!(self.approved));
        fields
    }
    
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id").ok(),
            post_id: row.try_get("post_id").ok(),
            author_name: row.try_get("author_name").unwrap_or_default(),
            content: row.try_get("content").unwrap_or_default(),
            approved: row.try_get("approved").unwrap_or(false),
        })
    }
}

impl InferableModel for Comment {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Role {
    pub id: Option<i64>,
    pub name: String,
    pub permissions: String,
}

impl Model for Role {
    type PrimaryKey = i64;
    
    fn table_name() -> &'static str {
        "roles"
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
        fields.insert("permissions".to_string(), serde_json::Value::String(self.permissions.clone()));
        fields
    }
    
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id").ok(),
            name: row.try_get("name").unwrap_or_default(),
            permissions: row.try_get("permissions").unwrap_or_default(),
        })
    }
}

impl InferableModel for Role {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: Option<i64>,
    pub name: String,
    pub slug: String,
}

impl Model for Tag {
    type PrimaryKey = i64;
    
    fn table_name() -> &'static str {
        "tags"
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
        fields.insert("slug".to_string(), serde_json::Value::String(self.slug.clone()));
        fields
    }
    
    fn from_row(row: &sqlx::postgres::PgRow) -> ModelResult<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id").ok(),
            name: row.try_get("name").unwrap_or_default(),
            slug: row.try_get("slug").unwrap_or_default(),
        })
    }
}

impl InferableModel for Tag {}

// Test modules
#[cfg(test)]
mod container_tests {
    use super::*;
    
    #[test]
    fn test_has_one_relationship() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasOne,
            "profile".to_string(),
            "profiles".to_string(),
            "Profile".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
        );
        
        let mut profile_rel: HasOne<Profile> = TypeSafeRelationship::new(metadata);
        
        // Initially not loaded
        assert!(!profile_rel.is_loaded());
        assert!(profile_rel.get_typed().is_none());
        
        // Load a profile
        let profile = Profile {
            id: Some(1),
            user_id: Some(1),
            bio: "Test bio".to_string(),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        };
        
        profile_rel.set_loaded(Some(profile.clone()));
        
        assert!(profile_rel.is_loaded());
        if let Some(Some(loaded_profile)) = profile_rel.get_typed() {
            assert_eq!(loaded_profile.bio, "Test bio");
            assert_eq!(loaded_profile.id, Some(1));
        } else {
            panic!("Profile should be loaded");
        }
        
        // Test try_get
        let result = profile_rel.try_get();
        assert!(result.is_ok());
        
        let profile_option = result.unwrap();
        if let Some(profile_ref) = profile_option {
            assert_eq!(profile_ref.user_id, Some(1));
        } else {
            panic!("Profile should be None in this test");
        }
    }
    
    #[test]
    fn test_has_many_relationship() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasMany,
            "posts".to_string(),
            "posts".to_string(),
            "Post".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
        );
        
        let mut posts_rel: HasMany<Post> = TypeSafeRelationship::new(metadata);
        
        let posts = vec![
            Post {
                id: Some(1),
                title: "First Post".to_string(),
                content: "Content 1".to_string(),
                user_id: Some(1),
                published: true,
                user: BelongsTo::new(RelationshipMetadata::new(
                    RelationshipType::BelongsTo,
                    "user".to_string(),
                    "users".to_string(),
                    "User".to_string(),
                    ForeignKeyConfig::simple("user_id".to_string(), "users".to_string()),
                )),
                comments: HasMany::new(RelationshipMetadata::new(
                    RelationshipType::HasMany,
                    "comments".to_string(),
                    "comments".to_string(),
                    "Comment".to_string(),
                    ForeignKeyConfig::simple("post_id".to_string(), "comments".to_string()),
                )),
                tags: ManyToMany::new(RelationshipMetadata::new_with_pivot(
                    RelationshipType::ManyToMany,
                    "tags".to_string(),
                    "tags".to_string(),
                    "Tag".to_string(),
                    ForeignKeyConfig::simple("post_id".to_string(), "post_tags".to_string()),
                    PivotConfig {
                        table: "post_tags".to_string(),
                        local_key: "post_id".to_string(),
                        foreign_key: "tag_id".to_string(),
                        additional_columns: Vec::new(),
                        with_timestamps: false,
                    },
                )),
            },
            Post {
                id: Some(2),
                title: "Second Post".to_string(),
                content: "Content 2".to_string(),
                user_id: Some(1),
                published: false,
                user: BelongsTo::new(RelationshipMetadata::new(
                    RelationshipType::BelongsTo,
                    "user".to_string(),
                    "users".to_string(),
                    "User".to_string(),
                    ForeignKeyConfig::simple("user_id".to_string(), "users".to_string()),
                )),
                comments: HasMany::new(RelationshipMetadata::new(
                    RelationshipType::HasMany,
                    "comments".to_string(),
                    "comments".to_string(),
                    "Comment".to_string(),
                    ForeignKeyConfig::simple("post_id".to_string(), "comments".to_string()),
                )),
                tags: ManyToMany::new(RelationshipMetadata::new_with_pivot(
                    RelationshipType::ManyToMany,
                    "tags".to_string(),
                    "tags".to_string(),
                    "Tag".to_string(),
                    ForeignKeyConfig::simple("post_id".to_string(), "post_tags".to_string()),
                    PivotConfig {
                        table: "post_tags".to_string(),
                        local_key: "post_id".to_string(),
                        foreign_key: "tag_id".to_string(),
                        additional_columns: Vec::new(),
                        with_timestamps: false,
                    },
                )),
            },
        ];
        
        posts_rel.set_loaded(posts.clone());
        
        assert!(posts_rel.is_loaded());
        if let Some(loaded_posts) = posts_rel.get_typed() {
            assert_eq!(loaded_posts.len(), 2);
            assert_eq!(loaded_posts[0].title, "First Post");
            assert_eq!(loaded_posts[1].title, "Second Post");
            assert!(loaded_posts[0].published);
            assert!(!loaded_posts[1].published);
        } else {
            panic!("Posts should be loaded");
        }
    }
    
    #[test]
    fn test_belongs_to_relationship() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::BelongsTo,
            "user".to_string(),
            "users".to_string(),
            "User".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "users".to_string()),
        );
        
        let mut user_rel: BelongsTo<User> = TypeSafeRelationship::new(metadata);
        
        let user = User {
            id: Some(1),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            created_at: None,
            posts: HasMany::new(RelationshipMetadata::new(
                RelationshipType::HasMany,
                "posts".to_string(),
                "posts".to_string(),
                "Post".to_string(),
                ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
            )),
            profile: HasOne::new(RelationshipMetadata::new(
                RelationshipType::HasOne,
                "profile".to_string(),
                "profiles".to_string(),
                "Profile".to_string(),
                ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
            )),
            roles: ManyToMany::new(RelationshipMetadata::new(
                RelationshipType::ManyToMany,
                "roles".to_string(),
                "roles".to_string(),
                "Role".to_string(),
                ForeignKeyConfig::simple("user_id".to_string(), "user_roles".to_string()),
            ).with_pivot(PivotConfig::new(
                "user_roles".to_string(),
                "user_id".to_string(),
                "role_id".to_string(),
            ).with_additional_columns(vec!["created_at".to_string()]))),
        };
        
        user_rel.set_loaded(Some(user.clone()));
        
        assert!(user_rel.is_loaded());
        if let Some(Some(loaded_user)) = user_rel.get_typed() {
            assert_eq!(loaded_user.name, "John Doe");
            assert_eq!(loaded_user.email, "john@example.com");
        } else {
            panic!("User should be loaded");
        }
    }
    
    #[test]
    fn test_relationship_loading_states() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasOne,
            "profile".to_string(),
            "profiles".to_string(),
            "Profile".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
        );
        
        let mut profile_rel: HasOne<Profile> = TypeSafeRelationship::new(metadata);
        
        // Test initial state
        assert!(!profile_rel.is_loaded());
        assert!(matches!(profile_rel.loading_state(), RelationshipLoadingState::NotLoaded));
        
        // Test loading state
        profile_rel.set_loading();
        assert!(!profile_rel.is_loaded());
        assert!(matches!(profile_rel.loading_state(), RelationshipLoadingState::Loading));
        assert!(profile_rel.try_get().is_err());
        
        // Test loaded state
        let profile = Profile {
            id: Some(1),
            user_id: Some(1),
            bio: "Test bio".to_string(),
            avatar_url: None,
        };
        profile_rel.set_loaded(Some(profile));
        
        assert!(profile_rel.is_loaded());
        assert!(matches!(profile_rel.loading_state(), RelationshipLoadingState::Loaded));
        assert!(profile_rel.try_get().is_ok());
        
        // Test failed state
        profile_rel.set_failed("Database connection lost".to_string());
        assert!(!profile_rel.is_loaded());
        assert!(matches!(profile_rel.loading_state(), RelationshipLoadingState::Failed(_)));
        assert!(profile_rel.try_get().is_err());
        
        // Test reset
        profile_rel.reset();
        assert!(!profile_rel.is_loaded());
        assert!(matches!(profile_rel.loading_state(), RelationshipLoadingState::NotLoaded));
    }
    
    #[test]
    fn test_polymorphic_relationships() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::MorphOne,
            "commentable".to_string(),
            "comments".to_string(),
            "Comment".to_string(),
            ForeignKeyConfig::simple("commentable_id".to_string(), "comments".to_string()),
        ).with_polymorphic(PolymorphicConfig::new(
            "commentable".to_string(),
            "commentable_type".to_string(),
            "commentable_id".to_string(),
        ));
        
        let mut morph_rel: MorphOne<Comment> = MorphOne::new(metadata);
        
        // Set polymorphic info
        morph_rel.set_morph_info("Post".to_string(), "123".to_string());
        
        assert_eq!(morph_rel.morph_type(), Some("Post"));
        assert_eq!(morph_rel.morph_id(), Some("123"));
        
        // Load comment
        let comment = Comment {
            id: Some(1),
            post_id: Some(123),
            author_name: "Test Author".to_string(),
            content: "Test comment".to_string(),
            approved: true,
        };
        
        morph_rel.set_loaded(Some(comment));
        
        assert!(morph_rel.is_loaded());
        if let Some(Some(loaded_comment)) = morph_rel.get() {
            assert_eq!(loaded_comment.author_name, "Test Author");
            assert_eq!(loaded_comment.content, "Test comment");
            assert!(loaded_comment.approved);
        } else {
            panic!("Comment should be loaded");
        }
    }
    
    #[test]
    fn test_relationship_mapping() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasOne,
            "profile".to_string(),
            "profiles".to_string(),
            "Profile".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
        );
        
        let mut profile_rel: TypeSafeRelationship<Option<Profile>> = TypeSafeRelationship::new(metadata);
        
        let profile = Profile {
            id: Some(1),
            user_id: Some(1),
            bio: "Test bio".to_string(),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        };
        
        profile_rel.set_loaded(Some(profile));
        
        // Map to extract just the bio
        let bio_rel = profile_rel.map(|profile_opt| {
            profile_opt.map(|p| p.bio)
        });
        
        assert!(bio_rel.is_loaded());
        if let Some(Some(bio)) = bio_rel.get_typed() {
            assert_eq!(bio, "Test bio");
        } else {
            panic!("Bio should be loaded");
        }
    }
    
    #[test]
    fn test_type_safe_utils() {
        let metadata1 = RelationshipMetadata::new(
            RelationshipType::HasOne,
            "profile1".to_string(),
            "profiles".to_string(),
            "Profile".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
        );
        
        let metadata2 = RelationshipMetadata::new(
            RelationshipType::HasOne,
            "profile2".to_string(),
            "profiles".to_string(),
            "Profile".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
        );
        
        let mut rel1: TypeSafeRelationship<Option<Profile>> = TypeSafeRelationship::new(metadata1);
        let mut rel2: TypeSafeRelationship<Option<Profile>> = TypeSafeRelationship::new(metadata2);
        
        // Load one successfully
        let profile1 = Profile {
            id: Some(1),
            user_id: Some(1),
            bio: "Profile 1".to_string(),
            avatar_url: None,
        };
        rel1.set_loaded(Some(profile1));
        
        // Fail the other
        rel2.set_failed("Connection timeout".to_string());
        
        let relationships = vec![rel1, rel2];
        
        // Test utility functions
        assert_eq!(type_safe_utils::count_loaded(&relationships), 1);
        assert!(!type_safe_utils::all_loaded(&relationships));
        
        let failed = type_safe_utils::get_failed_relationships(&relationships);
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].0, "profile2");
        assert_eq!(failed[0].1, "Connection timeout");
        
        let loaded_data = type_safe_utils::extract_loaded_data(&relationships);
        assert_eq!(loaded_data.len(), 1);
    }
}

#[cfg(test)]
mod inference_tests {
    use super::*;
    
    #[test]
    fn test_relationship_inference_engine() {
        let mut engine = RelationshipInferenceEngine::<User>::new();
        
        // Test inference of all relationships based on hints
        let relationships = engine.infer_all_relationships().unwrap();
        
        assert_eq!(relationships.len(), 3);
        
        // Check posts relationship
        let posts_rel = relationships.iter().find(|r| r.name == "posts").unwrap();
        assert_eq!(posts_rel.relationship_type, RelationshipType::HasMany);
        assert_eq!(posts_rel.related_model, "Post");
        assert!(!posts_rel.eager_load);
        
        // Check profile relationship
        let profile_rel = relationships.iter().find(|r| r.name == "profile").unwrap();
        assert_eq!(profile_rel.relationship_type, RelationshipType::HasOne);
        assert_eq!(profile_rel.related_model, "Profile");
        assert!(profile_rel.eager_load);
        
        // Check roles relationship
        let roles_rel = relationships.iter().find(|r| r.name == "roles").unwrap();
        assert_eq!(roles_rel.relationship_type, RelationshipType::ManyToMany);
        assert_eq!(roles_rel.related_model, "Role");
    }
    
    #[test]
    fn test_individual_relationship_inference() {
        let mut engine = RelationshipInferenceEngine::<User>::new();
        
        let posts_metadata = engine.infer_relationship::<Post>("posts", RelationshipType::HasMany).unwrap();
        
        assert_eq!(posts_metadata.name, "posts");
        assert_eq!(posts_metadata.relationship_type, RelationshipType::HasMany);
        assert_eq!(posts_metadata.related_table, "posts");
        assert_eq!(posts_metadata.related_model, "Post");
    }
    
    #[test]
    fn test_foreign_key_naming_conventions() {
        let engine = RelationshipInferenceEngine::<User>::new();
        
        // Test underscore convention (default)
        let fk = engine.infer_foreign_key_name("user").unwrap();
        assert_eq!(fk, "user_id");
        
        let fk = engine.infer_foreign_key_name("posts").unwrap();
        assert_eq!(fk, "post_id");
        
        let fk = engine.infer_foreign_key_name("categories").unwrap();
        assert_eq!(fk, "category_id");
    }
    
    #[test]
    fn test_table_name_inference() {
        let engine = RelationshipInferenceEngine::<User>::new();
        
        assert_eq!(engine.infer_table_name("Post"), "posts");
        assert_eq!(engine.infer_table_name("User"), "users");
        assert_eq!(engine.infer_table_name("Category"), "categories");
        assert_eq!(engine.infer_table_name("Box"), "boxes");
    }
    
    #[test]
    fn test_pluralization() {
        let engine = RelationshipInferenceEngine::<User>::new();
        
        // Regular plurals
        assert_eq!(engine.pluralize_name("post"), "posts");
        assert_eq!(engine.pluralize_name("user"), "users");
        
        // -y endings
        assert_eq!(engine.pluralize_name("category"), "categories");
        assert_eq!(engine.pluralize_name("company"), "companies");
        
        // Special cases
        assert_eq!(engine.pluralize_name("box"), "boxes");
        assert_eq!(engine.pluralize_name("class"), "classes");
        assert_eq!(engine.pluralize_name("brush"), "brushes");
        
        // -ay, -ey endings (don't change y to ies)
        assert_eq!(engine.pluralize_name("day"), "days");
        assert_eq!(engine.pluralize_name("key"), "keys");
    }
    
    #[test]
    fn test_singularization() {
        let engine = RelationshipInferenceEngine::<User>::new();
        
        // Regular singulars
        assert_eq!(engine.singularize_table_name("posts"), "post");
        assert_eq!(engine.singularize_table_name("users"), "user");
        
        // -ies endings
        assert_eq!(engine.singularize_table_name("categories"), "category");
        assert_eq!(engine.singularize_table_name("companies"), "company");
        
        // Special cases
        assert_eq!(engine.singularize_table_name("boxes"), "box");
        assert_eq!(engine.singularize_table_name("classes"), "class");
        assert_eq!(engine.singularize_table_name("brushes"), "brush");
    }
    
    #[test]
    fn test_case_conversions() {
        let engine = RelationshipInferenceEngine::<User>::new();
        
        // camelCase conversion
        assert_eq!(engine.to_camel_case("user_id"), "userId");
        assert_eq!(engine.to_camel_case("created_at"), "createdAt");
        assert_eq!(engine.to_camel_case("user"), "user");
        assert_eq!(engine.to_camel_case("some_long_field_name"), "someLongFieldName");
        
        // PascalCase conversion
        assert_eq!(engine.to_pascal_case("user_id"), "UserId");
        assert_eq!(engine.to_pascal_case("created_at"), "CreatedAt");
        assert_eq!(engine.to_pascal_case("user"), "User");
        assert_eq!(engine.to_pascal_case("some_long_field_name"), "SomeLongFieldName");
    }
    
    #[test]
    fn test_type_inference_helper() {
        // Test type name inference
        assert_eq!(
            TypeInferenceHelper::infer_from_type_name("Option<Profile>"),
            Some(RelationshipType::HasOne)
        );
        assert_eq!(
            TypeInferenceHelper::infer_from_type_name("Vec<Post>"),
            Some(RelationshipType::HasMany)
        );
        assert_eq!(
            TypeInferenceHelper::infer_from_type_name("MorphOne<Comment>"),
            Some(RelationshipType::MorphOne)
        );
        assert_eq!(
            TypeInferenceHelper::infer_from_type_name("MorphMany<Tag>"),
            Some(RelationshipType::MorphMany)
        );
        
        // Test field name inference
        assert_eq!(
            TypeInferenceHelper::infer_from_field_name("user_id"),
            Some(RelationshipType::BelongsTo)
        );
        assert_eq!(
            TypeInferenceHelper::infer_from_field_name("role_ids"),
            Some(RelationshipType::ManyToMany)
        );
        assert_eq!(
            TypeInferenceHelper::infer_from_field_name("commentable"),
            Some(RelationshipType::MorphTo)
        );
        
        // Test relationship type suggestion
        let rt = TypeInferenceHelper::suggest_relationship_type(
            "posts",
            "Vec<Post>",
            true,
            false,
        );
        assert_eq!(rt, RelationshipType::HasMany);
        
        let rt = TypeInferenceHelper::suggest_relationship_type(
            "user_id",
            "i64",
            false,
            false,
        );
        assert_eq!(rt, RelationshipType::BelongsTo);
        
        let rt = TypeInferenceHelper::suggest_relationship_type(
            "profile",
            "Option<Profile>",
            false,
            true,
        );
        assert_eq!(rt, RelationshipType::HasOne);
    }
}

#[cfg(test)]
mod eager_loading_tests {
    use super::*;
    
    #[test]
    fn test_type_safe_eager_loader_creation() {
        let loader = TypeSafeEagerLoader::<User>::new();
        
        assert!(loader.loaded_relations().is_empty());
        assert!(!loader.is_loaded("posts"));
    }
    
    #[test]
    fn test_eager_load_spec_creation() -> ModelResult<()> {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasMany,
            "posts".to_string(),
            "posts".to_string(),
            "Post".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
        );
        
        let spec = TypeSafeEagerLoadSpec::<User, Post>::new("posts".to_string(), metadata);
        
        assert_eq!(spec.relation, "posts");
        assert_eq!(spec.relationship_type, RelationshipType::HasMany);
        assert!(spec.use_type_safe);
        
        Ok(())
    }
    
    #[test]
    fn test_relationship_data_variants() {
        // Test single relationship data
        let single_data = TypeSafeRelationshipData::Single(Some(serde_json::json!({
            "id": 1,
            "bio": "Test bio"
        })));
        
        match single_data {
            TypeSafeRelationshipData::Single(Some(_)) => assert!(true),
            _ => panic!("Expected single relationship data"),
        }
        
        // Test collection relationship data
        let collection_data = TypeSafeRelationshipData::Collection(vec![
            serde_json::json!({"id": 1, "title": "Post 1"}),
            serde_json::json!({"id": 2, "title": "Post 2"}),
        ]);
        
        match collection_data {
            TypeSafeRelationshipData::Collection(ref items) => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Expected collection relationship data"),
        }
        
        // Test polymorphic relationship data
        let polymorphic_data = TypeSafeRelationshipData::Polymorphic {
            data: Some(serde_json::json!({"id": 1, "content": "Test comment"})),
            morph_type: Some("Post".to_string()),
            morph_id: Some("123".to_string()),
        };
        
        match polymorphic_data {
            TypeSafeRelationshipData::Polymorphic { morph_type: Some(ref t), morph_id: Some(ref id), .. } => {
                assert_eq!(t, "Post");
                assert_eq!(id, "123");
            }
            _ => panic!("Expected polymorphic relationship data"),
        }
    }
}