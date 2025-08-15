//! Type-Safe Eager Loading System - Efficient relationship loading with compile-time safety

use std::collections::HashMap;
use std::marker::PhantomData;
use serde::de::DeserializeOwned;
use sqlx::{Pool, Postgres};

use crate::error::{ModelError, ModelResult};
use crate::model::Model;
use crate::query::QueryBuilder;
use super::containers::*;
use super::hydration::*;
use super::inference::*;
use super::metadata::{RelationshipMetadata, RelationshipType};
use super::constraints::RelationshipConstraintBuilder;

/// Type-safe eager loading specification
#[derive(Debug)]
pub struct TypeSafeEagerLoadSpec<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + DeserializeOwned + Send + Sync,
{
    /// Relationship name
    pub relation: String,
    
    /// Relationship type
    pub relationship_type: RelationshipType,
    
    /// Optional constraints for the relationship query
    pub constraints: Option<RelationshipConstraintBuilder>,
    
    /// The hydrator for this relationship
    pub hydrator: RelationshipHydrator<Parent, Related>,
    
    /// Whether to use type-safe containers
    pub use_type_safe: bool,
    
    /// Phantom data for type safety
    _phantom_parent: PhantomData<Parent>,
    _phantom_related: PhantomData<Related>,
}

impl<Parent, Related> TypeSafeEagerLoadSpec<Parent, Related>
where
    Parent: Model + DeserializeOwned + Send + Sync,
    Related: Model + DeserializeOwned + Send + Sync,
{
    pub fn new(relation: String, metadata: RelationshipMetadata) -> Self {
        Self {
            relationship_type: metadata.relationship_type,
            hydrator: RelationshipHydrator::new(metadata.clone()),
            relation,
            constraints: None,
            use_type_safe: true,
            _phantom_parent: PhantomData,
            _phantom_related: PhantomData,
        }
    }
    
    /// Add constraints to the eager loading specification
    pub fn with_constraints(mut self, constraints: RelationshipConstraintBuilder) -> Self {
        self.constraints = Some(constraints);
        self
    }
    
    /// Disable type-safe containers (fallback to legacy system)
    pub fn without_type_safety(mut self) -> Self {
        self.use_type_safe = false;
        self
    }
}

/// Type-safe eager loader with compile-time relationship validation
pub struct TypeSafeEagerLoader<Parent>
where
    Parent: InferableModel + DeserializeOwned + Send + Sync,
{
    /// The parent model type
    _parent: PhantomData<Parent>,
    
    /// Inference engine for automatic relationship discovery
    inference_engine: RelationshipInferenceEngine<Parent>,
    
    /// Loaded relationship data organized by relationship name and parent key
    loaded_data: HashMap<String, HashMap<String, TypeSafeRelationshipData>>,
}

/// Wrapper for type-safe relationship data
#[derive(Debug, Clone)]
pub enum TypeSafeRelationshipData {
    /// Single optional related model
    Single(Option<serde_json::Value>),
    
    /// Collection of related models
    Collection(Vec<serde_json::Value>),
    
    /// Polymorphic relationship with type info
    Polymorphic {
        data: Option<serde_json::Value>,
        morph_type: Option<String>,
        morph_id: Option<String>,
    },
}

