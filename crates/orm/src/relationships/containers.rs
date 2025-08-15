//! Type-Safe Relationship Containers - Generic relationship types with compile-time safety

use std::marker::PhantomData;
use std::fmt::Debug;
use async_trait::async_trait;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use sqlx::{Pool, Postgres};

use crate::error::{ModelError, ModelResult};
use crate::model::Model;
use super::metadata::RelationshipMetadata;

/// Represents the loading state of a relationship
#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipLoadingState {
    /// Not loaded yet
    NotLoaded,
    /// Currently being loaded
    Loading,
    /// Successfully loaded
    Loaded,
    /// Failed to load with error message
    Failed(String),
}

/// Core trait for relationship data containers
pub trait RelationshipContainer<T>: Debug + Clone + Send + Sync {
    /// Check if the relationship is loaded
    fn is_loaded(&self) -> bool;
    
    /// Get the loading state
    fn loading_state(&self) -> &RelationshipLoadingState;
    
    /// Get the loaded data if available
    fn get(&self) -> Option<&T>;
    
    /// Get the loaded data mutably if available
    fn get_mut(&mut self) -> Option<&mut T>;
    
    /// Set the loaded data
    fn set_loaded(&mut self, data: T);
    
    /// Mark as loading
    fn set_loading(&mut self);
    
    /// Mark as failed with error
    fn set_failed(&mut self, error: String);
    
    /// Reset to not loaded state
    fn reset(&mut self);
    
    /// Take the loaded data, leaving the container empty
    fn take(&mut self) -> Option<T>;
}

/// Generic relationship container with type-safe data storage
#[derive(Debug, Clone, PartialEq)]
pub struct TypeSafeRelationship<T> {
    /// Relationship metadata
    metadata: RelationshipMetadata,
    
    /// Loading state
    loading_state: RelationshipLoadingState,
    
    /// The actual typed data (if loaded)
    data: Option<T>,
    
    /// Whether this relationship should be eagerly loaded by default
    eager_load: bool,
    
    /// Phantom data for type safety
    _phantom: PhantomData<T>,
}

impl<T> TypeSafeRelationship<T>
where
    T: Clone + Debug + Send + Sync,
{
    /// Create a new type-safe relationship
    pub fn new(metadata: RelationshipMetadata) -> Self {
        let eager_load = metadata.eager_load;
        Self {
            metadata,
            loading_state: RelationshipLoadingState::NotLoaded,
            data: None,
            eager_load,
            _phantom: PhantomData,
        }
    }
    
    /// Get the relationship metadata
    pub fn metadata(&self) -> &RelationshipMetadata {
        &self.metadata
    }
    
    /// Get the relationship name
    pub fn name(&self) -> &str {
        &self.metadata.name
    }
    
    /// Check if this relationship should be eagerly loaded
    pub fn is_eager(&self) -> bool {
        self.eager_load
    }
    
    /// Set eager loading flag
    pub fn set_eager(&mut self, eager: bool) {
        self.eager_load = eager;
    }
    
    /// Get a reference to the data with compile-time type safety
    pub fn get_typed(&self) -> Option<&T> {
        if matches!(self.loading_state, RelationshipLoadingState::Loaded) {
            self.data.as_ref()
        } else {
            None
        }
    }
    
    /// Get mutable reference to the data with compile-time type safety  
    pub fn get_typed_mut(&mut self) -> Option<&mut T> {
        if matches!(self.loading_state, RelationshipLoadingState::Loaded) {
            self.data.as_mut()
        } else {
            None
        }
    }
    
    /// Unwrap the data, panicking if not loaded
    pub fn unwrap(&self) -> &T {
        self.get_typed().expect("Relationship not loaded")
    }
    
    /// Try to get the data, returning an error if not loaded
    pub fn try_get(&self) -> ModelResult<&T> {
        match &self.loading_state {
            RelationshipLoadingState::Loaded => {
                self.data.as_ref()
                    .ok_or_else(|| ModelError::Configuration("Relationship marked as loaded but no data".to_string()))
            }
            RelationshipLoadingState::NotLoaded => {
                Err(ModelError::Configuration(format!("Relationship '{}' not loaded", self.name())))
            }
            RelationshipLoadingState::Loading => {
                Err(ModelError::Configuration(format!("Relationship '{}' is currently loading", self.name())))
            }
            RelationshipLoadingState::Failed(error) => {
                Err(ModelError::Configuration(format!("Relationship '{}' failed to load: {}", self.name(), error)))
            }
        }
    }
    
    /// Map the loaded data to a different type
    pub fn map<U, F>(self, f: F) -> TypeSafeRelationship<U>
    where
        U: Clone + Debug + Send + Sync,
        F: FnOnce(T) -> U,
    {
        let mut new_rel = TypeSafeRelationship::new(self.metadata);
        new_rel.loading_state = self.loading_state.clone();
        new_rel.eager_load = self.eager_load;
        
        if let Some(data) = self.data {
            new_rel.data = Some(f(data));
        }
        
        new_rel
    }
    
    /// Apply a function if the relationship is loaded
    pub fn if_loaded<F>(&self, f: F) -> Option<()>
    where
        F: FnOnce(&T),
    {
        if let Some(data) = self.get_typed() {
            f(data);
            Some(())
        } else {
            None
        }
    }
}

