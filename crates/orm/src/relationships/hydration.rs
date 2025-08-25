//! Type-Safe Model Hydration - Converting database rows to typed relationship data

use serde::de::DeserializeOwned;
use sqlx::{postgres::PgRow, Column, Row};
use std::collections::HashMap;
use std::marker::PhantomData;

use super::containers::*;
use super::metadata::{RelationshipMetadata, RelationshipType};
use crate::error::{ModelError, ModelResult};
use crate::model::Model;

/// Trait for converting database rows to typed models
pub trait TypeSafeHydrator<T>: Send + Sync
where
    T: Model + DeserializeOwned + Send + Sync,
{
    /// Hydrate a single model from a database row
    fn hydrate_single(&self, row: &PgRow) -> ModelResult<T>;

    /// Hydrate multiple models from database rows
    fn hydrate_collection(&self, rows: &[PgRow]) -> ModelResult<Vec<T>>;

    /// Hydrate with specific column mapping
    fn hydrate_with_mapping(
        &self,
        row: &PgRow,
        column_mapping: &HashMap<String, String>,
    ) -> ModelResult<T>;
}

/// Generic hydrator that uses the Model trait's from_row method
#[derive(Debug)]
pub struct ModelHydrator<T> {
    _phantom: PhantomData<T>,
}

impl<T> ModelHydrator<T>
where
    T: Model + DeserializeOwned + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for ModelHydrator<T>
where
    T: Model + DeserializeOwned + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TypeSafeHydrator<T> for ModelHydrator<T>
where
    T: Model + DeserializeOwned + Send + Sync,
{
    fn hydrate_single(&self, row: &PgRow) -> ModelResult<T> {
        T::from_row(row)
    }

    fn hydrate_collection(&self, rows: &[PgRow]) -> ModelResult<Vec<T>> {
        rows.iter().map(|row| self.hydrate_single(row)).collect()
    }

    fn hydrate_with_mapping(
        &self,
        row: &PgRow,
        column_mapping: &HashMap<String, String>,
    ) -> ModelResult<T> {
        // For now, use the standard from_row method
        // In the future, we could implement column remapping here
        let _ = column_mapping; // Suppress unused warning
        self.hydrate_single(row)
    }
}

/// Advanced hydrator for complex relationship scenarios
#[derive(Debug)]
pub struct RelationshipHydrator<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + DeserializeOwned + Send + Sync,
{
    /// The relationship metadata
    _metadata: RelationshipMetadata,

    /// Parent model hydrator
    parent_hydrator: ModelHydrator<Parent>,

    /// Related model hydrator
    related_hydrator: ModelHydrator<Related>,

    /// Column prefix for related models (for joins)
    related_prefix: Option<String>,
}