impl<Parent> TypeSafeEagerLoader<Parent>
where
    Parent: InferableModel + DeserializeOwned + Send + Sync,
{
    /// Create a new type-safe eager loader
    pub fn new() -> Self {
        Self {
            _parent: PhantomData,
            inference_engine: RelationshipInferenceEngine::new(),
            loaded_data: HashMap::new(),
        }
    }
    
    /// Add a relationship to eagerly load with type safety
    pub fn with_typed_relationship<Related>(
        mut self, 
        relation: &str
    ) -> ModelResult<Self>
    where
        Related: InferableModel + DeserializeOwned + Send + Sync,
    {
        // Infer the relationship type and metadata
        let relationship_type = self.infer_relationship_type::<Related>(relation)?;
        let metadata = self.inference_engine.infer_relationship::<Related>(relation, relationship_type)?;
        
        // Store the relationship specification
        // For now, we'll just track that we want to load this relationship
        // In a full implementation, we would store the specification and use it during loading
        let _ = metadata; // Suppress unused warning
        
        Ok(self)
    }
    
    /// Add a relationship with explicit metadata
    pub fn with_relationship_metadata<Related>(
        self,
        metadata: RelationshipMetadata,
    ) -> Self
    where
        Related: InferableModel + DeserializeOwned + Send + Sync,
    {
        // Store relationship specification
        let _ = metadata; // Suppress unused warning for now
        self
    }
    
    /// Load relationships for a collection of models with type safety
    pub async fn load_for_models_typed<Related>(
        &mut self, 
        pool: &Pool<Postgres>, 
        parents: &mut [Parent],
        relation: &str,
    ) -> ModelResult<()>
    where
        Related: InferableModel + DeserializeOwned + Send + Sync,
    {
        if parents.is_empty() {
            return Ok(());
        }
        
        // Infer relationship metadata
        let relationship_type = self.infer_relationship_type::<Related>(relation)?;
        let metadata = self.inference_engine.infer_relationship::<Related>(relation, relationship_type)?;
        
        // Load the relationship data using the hydrator
        let hydrator = RelationshipHydrator::<Parent, Related>::new(metadata.clone());
        
        match relationship_type {
            RelationshipType::HasOne => {
                self.load_has_one_typed(pool, parents, relation, &hydrator).await?;
            }
            RelationshipType::HasMany => {
                self.load_has_many_typed(pool, parents, relation, &hydrator).await?;
            }
            RelationshipType::BelongsTo => {
                self.load_belongs_to_typed(pool, parents, relation, &hydrator).await?;
            }
            RelationshipType::ManyToMany => {
                self.load_many_to_many_typed(pool, parents, relation, &hydrator).await?;
            }
            RelationshipType::MorphOne | RelationshipType::MorphMany | RelationshipType::MorphTo => {
                self.load_polymorphic_typed(pool, parents, relation, &hydrator, relationship_type).await?;
            }
        }
        
        Ok(())
    }
    
    /// Load HasOne relationship with type safety
    async fn load_has_one_typed<Related>(
        &mut self,
        pool: &Pool<Postgres>,
        parents: &[Parent],
        relation: &str,
        hydrator: &RelationshipHydrator<Parent, Related>,
    ) -> ModelResult<()>
    where
        Related: Model + DeserializeOwned + Send + Sync,
    {
        // Collect parent keys
        let parent_keys: Vec<String> = parents
            .iter()
            .filter_map(|p| p.primary_key().map(|pk| pk.to_string()))
            .collect();
            
        if parent_keys.is_empty() {
            return Ok(());
        }
        
        // Build query to load related models
        let query = self.build_related_query::<Related>(&parent_keys, "user_id").await?; // TODO: Use proper foreign key
        
        // Execute query
        let rows = sqlx::query(&query).fetch_all(pool).await
            .map_err(|e| ModelError::Database(e.to_string()))?;
        
        // Group by parent key using the hydrator
        let grouped = hydrator.group_by_parent_key(&rows, "user_id")?; // TODO: Use proper foreign key
        
        // Store as type-safe data
        let mut relationship_data = HashMap::new();
        for (parent_key, related_models) in grouped {
            let data = if let Some(first) = related_models.into_iter().next() {
                TypeSafeRelationshipData::Single(Some(serde_json::to_value(first)?))
            } else {
                TypeSafeRelationshipData::Single(None)
            };
            relationship_data.insert(parent_key, data);
        }
        
        self.loaded_data.insert(relation.to_string(), relationship_data);
        Ok(())
    }
    
    /// Load HasMany relationship with type safety
    async fn load_has_many_typed<Related>(
        &mut self,
        pool: &Pool<Postgres>,
        parents: &[Parent],
        relation: &str,
        hydrator: &RelationshipHydrator<Parent, Related>,
    ) -> ModelResult<()>
    where
        Related: Model + DeserializeOwned + Send + Sync,
    {
        // Collect parent keys
        let parent_keys: Vec<String> = parents
            .iter()
            .filter_map(|p| p.primary_key().map(|pk| pk.to_string()))
            .collect();
            
        if parent_keys.is_empty() {
            return Ok(());
        }
        
        // Build query to load related models
        let query = self.build_related_query::<Related>(&parent_keys, "user_id").await?; // TODO: Use proper foreign key
        
        // Execute query
        let rows = sqlx::query(&query).fetch_all(pool).await
            .map_err(|e| ModelError::Database(e.to_string()))?;
        
        // Group by parent key using the hydrator
        let grouped = hydrator.group_by_parent_key(&rows, "user_id")?; // TODO: Use proper foreign key
        
        // Store as type-safe data
        let mut relationship_data = HashMap::new();
        for (parent_key, related_models) in grouped {
            let data = TypeSafeRelationshipData::Collection(
                related_models
                    .into_iter()
                    .map(|model| serde_json::to_value(model))
                    .collect::<Result<Vec<_>, _>>()?
            );
            relationship_data.insert(parent_key, data);
        }
        
        // Ensure all parents have an entry (even if empty)
        for parent in parents {
            if let Some(parent_key) = parent.primary_key().map(|pk| pk.to_string()) {
                relationship_data.entry(parent_key).or_insert_with(|| {
                    TypeSafeRelationshipData::Collection(Vec::new())
                });
            }
        }
        
        self.loaded_data.insert(relation.to_string(), relationship_data);
        Ok(())
    }
    
    /// Load BelongsTo relationship with type safety
    async fn load_belongs_to_typed<Related>(
        &mut self,
        pool: &Pool<Postgres>,
        parents: &[Parent],
        relation: &str,
        hydrator: &RelationshipHydrator<Parent, Related>,
    ) -> ModelResult<()>
    where
        Related: Model + DeserializeOwned + Send + Sync,
    {
        // For BelongsTo, we need to collect foreign key values from parents
        // and load the related models by their primary keys
        
        // This is a simplified implementation
        // In practice, we'd need to extract the foreign key values from the parent models
        let _ = (pool, parents, relation, hydrator); // Suppress unused warnings
        
        // TODO: Implement proper BelongsTo loading with foreign key extraction
        Ok(())
    }
    
    /// Load ManyToMany relationship with type safety
    async fn load_many_to_many_typed<Related>(
        &mut self,
        pool: &Pool<Postgres>,
        parents: &[Parent],
        relation: &str,
        hydrator: &RelationshipHydrator<Parent, Related>,
    ) -> ModelResult<()>
    where
        Related: Model + DeserializeOwned + Send + Sync,
    {
        // For ManyToMany, we need to query through the pivot table
        
        // This is a simplified implementation
        let _ = (pool, parents, relation, hydrator); // Suppress unused warnings
        
        // TODO: Implement proper ManyToMany loading with pivot table queries
        Ok(())
    }
    
    /// Load polymorphic relationships with type safety
    async fn load_polymorphic_typed<Related>(
        &mut self,
        pool: &Pool<Postgres>,
        parents: &[Parent],
        relation: &str,
        hydrator: &RelationshipHydrator<Parent, Related>,
        relationship_type: RelationshipType,
    ) -> ModelResult<()>
    where
        Related: Model + DeserializeOwned + Send + Sync,
    {
        // For polymorphic relationships, we need to handle the morph_type and morph_id columns
        
        // This is a simplified implementation
        let _ = (pool, parents, relation, hydrator, relationship_type); // Suppress unused warnings
        
        // TODO: Implement proper polymorphic relationship loading
        Ok(())
    }
    
    /// Build query for loading related models
    async fn build_related_query<Related>(
        &self,
        parent_keys: &[String],
        foreign_key: &str,
    ) -> ModelResult<String>
    where
        Related: Model,
    {
        let mut query = QueryBuilder::<Related>::new();
        
        query = query
            .select("*")
            .from(Related::table_name())
            .where_in(foreign_key, parent_keys.to_vec());
        
        Ok(query.to_sql())
    }
    
    /// Infer relationship type for a given field
    fn infer_relationship_type<Related>(&self, field_name: &str) -> ModelResult<RelationshipType>
    where
        Related: Model,
    {
        // Use the type inference helper
        if let Some(rt) = TypeInferenceHelper::infer_from_field_name(field_name) {
            Ok(rt)
        } else {
            // Default to HasMany for collections, HasOne for singles
            // This is a simplification - in practice we'd use more sophisticated inference
            if field_name.ends_with('s') && !field_name.ends_with("ss") {
                Ok(RelationshipType::HasMany)
            } else {
                Ok(RelationshipType::HasOne)
            }
        }
    }
    
    /// Get loaded relationship data for a parent
    pub fn get_loaded_data(&self, relation: &str, parent_key: &str) -> Option<&TypeSafeRelationshipData> {
        self.loaded_data
            .get(relation)?
            .get(parent_key)
    }
    
    /// Check if a relationship has been loaded
    pub fn is_loaded(&self, relation: &str) -> bool {
        self.loaded_data.contains_key(relation)
    }
    
    /// Get all loaded relationship names
    pub fn loaded_relations(&self) -> Vec<&String> {
        self.loaded_data.keys().collect()
    }
    
    /// Create a typed relationship container from loaded data
    pub fn create_typed_container<Related>(
        &self,
        relation: &str,
        parent_key: &str,
        relationship_type: RelationshipType,
    ) -> ModelResult<TypeSafeRelationship<Related>>
    where
        Related: Model + DeserializeOwned + Clone + std::fmt::Debug + Send + Sync,
    {
        // This would create a properly typed relationship container
        // For now, return a placeholder
        let metadata = super::metadata::RelationshipMetadata::new(
            relationship_type,
            relation.to_string(),
            Related::table_name().to_string(),
            std::any::type_name::<Related>().to_string(),
            super::metadata::ForeignKeyConfig::simple(
                "id".to_string(),
                Related::table_name().to_string(),
            ),
        );
        
        let mut container = TypeSafeRelationship::new(metadata);
        
        // Try to load data if available
        if let Some(data) = self.get_loaded_data(relation, parent_key) {
            match data {
                TypeSafeRelationshipData::Single(Some(json_data)) => {
                    if let Ok(related) = serde_json::from_value::<Related>(json_data.clone()) {
                        container.set_loaded(related);
                    }
                }
                TypeSafeRelationshipData::Collection(json_array) => {
                    let related_collection: Result<Vec<Related>, _> = json_array
                        .iter()
                        .map(|json| serde_json::from_value(json.clone()))
                        .collect();
                    
                    if let Ok(collection) = related_collection {
                        // This assumes T is Vec<Related> which might not always be true
                        // In a proper implementation, we'd handle this with proper typing
                        let _ = collection; // Suppress unused warning
                    }
                }
                _ => {}
            }
        }
        
        Ok(container)
    }
}