impl<T> RelationshipContainer<T> for TypeSafeRelationship<T>
where
    T: Clone + Debug + Send + Sync,
{
    fn is_loaded(&self) -> bool {
        matches!(self.loading_state, RelationshipLoadingState::Loaded)
    }
    
    fn loading_state(&self) -> &RelationshipLoadingState {
        &self.loading_state
    }
    
    fn get(&self) -> Option<&T> {
        self.get_typed()
    }
    
    fn get_mut(&mut self) -> Option<&mut T> {
        self.get_typed_mut()
    }
    
    fn set_loaded(&mut self, data: T) {
        self.data = Some(data);
        self.loading_state = RelationshipLoadingState::Loaded;
    }
    
    fn set_loading(&mut self) {
        self.loading_state = RelationshipLoadingState::Loading;
    }
    
    fn set_failed(&mut self, error: String) {
        self.loading_state = RelationshipLoadingState::Failed(error);
        self.data = None;
    }
    
    fn reset(&mut self) {
        self.loading_state = RelationshipLoadingState::NotLoaded;
        self.data = None;
    }
    
    fn take(&mut self) -> Option<T> {
        if matches!(self.loading_state, RelationshipLoadingState::Loaded) {
            self.reset();
            self.data.take()
        } else {
            None
        }
    }
}

impl<T> Default for TypeSafeRelationship<T>
where
    T: Clone + Debug + Send + Sync,
{
    fn default() -> Self {
        Self {
            metadata: RelationshipMetadata::default(),
            loading_state: RelationshipLoadingState::NotLoaded,
            data: None,
            eager_load: false,
            _phantom: PhantomData,
        }
    }
}

/// Specialized relationship types with compile-time guarantees

/// HasOne relationship - holds Option<T> for optional single related model
pub type HasOne<T> = TypeSafeRelationship<Option<T>>;

/// HasMany relationship - holds Vec<T> for collection of related models  
pub type HasMany<T> = TypeSafeRelationship<Vec<T>>;

/// BelongsTo relationship - holds Option<T> for optional parent model
pub type BelongsTo<T> = TypeSafeRelationship<Option<T>>;

/// ManyToMany relationship - holds Vec<T> for many-to-many collection
pub type ManyToMany<T> = TypeSafeRelationship<Vec<T>>;

/// Polymorphic relationships with additional type information

/// MorphOne relationship - polymorphic one-to-one with type info
#[derive(Debug, Clone)]
pub struct MorphOne<T>
where
    T: Clone + Debug + Send + Sync,
{
    /// Core relationship container
    relationship: TypeSafeRelationship<Option<T>>,
    
    /// The morphable type (e.g., "Post", "User")
    morph_type: Option<String>,
    
    /// The morphable ID
    morph_id: Option<String>,
}

impl<T> MorphOne<T>
where
    T: Clone + Debug + Send + Sync,
{
    pub fn new(metadata: RelationshipMetadata) -> Self {
        Self {
            relationship: TypeSafeRelationship::new(metadata),
            morph_type: None,
            morph_id: None,
        }
    }
    
    pub fn set_morph_info(&mut self, morph_type: String, morph_id: String) {
        self.morph_type = Some(morph_type);
        self.morph_id = Some(morph_id);
    }
    
    pub fn morph_type(&self) -> Option<&str> {
        self.morph_type.as_deref()
    }
    
    pub fn morph_id(&self) -> Option<&str> {
        self.morph_id.as_deref()
    }
    
    /// Delegate to core relationship
    pub fn get(&self) -> Option<&Option<T>> {
        self.relationship.get_typed()
    }
    
    pub fn is_loaded(&self) -> bool {
        self.relationship.is_loaded()
    }
    
    pub fn set_loaded(&mut self, data: Option<T>) {
        self.relationship.set_loaded(data);
    }
}