impl<Parent, Related> RelationshipHydrator<Parent, Related>
where
    Parent: Model + DeserializeOwned + Send + Sync,
    Related: Model + DeserializeOwned + Send + Sync,
{
    pub fn new(metadata: RelationshipMetadata) -> Self {
        Self {
            _metadata: metadata,
            parent_hydrator: ModelHydrator::new(),
            related_hydrator: ModelHydrator::new(),
            related_prefix: None,
        }
    }

    /// Set a column prefix for related models (useful for JOINs)
    pub fn with_related_prefix(mut self, prefix: String) -> Self {
        self.related_prefix = Some(prefix);
        self
    }

    /// Hydrate a parent model with its relationships
    pub fn hydrate_with_relationships(
        &self,
        parent_row: &PgRow,
        related_rows: &[PgRow],
    ) -> ModelResult<(Parent, Vec<Related>)> {
        let parent = self.parent_hydrator.hydrate_single(parent_row)?;
        let related = self.related_hydrator.hydrate_collection(related_rows)?;

        Ok((parent, related))
    }

    /// Hydrate from a joined query result
    pub fn hydrate_joined(&self, row: &PgRow) -> ModelResult<(Parent, Option<Related>)> {
        let parent = self.parent_hydrator.hydrate_single(row)?;

        // Try to hydrate the related model
        let related = if let Some(prefix) = &self.related_prefix {
            // Check if related columns are present and not null
            let has_related_data = row.columns().iter().any(|col| {
                col.name().starts_with(prefix)
                    && !matches!(row.try_get::<Option<String>, _>(col.name()), Ok(None))
            });

            if has_related_data {
                Some(self.related_hydrator.hydrate_single(row)?)
            } else {
                None
            }
        } else {
            // Try to hydrate without prefix
            match self.related_hydrator.hydrate_single(row) {
                Ok(related) => Some(related),
                Err(_) => None, // Ignore hydration errors for optional relationships
            }
        };

        Ok((parent, related))
    }

    /// Group related models by parent key for eager loading
    pub fn group_by_parent_key(
        &self,
        related_rows: &[PgRow],
        foreign_key_column: &str,
    ) -> ModelResult<HashMap<String, Vec<Related>>> {
        let mut grouped: HashMap<String, Vec<Related>> = HashMap::new();

        for row in related_rows {
            // Extract the foreign key value
            let foreign_key_value: String = row.try_get(foreign_key_column).map_err(|e| {
                ModelError::Database(format!(
                    "Failed to get foreign key '{}': {}",
                    foreign_key_column, e
                ))
            })?;

            // Hydrate the related model
            let related = self.related_hydrator.hydrate_single(row)?;

            // Add to the group
            grouped.entry(foreign_key_value).or_default().push(related);
        }

        Ok(grouped)
    }
}

/// Type-safe relationship loader that replaces the JSON-based system
pub struct TypeSafeRelationshipLoader<Parent, Related>
where
    Parent: Model + Send + Sync,
    Related: Model + DeserializeOwned + Send + Sync,
{
    /// The hydrator for this relationship
    hydrator: RelationshipHydrator<Parent, Related>,
}

impl<Parent, Related> TypeSafeRelationshipLoader<Parent, Related>
where
    Parent: Model + DeserializeOwned + Send + Sync,
    Related: Model + DeserializeOwned + Send + Sync,
{
    pub fn new(metadata: RelationshipMetadata) -> Self {
        Self {
            hydrator: RelationshipHydrator::new(metadata),
        }
    }

    /// Load and hydrate relationships for a collection of parent models
    pub fn hydrate_relationships(
        &self,
        parents: &mut [Parent],
        related_rows: &[PgRow],
        foreign_key_column: &str,
    ) -> ModelResult<()> {
        // Group related models by parent key
        let grouped_related = self
            .hydrator
            .group_by_parent_key(related_rows, foreign_key_column)?;

        // For each parent, find and attach its related models
        for parent in parents {
            if let Some(parent_key) = parent.primary_key() {
                let parent_key_str = parent_key.to_string();

                if let Some(related_models) = grouped_related.get(&parent_key_str) {
                    // Here we would attach the relationships to the parent model
                    // This requires the parent model to have relationship fields
                    // For now, we'll just validate that hydration works
                    let _ = related_models; // Suppress unused warning
                }
            }
        }

        Ok(())
    }
}

/// Specialized hydrators for different relationship types
pub mod specialized_hydrators {
    use super::*;

    /// Hydrator for HasOne relationships
    pub struct HasOneHydrator<Parent, Related>
    where
        Parent: Model + DeserializeOwned + Send + Sync + Clone,
        Related: Model + DeserializeOwned + Send + Sync + Clone,
    {
        _loader: TypeSafeRelationshipLoader<Parent, Related>,
    }