impl<Parent> Default for TypeSafeEagerLoader<Parent>
where
    Parent: InferableModel + DeserializeOwned + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for QueryBuilder to add type-safe eager loading
pub trait QueryBuilderTypeSafeEagerLoading<M> {
    /// Add a type-safe relationship to eagerly load
    fn with_typed<Related>(self, relation: &str) -> Self
    where
        Related: InferableModel + DeserializeOwned + Send + Sync,
        M: InferableModel + DeserializeOwned + Send + Sync;
    
    /// Add a type-safe relationship with constraints
    fn with_typed_where<Related, F>(self, relation: &str, constraint: F) -> Self
    where
        Related: InferableModel + DeserializeOwned + Send + Sync,
        M: InferableModel + DeserializeOwned + Send + Sync,
        F: FnOnce(RelationshipConstraintBuilder) -> RelationshipConstraintBuilder;
    
    /// Load relationship with specific type conversion
    fn with_typed_conversion<Related, Target>(self, relation: &str) -> Self
    where
        Related: InferableModel + DeserializeOwned + Send + Sync,
        Target: DeserializeOwned + Send + Sync,
        M: InferableModel + DeserializeOwned + Send + Sync;
}

// Note: The actual implementation of this trait would be added to the QueryBuilder
// in the query builder module, not here

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::metadata::*;
    
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    
    impl InferableModel for TestUser {
        fn relationship_hints() -> Vec<RelationshipHint> {
            vec![
                RelationshipHint {
                    field_name: "posts".to_string(),
                    relationship_type: RelationshipType::HasMany,
                    related_model: "Post".to_string(),
                    custom_foreign_key: None,
                    eager_load: false,
                },
            ]
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestPost {
        id: Option<i64>,
        title: String,
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
            fields.insert("user_id".to_string(), serde_json::json!(self.user_id));
            fields
        }
        
        fn from_row(row: &sqlx::postgres::PgRow) -> crate::error::ModelResult<Self> {
            use sqlx::Row;
            Ok(Self {
                id: row.try_get("id").ok(),
                title: row.try_get("title").unwrap_or_default(),
                user_id: row.try_get("user_id").ok(),
            })
        }
    }
    
    impl InferableModel for TestPost {}
    
    #[test]
    fn test_type_safe_eager_loader_creation() {
        let loader = TypeSafeEagerLoader::<TestUser>::new();
        
        assert!(loader.loaded_relations().is_empty());
        assert!(!loader.is_loaded("posts"));
    }
    
    #[test]
    fn test_type_safe_eager_load_spec() -> ModelResult<()> {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasMany,
            "posts".to_string(),
            "posts".to_string(),
            "TestPost".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
        );
        
        let spec = TypeSafeEagerLoadSpec::<TestUser, TestPost>::new("posts".to_string(), metadata);
        
        assert_eq!(spec.relation, "posts");
        assert_eq!(spec.relationship_type, RelationshipType::HasMany);
        assert!(spec.use_type_safe);
        
        Ok(())
    }
    
    #[test]
    fn test_relationship_type_inference() -> ModelResult<()> {
        let loader = TypeSafeEagerLoader::<TestUser>::new();
        
        let rt = loader.infer_relationship_type::<TestPost>("posts")?;
        assert_eq!(rt, RelationshipType::HasMany);
        
        let rt = loader.infer_relationship_type::<TestPost>("post")?;
        assert_eq!(rt, RelationshipType::HasOne);
        
        Ok(())
    }
    
    #[test]
    fn test_type_safe_relationship_data() {
        let data = TypeSafeRelationshipData::Single(Some(serde_json::json!({
            "id": 1,
            "title": "Test Post"
        })));
        
        match data {
            TypeSafeRelationshipData::Single(Some(_)) => assert!(true),
            _ => panic!("Expected single relationship data"),
        }
        
        let data = TypeSafeRelationshipData::Collection(vec![
            serde_json::json!({"id": 1, "title": "Post 1"}),
            serde_json::json!({"id": 2, "title": "Post 2"}),
        ]);
        
        match data {
            TypeSafeRelationshipData::Collection(ref items) => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Expected collection relationship data"),
        }
    }
}