/// MorphMany relationship - polymorphic one-to-many with type info
#[derive(Debug, Clone)]
pub struct MorphMany<T>
where
    T: Clone + Debug + Send + Sync,
{
    /// Core relationship container
    relationship: TypeSafeRelationship<Vec<T>>,
    
    /// The morphable type (e.g., "Post", "User")
    morph_type: Option<String>,
}

impl<T> MorphMany<T>
where
    T: Clone + Debug + Send + Sync,
{
    pub fn new(metadata: RelationshipMetadata) -> Self {
        Self {
            relationship: TypeSafeRelationship::new(metadata),
            morph_type: None,
        }
    }
    
    pub fn set_morph_type(&mut self, morph_type: String) {
        self.morph_type = Some(morph_type);
    }
    
    pub fn morph_type(&self) -> Option<&str> {
        self.morph_type.as_deref()
    }
    
    /// Delegate to core relationship
    pub fn get(&self) -> Option<&Vec<T>> {
        self.relationship.get_typed()
    }
    
    pub fn is_loaded(&self) -> bool {
        self.relationship.is_loaded()
    }
    
    pub fn set_loaded(&mut self, data: Vec<T>) {
        self.relationship.set_loaded(data);
    }
}

/// Trait for loading relationships with type safety
#[async_trait]
pub trait TypeSafeRelationshipLoader<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync + DeserializeOwned,
{
    /// Load a single relationship instance
    async fn load_for_instance(
        &self,
        parent: &Parent,
        pool: &Pool<Postgres>,
    ) -> ModelResult<Related>;
    
    /// Load relationship for multiple parents (eager loading)
    async fn load_for_instances(
        &self,
        parents: &mut [Parent],
        pool: &Pool<Postgres>,
    ) -> ModelResult<Vec<Related>>;
    
    /// Load with specific type conversion
    async fn load_typed<T>(
        &self,
        parent: &Parent,
        pool: &Pool<Postgres>,
    ) -> ModelResult<T>
    where
        T: DeserializeOwned + Send + Sync;
}

/// Utility functions for working with type-safe relationships
pub mod type_safe_utils {
    use super::*;
    
    /// Convert a collection of relationships to their loaded data
    pub fn extract_loaded_data<T>(relationships: &[TypeSafeRelationship<T>]) -> Vec<&T>
    where
        T: Clone + Debug + Send + Sync,
    {
        relationships
            .iter()
            .filter_map(|rel| rel.get_typed())
            .collect()
    }
    
    /// Count loaded relationships
    pub fn count_loaded<T>(relationships: &[TypeSafeRelationship<T>]) -> usize
    where
        T: Clone + Debug + Send + Sync,
    {
        relationships
            .iter()
            .filter(|rel| rel.is_loaded())
            .count()
    }
    
    /// Get all failed relationships with their error messages
    pub fn get_failed_relationships<T>(relationships: &[TypeSafeRelationship<T>]) -> Vec<(&str, &str)>
    where
        T: Clone + Debug + Send + Sync,
    {
        relationships
            .iter()
            .filter_map(|rel| {
                if let RelationshipLoadingState::Failed(error) = rel.loading_state() {
                    Some((rel.name(), error.as_str()))
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Bulk set relationships to loading state
    pub fn set_all_loading<T>(relationships: &mut [TypeSafeRelationship<T>])
    where
        T: Clone + Debug + Send + Sync,
    {
        for rel in relationships {
            rel.set_loading();
        }
    }
    
    /// Check if all relationships in a collection are loaded
    pub fn all_loaded<T>(relationships: &[TypeSafeRelationship<T>]) -> bool
    where
        T: Clone + Debug + Send + Sync,
    {
        !relationships.is_empty() && relationships.iter().all(|rel| rel.is_loaded())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::metadata::*;
    use crate::model::Model;
    
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
        
        fn to_fields(&self) -> std::collections::HashMap<String, serde_json::Value> {
            let mut fields = std::collections::HashMap::new();
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
        
        fn to_fields(&self) -> std::collections::HashMap<String, serde_json::Value> {
            let mut fields = std::collections::HashMap::new();
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
}