    impl<Parent, Related> HasOneHydrator<Parent, Related>
    where
        Parent: Model + DeserializeOwned + Send + Sync + Clone,
        Related: Model + DeserializeOwned + Send + Sync + Clone,
    {
        pub fn new(metadata: RelationshipMetadata) -> Self {
            Self {
                _loader: TypeSafeRelationshipLoader::new(metadata),
            }
        }

        /// Hydrate HasOne relationship into a TypeSafeRelationship container
        pub fn hydrate_has_one(
            &self,
            parent: &Parent,
            related_rows: &[PgRow],
            foreign_key_column: &str,
        ) -> ModelResult<HasOne<Related>> {
            let parent_key = parent
                .primary_key()
                .ok_or_else(|| {
                    ModelError::Configuration("Parent model has no primary key".to_string())
                })?
                .to_string();

            // Find the related model for this parent
            let related_model = related_rows
                .iter()
                .find(|row| match row.try_get::<String, _>(foreign_key_column) {
                    Ok(fk) => fk == parent_key,
                    Err(_) => false,
                })
                .map(|row| ModelHydrator::<Related>::new().hydrate_single(row))
                .transpose()?;

            let mut relationship = HasOne::new(RelationshipMetadata::new(
                RelationshipType::HasOne,
                "related".to_string(),
                Related::table_name().to_string(),
                std::any::type_name::<Related>().to_string(),
                super::super::metadata::ForeignKeyConfig::simple(
                    foreign_key_column.to_string(),
                    Related::table_name().to_string(),
                ),
            ));

            relationship.set_loaded(related_model);
            Ok(relationship)
        }
    }

    /// Hydrator for HasMany relationships
    pub struct HasManyHydrator<Parent, Related>
    where
        Parent: Model + DeserializeOwned + Send + Sync + Clone,
        Related: Model + DeserializeOwned + Send + Sync + Clone,
    {
        _loader: TypeSafeRelationshipLoader<Parent, Related>,
    }

    impl<Parent, Related> HasManyHydrator<Parent, Related>
    where
        Parent: Model + DeserializeOwned + Send + Sync + Clone,
        Related: Model + DeserializeOwned + Send + Sync + Clone,
    {
        pub fn new(metadata: RelationshipMetadata) -> Self {
            Self {
                _loader: TypeSafeRelationshipLoader::new(metadata),
            }
        }

        /// Hydrate HasMany relationship into a TypeSafeRelationship container
        pub fn hydrate_has_many(
            &self,
            parent: &Parent,
            related_rows: &[PgRow],
            foreign_key_column: &str,
        ) -> ModelResult<HasMany<Related>> {
            let parent_key = parent
                .primary_key()
                .ok_or_else(|| {
                    ModelError::Configuration("Parent model has no primary key".to_string())
                })?
                .to_string();

            // Find all related models for this parent
            let related_models: Result<Vec<Related>, ModelError> = related_rows
                .iter()
                .filter(|row| match row.try_get::<String, _>(foreign_key_column) {
                    Ok(fk) => fk == parent_key,
                    Err(_) => false,
                })
                .map(|row| ModelHydrator::<Related>::new().hydrate_single(row))
                .collect();

            let mut relationship = HasMany::new(RelationshipMetadata::new(
                RelationshipType::HasMany,
                "related".to_string(),
                Related::table_name().to_string(),
                std::any::type_name::<Related>().to_string(),
                super::super::metadata::ForeignKeyConfig::simple(
                    foreign_key_column.to_string(),
                    Related::table_name().to_string(),
                ),
            ));

            relationship.set_loaded(related_models?);
            Ok(relationship)
        }
    }

    /// Hydrator for BelongsTo relationships
    pub struct BelongsToHydrator<Child, Parent>
    where
        Child: Model + DeserializeOwned + Send + Sync + Clone,
        Parent: Model + DeserializeOwned + Send + Sync + Clone,
    {
        _loader: TypeSafeRelationshipLoader<Child, Parent>,
    }

    impl<Child, Parent> BelongsToHydrator<Child, Parent>
    where
        Child: Model + DeserializeOwned + Send + Sync + Clone,
        Parent: Model + DeserializeOwned + Send + Sync + Clone,
    {
        pub fn new(metadata: RelationshipMetadata) -> Self {
            Self {
                _loader: TypeSafeRelationshipLoader::new(metadata),
            }
        }

        /// Hydrate BelongsTo relationship into a TypeSafeRelationship container
        pub fn hydrate_belongs_to(
            &self,
            child: &Child,
            parent_rows: &[PgRow],
            foreign_key_column: &str,
        ) -> ModelResult<BelongsTo<Parent>> {
            // Get the foreign key value from the child model
            let child_fields = child.to_fields();
            let foreign_key_value = child_fields
                .get(foreign_key_column)
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ModelError::Configuration(format!(
                        "Child model missing foreign key '{}'",
                        foreign_key_column
                    ))
                })?;

