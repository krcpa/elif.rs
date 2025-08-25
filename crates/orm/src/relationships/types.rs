//! Relationship Types - Definitions and behaviors for different relationship types

use async_trait::async_trait;
use sqlx::Pool;
use sqlx::Postgres;
use std::marker::PhantomData;

use super::metadata::{RelationshipMetadata, RelationshipType};
use crate::error::ModelResult;
use crate::model::Model;
use crate::query::QueryBuilder;

/// A relationship container that holds metadata and loaded state
#[derive(Debug, Clone)]
pub struct Relationship<T> {
    /// The relationship metadata
    metadata: RelationshipMetadata,

    /// Whether the relationship has been loaded
    loaded: bool,

    /// The loaded data (if any)
    data: Option<T>,

    /// Phantom data for type safety
    _phantom: PhantomData<T>,
}

impl<T> Relationship<T> {
    /// Create a new relationship instance
    pub fn new(metadata: RelationshipMetadata) -> Self {
        Self {
            metadata,
            loaded: false,
            data: None,
            _phantom: PhantomData,
        }
    }

    /// Get the relationship metadata
    pub fn metadata(&self) -> &RelationshipMetadata {
        &self.metadata
    }

    /// Check if the relationship is loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Mark the relationship as loaded with data
    pub fn set_loaded(&mut self, data: T) {
        self.data = Some(data);
        self.loaded = true;
    }

    /// Mark the relationship as unloaded
    pub fn unload(&mut self) {
        self.data = None;
        self.loaded = false;
    }

    /// Get the loaded data (if available)
    pub fn get(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Get the loaded data mutably (if available)
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }

    /// Take the loaded data, leaving None in its place
    pub fn take(&mut self) -> Option<T> {
        self.loaded = false;
        self.data.take()
    }

    /// Get the relationship type from metadata
    pub fn relationship_type(&self) -> RelationshipType {
        self.metadata.relationship_type
    }

    /// Get the relationship name
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Check if this relationship is a collection type
    pub fn is_collection(&self) -> bool {
        self.metadata.relationship_type.is_collection()
    }

    /// Check if this relationship is polymorphic
    pub fn is_polymorphic(&self) -> bool {
        self.metadata.relationship_type.is_polymorphic()
    }
}

/// HasOne relationship - one parent has one related model
pub type HasOneRelationship<Related> = Relationship<Option<Related>>;

/// HasMany relationship - one parent has many related models
pub type HasManyRelationship<Related> = Relationship<Vec<Related>>;

/// BelongsTo relationship - many models belong to one parent
pub type BelongsToRelationship<Related> = Relationship<Option<Related>>;

/// ManyToMany relationship - many models related to many through pivot
pub type ManyToManyRelationship<Related> = Relationship<Vec<Related>>;

/// MorphOne relationship - polymorphic one-to-one
pub type MorphOneRelationship<Related> = Relationship<Option<Related>>;

/// MorphMany relationship - polymorphic one-to-many
pub type MorphManyRelationship<Related> = Relationship<Vec<Related>>;

/// MorphTo relationship - inverse polymorphic relationship
pub type MorphToRelationship<Related> = Relationship<Option<Related>>;

/// Trait for loading relationships from the database
#[async_trait]
pub trait RelationshipLoader<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + Send + Sync,
{
    /// Load the relationship data for a single parent instance
    async fn load_for_instance(&self, parent: &Parent, pool: &Pool<Postgres>) -> ModelResult<()>;

    /// Load the relationship data for multiple parent instances (eager loading)
    async fn load_for_instances(
        &self,
        parents: &mut [Parent],
        pool: &Pool<Postgres>,
    ) -> ModelResult<()>;

    /// Build a query for this relationship
    fn build_query(&self, parent: &Parent) -> QueryBuilder<Related>;

    /// Get constraints for this relationship
    fn get_constraints(&self) -> &[super::metadata::RelationshipConstraint];
}

/// Builder for creating relationship queries with constraints
#[derive(Debug, Clone)]
pub struct RelationshipQueryBuilder<T> {
    /// The base query builder
    query: QueryBuilder<T>,

    /// The relationship metadata
    metadata: RelationshipMetadata,

    /// Additional constraints applied to this query
    constraints: Vec<super::metadata::RelationshipConstraint>,
}

impl<T> RelationshipQueryBuilder<T>
where
    T: Model + Send + Sync,
{
    /// Create a new relationship query builder
    pub fn new(metadata: RelationshipMetadata) -> Self {
        let mut query = QueryBuilder::<T>::new();

        // Apply relationship-specific constraints from metadata
        for constraint in &metadata.constraints {
            query = query.where_raw(&format!(
                "{} {} '{}'",
                constraint.column,
                constraint.operator.to_sql(),
                constraint.value
            ));
        }

        Self {
            query,
            constraints: metadata.constraints.clone(),
            metadata,
        }
    }

    /// Add an additional constraint to the relationship query
    pub fn where_constraint(
        mut self,
        column: &str,
        operator: super::metadata::ConstraintOperator,
        value: String,
    ) -> Self {
        self.constraints
            .push(super::metadata::RelationshipConstraint {
                column: column.to_string(),
                operator: operator.clone(),
                value: value.clone(),
            });

        self.query = self
            .query
            .where_raw(&format!("{} {} '{}'", column, operator.to_sql(), value));

        self
    }

    /// Get the underlying query builder
    pub fn query(&self) -> &QueryBuilder<T> {
        &self.query
    }

    /// Get the underlying query builder mutably
    pub fn query_mut(&mut self) -> &mut QueryBuilder<T> {
        &mut self.query
    }

    /// Get the relationship metadata
    pub fn metadata(&self) -> &RelationshipMetadata {
        &self.metadata
    }

    /// Execute the relationship query
    pub async fn execute(&self, pool: &Pool<Postgres>) -> ModelResult<Vec<T>> {
        self.query.clone().get(pool).await
    }

    /// Execute the relationship query and return the first result
    pub async fn first(&self, pool: &Pool<Postgres>) -> ModelResult<Option<T>> {
        self.query.clone().first(pool).await
    }
}

/// Trait for relationship-aware models
pub trait WithRelationships {
    /// Get metadata for a specific relationship by name
    fn relationship_metadata(name: &str) -> Option<&'static RelationshipMetadata>;

    /// Get all relationship metadata for this model
    fn all_relationship_metadata() -> &'static [RelationshipMetadata];

    /// Check if a relationship exists
    fn has_relationship(name: &str) -> bool {
        Self::relationship_metadata(name).is_some()
    }

    /// Get all relationship names
    fn relationship_names() -> Vec<&'static str> {
        Self::all_relationship_metadata()
            .iter()
            .map(|meta| meta.name.as_str())
            .collect()
    }

    /// Check if any relationships should be eagerly loaded
    fn has_eager_relationships() -> bool {
        Self::all_relationship_metadata()
            .iter()
            .any(|meta| meta.eager_load)
    }

    /// Get all relationships that should be eagerly loaded
    fn eager_relationships() -> Vec<&'static RelationshipMetadata> {
        Self::all_relationship_metadata()
            .iter()
            .filter(|meta| meta.eager_load)
            .collect()
    }
}

/// Utility functions for relationship type detection
pub mod utils {
    use super::*;

    /// Detect the inverse relationship type for a given type
    pub fn inverse_relationship_type(
        relationship_type: RelationshipType,
    ) -> Option<RelationshipType> {
        match relationship_type {
            RelationshipType::HasOne => Some(RelationshipType::BelongsTo),
            RelationshipType::HasMany => Some(RelationshipType::BelongsTo),
            RelationshipType::BelongsTo => Some(RelationshipType::HasOne), // or HasMany
            RelationshipType::ManyToMany => Some(RelationshipType::ManyToMany),
            // Polymorphic relationships don't have simple inverses
            RelationshipType::MorphOne => None,
            RelationshipType::MorphMany => None,
            RelationshipType::MorphTo => None,
        }
    }

    /// Generate a default foreign key name for a relationship
    pub fn default_foreign_key(model_name: &str) -> String {
        format!("{}_id", model_name.to_lowercase())
    }

    /// Generate a default pivot table name for many-to-many relationships
    pub fn default_pivot_table_name(local_table: &str, foreign_table: &str) -> String {
        let mut tables = [local_table, foreign_table];
        tables.sort();
        tables.join("_")
    }