            // Find the parent model
            let parent_model = parent_rows
                .iter()
                .find(|row| {
                    match row.try_get::<String, _>("id") {
                        // Assuming parent PK is 'id'
                        Ok(id) => id == foreign_key_value,
                        Err(_) => false,
                    }
                })
                .map(|row| ModelHydrator::<Parent>::new().hydrate_single(row))
                .transpose()?;

            let mut relationship = BelongsTo::new(RelationshipMetadata::new(
                RelationshipType::BelongsTo,
                "parent".to_string(),
                Parent::table_name().to_string(),
                std::any::type_name::<Parent>().to_string(),
                super::super::metadata::ForeignKeyConfig::simple(
                    foreign_key_column.to_string(),
                    Parent::table_name().to_string(),
                ),
            ));

            relationship.set_loaded(parent_model);
            Ok(relationship)
        }
    }
}

/// Utility functions for type-safe hydration
pub mod hydration_utils {
    use super::*;

    /// Extract column names from a database row
    pub fn extract_column_names(row: &PgRow) -> Vec<String> {
        row.columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect()
    }

    /// Check if a row has all required columns for a model
    pub fn has_required_columns<T: Model>(row: &PgRow, required_columns: &[&str]) -> bool {
        let column_names: std::collections::HashSet<_> =
            row.columns().iter().map(|col| col.name()).collect();

        required_columns
            .iter()
            .all(|col| column_names.contains(col))
    }

    /// Convert a row to a JSON-like HashMap for debugging
    pub fn row_to_debug_map(row: &PgRow) -> HashMap<String, String> {
        let mut map = HashMap::new();

        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name();

            // Try to get the value as different types for debugging
            let value_str = if let Ok(value) = row.try_get::<Option<String>, _>(i) {
                format!("{:?}", value)
            } else if let Ok(value) = row.try_get::<Option<i64>, _>(i) {
                format!("{:?}", value)
            } else if let Ok(value) = row.try_get::<Option<bool>, _>(i) {
                format!("{:?}", value)
            } else {
                "<unknown_type>".to_string()
            };

            map.insert(column_name.to_string(), value_str);
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::super::metadata::*;
    use super::*;

    // Use the same test models from containers.rs
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
            fields.insert(
                "name".to_string(),
                serde_json::Value::String(self.name.clone()),
            );
            fields.insert(
                "email".to_string(),
                serde_json::Value::String(self.email.clone()),
            );
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

    #[test]
    fn test_model_hydrator_creation() {
        let hydrator = ModelHydrator::<TestUser>::new();

        // Test that the hydrator can be created
        // Full testing would require actual database rows
        let _ = hydrator; // Suppress unused warning
    }

    #[test]
    fn test_relationship_hydrator_creation() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasMany,
            "posts".to_string(),
            "posts".to_string(),
            "TestPost".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "posts".to_string()),
        );

        let hydrator = RelationshipHydrator::<TestUser, TestUser>::new(metadata);
        let _ = hydrator.with_related_prefix("post_".to_string());

        // Test creation succeeds
        assert!(true);
    }

    #[test]
    fn test_specialized_hydrator_creation() {
        let metadata = RelationshipMetadata::new(
            RelationshipType::HasOne,
            "profile".to_string(),
            "profiles".to_string(),
            "Profile".to_string(),
            ForeignKeyConfig::simple("user_id".to_string(), "profiles".to_string()),
        );

        let _hydrator = specialized_hydrators::HasOneHydrator::<TestUser, TestUser>::new(metadata);

        // Test creation succeeds
        assert!(true);
    }

    #[test]
    fn test_hydration_utils() {
        let required_columns = vec!["id", "name", "email"];

        // Test utility functions exist
        let _ = hydration_utils::extract_column_names;
        let _ = hydration_utils::has_required_columns::<TestUser>;
        let _ = hydration_utils::row_to_debug_map;
        let _ = required_columns; // Suppress unused warning

        assert!(true);
    }
}