    /// Check if a relationship configuration is valid for the given type
    pub fn validate_relationship_configuration(
        relationship_type: RelationshipType,
        metadata: &RelationshipMetadata,
    ) -> ModelResult<()> {
        match relationship_type {
            RelationshipType::ManyToMany => {
                if metadata.pivot_config.is_none() {
                    return Err(crate::error::ModelError::Configuration(
                        "ManyToMany relationships require pivot configuration".to_string(),
                    ));
                }
            }
            RelationshipType::MorphOne
            | RelationshipType::MorphMany
            | RelationshipType::MorphTo => {
                if metadata.polymorphic_config.is_none() {
                    return Err(crate::error::ModelError::Configuration(
                        "Polymorphic relationships require polymorphic configuration".to_string(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::metadata::*;
    use super::*;

    // Mock model for testing
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct MockModel {
        id: Option<i64>,
        name: String,
    }

    impl Model for MockModel {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "mock_models"
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
            fields.insert(
                "name".to_string(),
                serde_json::Value::String(self.name.clone()),
            );
            fields
        }

        fn from_row(row: &sqlx::postgres::PgRow) -> crate::error::ModelResult<Self> {
            use sqlx::Row;
            Ok(Self {
                id: row.try_get("id").ok(),
                name: row.try_get("name").unwrap_or_default(),
            })
        }
    }

    #[test]
    fn test_relationship_creation() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasMany,
            "posts".to_string(),
            "posts".to_string(),
            "Post".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
        );

        let relationship: HasManyRelationship<MockModel> = Relationship::new(metadata);

        assert!(!relationship.is_loaded());
        assert_eq!(relationship.relationship_type(), RelationshipType::HasMany);
        assert_eq!(relationship.name(), "posts");
        assert!(relationship.is_collection());
        assert!(!relationship.is_polymorphic());
    }

    #[test]
    fn test_relationship_loading_state() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasOne,
            "profile".to_string(),
            "profiles".to_string(),
            "Profile".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
        );

        let mut relationship: HasOneRelationship<MockModel> = Relationship::new(metadata);

        // Initially not loaded
        assert!(!relationship.is_loaded());
        assert!(relationship.get().is_none());

        // Load data
        let mock_profile = MockModel {
            id: Some(1),
            name: "Profile".to_string(),
        };
        relationship.set_loaded(Some(mock_profile));

        assert!(relationship.is_loaded());
        assert!(relationship.get().is_some());

        // Unload
        relationship.unload();
        assert!(!relationship.is_loaded());
        assert!(relationship.get().is_none());
    }

    #[test]
    fn test_relationship_query_builder() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasMany,
            "posts".to_string(),
            "posts".to_string(),
            "Post".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
        );

        let query_builder = RelationshipQueryBuilder::<MockModel>::new(metadata.clone());

        assert_eq!(query_builder.metadata().name, "posts");
        assert_eq!(
            query_builder.metadata().relationship_type,
            RelationshipType::HasMany
        );
    }

    #[test]
    fn test_relationship_utils() {
        use super::utils::*;

        assert_eq!(
            inverse_relationship_type(RelationshipType::HasOne),
            Some(RelationshipType::BelongsTo)
        );

        assert_eq!(
            inverse_relationship_type(RelationshipType::HasMany),
            Some(RelationshipType::BelongsTo)
        );

        assert_eq!(
            inverse_relationship_type(RelationshipType::ManyToMany),
            Some(RelationshipType::ManyToMany)
        );

        assert_eq!(default_foreign_key("User"), "user_id");
        assert_eq!(default_pivot_table_name("users", "roles"), "roles_users");
        assert_eq!(default_pivot_table_name("roles", "users"), "roles_users"); // Should be sorted
    }

    #[test]
    fn test_relationship_type_properties() {
        assert!(RelationshipType::HasMany.is_collection());
        assert!(RelationshipType::ManyToMany.is_collection());
        assert!(!RelationshipType::HasOne.is_collection());
        assert!(!RelationshipType::BelongsTo.is_collection());

        assert!(RelationshipType::MorphOne.is_polymorphic());
        assert!(RelationshipType::MorphMany.is_polymorphic());
        assert!(RelationshipType::MorphTo.is_polymorphic());
        assert!(!RelationshipType::HasOne.is_polymorphic());

        assert!(RelationshipType::ManyToMany.requires_pivot());
        assert!(!RelationshipType::HasMany.requires_pivot());
    }
}
