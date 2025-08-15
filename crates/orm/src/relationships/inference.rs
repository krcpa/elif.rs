//! Relationship Type Inference - Utilities for inferring relationship types and configurations

use std::marker::PhantomData;
use std::collections::HashMap;
use serde::de::DeserializeOwned;

use crate::error::ModelResult;
use crate::model::Model;
use super::metadata::{RelationshipMetadata, RelationshipType, ForeignKeyConfig, PivotConfig, PolymorphicConfig};

/// Trait for models that can have their relationships inferred
pub trait InferableModel: Model {
    /// Get relationship inference hints for this model
    fn relationship_hints() -> Vec<RelationshipHint> {
        Vec::new()
    }
    
    /// Get foreign key naming convention for this model
    fn foreign_key_convention() -> ForeignKeyConvention {
        ForeignKeyConvention::Underscore
    }
    
    /// Get table naming convention for this model
    fn table_naming_convention() -> TableNamingConvention {
        TableNamingConvention::Plural
    }
}

/// Hint for relationship inference
#[derive(Debug, Clone)]
pub struct RelationshipHint {
    /// The field name in the model
    pub field_name: String,
    
    /// The expected relationship type
    pub relationship_type: RelationshipType,
    
    /// The related model type name
    pub related_model: String,
    
    /// Custom foreign key if different from convention
    pub custom_foreign_key: Option<String>,
    
    /// Whether this should be eagerly loaded by default
    pub eager_load: bool,
}

/// Foreign key naming conventions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForeignKeyConvention {
    /// model_id (e.g., user_id)
    Underscore,
    /// modelId (camelCase)
    CamelCase,
    /// modelID (PascalCase with ID suffix)
    PascalCase,
    /// Custom pattern with {model} placeholder
    Custom(&'static str),
}

/// Table naming conventions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableNamingConvention {
    /// Plural form (users, posts)
    Plural,
    /// Singular form (user, post)
    Singular,
    /// Custom pattern
    Custom(&'static str),
}

/// Relationship type inference engine
pub struct RelationshipInferenceEngine<Parent>
where
    Parent: InferableModel + DeserializeOwned + Send + Sync,
{
    parent_model: PhantomData<Parent>,
    
    /// Cache of inferred relationships
    inference_cache: HashMap<String, RelationshipMetadata>,
}

impl<Parent> RelationshipInferenceEngine<Parent>
where
    Parent: InferableModel + DeserializeOwned + Send + Sync,
{
    /// Create a new inference engine
    pub fn new() -> Self {
        Self {
            parent_model: PhantomData,
            inference_cache: HashMap::new(),
        }
    }
    
    /// Infer relationship metadata for a given field name and related model
    pub fn infer_relationship<Related>(
        &mut self,
        field_name: &str,
        relationship_type: RelationshipType,
    ) -> ModelResult<RelationshipMetadata>
    where
        Related: InferableModel + DeserializeOwned + Send + Sync,
    {
        let cache_key = format!("{}::{}", field_name, std::any::type_name::<Related>());
        
        // Check cache first
        if let Some(cached) = self.inference_cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        
        let metadata = self.infer_relationship_metadata::<Related>(field_name, relationship_type)?;
        
        // Cache the result
        self.inference_cache.insert(cache_key, metadata.clone());
        
        Ok(metadata)
    }
    
    /// Infer all relationships for the parent model using hints
    pub fn infer_all_relationships(&mut self) -> ModelResult<Vec<RelationshipMetadata>> {
        let hints = Parent::relationship_hints();
        let mut relationships = Vec::new();
        
        for hint in hints {
            let metadata = self.infer_from_hint(&hint)?;
            relationships.push(metadata);
        }
        
        Ok(relationships)
    }
    
    /// Infer relationship metadata from model structure and naming conventions
    fn infer_relationship_metadata<Related>(
        &self,
        field_name: &str,
        relationship_type: RelationshipType,
    ) -> ModelResult<RelationshipMetadata>
    where
        Related: InferableModel + DeserializeOwned + Send + Sync,
    {
        let parent_table = Parent::table_name();
        let related_table = Related::table_name();
        let related_model_name = std::any::type_name::<Related>()
            .split("::")
            .last()
            .unwrap_or(std::any::type_name::<Related>());
        
        let foreign_key_config = match relationship_type {
            RelationshipType::HasOne | RelationshipType::HasMany => {
                // Related table has foreign key pointing to parent
                let foreign_key = self.infer_foreign_key_name(Parent::table_name())?;
                ForeignKeyConfig::simple(foreign_key, related_table.to_string())
            }
            RelationshipType::BelongsTo => {
                // Parent table has foreign key pointing to related
                let foreign_key = self.infer_foreign_key_name(Related::table_name())?;
                ForeignKeyConfig::simple(foreign_key, parent_table.to_string())
            }
            RelationshipType::ManyToMany => {
                // Pivot table with both foreign keys
                let pivot_table = self.infer_pivot_table_name(parent_table, related_table);
                let local_key = self.infer_foreign_key_name(parent_table)?;
                let foreign_key = self.infer_foreign_key_name(related_table)?;
                
                return Ok(RelationshipMetadata::new(
                    relationship_type,
                    field_name.to_string(),
                    related_table.to_string(),
                    related_model_name.to_string(),
                    ForeignKeyConfig::simple(local_key.clone(), pivot_table.clone()),
                ).with_pivot(PivotConfig::new(pivot_table, local_key, foreign_key)));
            }
            RelationshipType::MorphOne | RelationshipType::MorphMany | RelationshipType::MorphTo => {
                // Polymorphic relationships
                let (type_column, id_column) = self.infer_polymorphic_columns(field_name);
                
                return Ok(RelationshipMetadata::new(
                    relationship_type,
                    field_name.to_string(),
                    related_table.to_string(),
                    related_model_name.to_string(),
                    ForeignKeyConfig::simple(id_column.clone(), related_table.to_string()),
                ).with_polymorphic(PolymorphicConfig::new(
                    field_name.to_string(),
                    type_column,
                    id_column,
                )));
            }
        };
        
        Ok(RelationshipMetadata::new(
            relationship_type,
            field_name.to_string(),
            related_table.to_string(),
            related_model_name.to_string(),
            foreign_key_config,
        ))
    }
    
    /// Infer relationship metadata from a hint
    fn infer_from_hint(&self, hint: &RelationshipHint) -> ModelResult<RelationshipMetadata> {
        let foreign_key = hint.custom_foreign_key
            .clone()
            .unwrap_or_else(|| {
                match self.infer_foreign_key_name(&hint.related_model.to_lowercase()) {
                    Ok(fk) => fk,
                    Err(_) => format!("{}_id", hint.related_model.to_lowercase()),
                }
            });
        
        let related_table = self.infer_table_name(&hint.related_model);
        
        let mut metadata = RelationshipMetadata::new(
            hint.relationship_type,
            hint.field_name.clone(),
            related_table,
            hint.related_model.clone(),
            ForeignKeyConfig::simple(foreign_key, hint.related_model.to_lowercase()),
        );
        
        metadata.eager_load = hint.eager_load;
        
        Ok(metadata)
    }
    
    /// Infer foreign key name based on convention
    pub fn infer_foreign_key_name(&self, table_or_model: &str) -> ModelResult<String> {
        let convention = Parent::foreign_key_convention();
        
        match convention {
            ForeignKeyConvention::Underscore => {
                let singular = self.singularize_table_name(table_or_model);
                Ok(format!("{}_id", singular))
            }
            ForeignKeyConvention::CamelCase => {
                let singular = self.singularize_table_name(table_or_model);
                Ok(format!("{}Id", self.to_camel_case(&singular)))
            }
            ForeignKeyConvention::PascalCase => {
                let singular = self.singularize_table_name(table_or_model);
                Ok(format!("{}ID", self.to_pascal_case(&singular)))
            }
            ForeignKeyConvention::Custom(pattern) => {
                let singular = self.singularize_table_name(table_or_model);
                Ok(pattern.replace("{model}", &singular))
            }
        }
    }
    
    /// Infer pivot table name for many-to-many relationships
    fn infer_pivot_table_name(&self, table1: &str, table2: &str) -> String {
        let mut tables = vec![table1, table2];
        tables.sort();
        tables.join("_")
    }
    
    /// Infer polymorphic column names
    fn infer_polymorphic_columns(&self, field_name: &str) -> (String, String) {
        // Standard Laravel-style naming: commentable_type, commentable_id
        let base = if field_name.ends_with("able") {
            field_name.to_string()
        } else {
            format!("{}_able", field_name)
        };
        
        (format!("{}_type", base), format!("{}_id", base))
    }
    
    /// Infer table name from model name
    pub fn infer_table_name(&self, model_name: &str) -> String {
        let convention = Parent::table_naming_convention();
        let base_name = model_name.to_lowercase();
        
        match convention {
            TableNamingConvention::Plural => self.pluralize_name(&base_name),
            TableNamingConvention::Singular => base_name,
            TableNamingConvention::Custom(pattern) => pattern.replace("{model}", &base_name),
        }
    }
    
    /// Simple pluralization (English-centric)
    pub fn pluralize_name(&self, name: &str) -> String {
        if name.ends_with('y') && !name.ends_with("ay") && !name.ends_with("ey") && !name.ends_with("iy") && !name.ends_with("oy") && !name.ends_with("uy") {
            format!("{}ies", &name[..name.len()-1])
        } else if name.ends_with('s') || name.ends_with("sh") || name.ends_with("ch") || name.ends_with('x') || name.ends_with('z') {
            format!("{}es", name)
        } else {
            format!("{}s", name)
        }
    }
    
    /// Simple singularization (English-centric)  
    pub fn singularize_table_name(&self, name: &str) -> String {
        if name.ends_with("ies") {
            format!("{}y", &name[..name.len()-3])
        } else if name.ends_with("ses") || name.ends_with("ches") || name.ends_with("shes") || name.ends_with("xes") || name.ends_with("zes") {
            name[..name.len()-2].to_string()
        } else if name.ends_with('s') && name.len() > 1 {
            name[..name.len()-1].to_string()
        } else {
            name.to_string()
        }
    }
    
    /// Convert to camelCase
    pub fn to_camel_case(&self, s: &str) -> String {
        let parts: Vec<&str> = s.split('_').collect();
        if parts.is_empty() {
            return s.to_string();
        }
        
        let mut result = parts[0].to_lowercase();
        for part in &parts[1..] {
            if !part.is_empty() {
                let mut chars = part.chars();
                if let Some(first) = chars.next() {
                    result.push(first.to_uppercase().next().unwrap());
                    result.extend(chars.map(|c| c.to_lowercase()).flatten());
                }
            }
        }
        
        result
    }
    
    /// Convert to PascalCase
    pub fn to_pascal_case(&self, s: &str) -> String {
        let camel = self.to_camel_case(s);
        let mut chars = camel.chars();
        if let Some(first) = chars.next() {
            first.to_uppercase().collect::<String>() + &chars.collect::<String>()
        } else {
            camel
        }
    }
}

impl<Parent> Default for RelationshipInferenceEngine<Parent>
where
    Parent: InferableModel + DeserializeOwned + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Utility for inferring relationship types from field types
pub struct TypeInferenceHelper;

impl TypeInferenceHelper {
    /// Infer relationship type from Rust type information
    pub fn infer_from_type_name(type_name: &str) -> Option<RelationshipType> {
        if type_name.contains("Option<") {
            // Single optional relationship
            if type_name.contains("Vec<") {
                None // Shouldn't have Option<Vec<T>>
            } else {
                Some(RelationshipType::HasOne) // Default to HasOne for Option<T>
            }
        } else if type_name.contains("Vec<") {
            Some(RelationshipType::HasMany) // Collection relationship
        } else if type_name.contains("MorphOne<") {
            Some(RelationshipType::MorphOne)
        } else if type_name.contains("MorphMany<") {
            Some(RelationshipType::MorphMany)
        } else if type_name.contains("MorphTo<") {
            Some(RelationshipType::MorphTo)
        } else {
            None // Can't infer from basic types
        }
    }
    
    /// Check if a field name suggests a specific relationship type
    pub fn infer_from_field_name(field_name: &str) -> Option<RelationshipType> {
        if field_name.ends_with("_id") {
            Some(RelationshipType::BelongsTo)
        } else if field_name.ends_with("_ids") {
            Some(RelationshipType::ManyToMany)
        } else if field_name.ends_with("able") || field_name.contains("morph") {
            Some(RelationshipType::MorphTo) // Default for polymorphic
        } else {
            None
        }
    }
    
    /// Suggest relationship type based on multiple hints
    pub fn suggest_relationship_type(
        field_name: &str,
        type_name: &str,
        is_collection: bool,
        is_optional: bool,
    ) -> RelationshipType {
        // Try field name inference first
        if let Some(rt) = Self::infer_from_field_name(field_name) {
            return rt;
        }
        
        // Try type name inference
        if let Some(rt) = Self::infer_from_type_name(type_name) {
            return rt;
        }
        
        // Fall back to collection/optional hints
        match (is_collection, is_optional) {
            (true, _) => RelationshipType::HasMany,
            (false, true) => RelationshipType::HasOne,
            (false, false) => RelationshipType::BelongsTo, // Required single relationship
        }
    }
}

/// Macro helper for generating relationship hints
#[macro_export]
macro_rules! relationship_hints {
    ($(($field:expr, $type:expr, $related:expr, $eager:expr)),* $(,)?) => {
        vec![
            $(
                $crate::relationships::inference::RelationshipHint {
                    field_name: $field.to_string(),
                    relationship_type: $type,
                    related_model: $related.to_string(),
                    custom_foreign_key: None,
                    eager_load: $eager,
                }
            ),*
        ]
    };
    
    ($(($field:expr, $type:expr, $related:expr, $eager:expr, $fk:expr)),* $(,)?) => {
        vec![
            $(
                $crate::relationships::inference::RelationshipHint {
                    field_name: $field.to_string(),
                    relationship_type: $type,
                    related_model: $related.to_string(),
                    custom_foreign_key: Some($fk.to_string()),
                    eager_load: $eager,
                }
            ),*
        ]
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::metadata::RelationshipType;
    
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
            relationship_hints![
                ("posts", RelationshipType::HasMany, "Post", false),
                ("profile", RelationshipType::HasOne, "Profile", true),
                ("roles", RelationshipType::ManyToMany, "Role", false)
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
    
    impl InferableModel for TestPost {
        fn relationship_hints() -> Vec<RelationshipHint> {
            relationship_hints![
                ("user", RelationshipType::BelongsTo, "User", true)
            ]
        }
    }
    
    #[test]
    fn test_inference_engine_creation() {
        let mut engine = RelationshipInferenceEngine::<TestUser>::new();
        
        // Test that we can create and use the engine
        let relationships = engine.infer_all_relationships().unwrap();
        
        assert_eq!(relationships.len(), 3);
        assert_eq!(relationships[0].name, "posts");
        assert_eq!(relationships[0].relationship_type, RelationshipType::HasMany);
        assert_eq!(relationships[1].name, "profile");
        assert_eq!(relationships[1].relationship_type, RelationshipType::HasOne);
        assert!(relationships[1].eager_load);
    }
    
    #[test]
    fn test_foreign_key_inference() {
        let engine = RelationshipInferenceEngine::<TestUser>::new();
        
        let fk = engine.infer_foreign_key_name("user").unwrap();
        assert_eq!(fk, "user_id");
        
        let fk = engine.infer_foreign_key_name("posts").unwrap();
        assert_eq!(fk, "post_id");
    }
    
    #[test]
    fn test_pluralization() {
        let engine = RelationshipInferenceEngine::<TestUser>::new();
        
        assert_eq!(engine.pluralize_name("user"), "users");
        assert_eq!(engine.pluralize_name("post"), "posts");
        assert_eq!(engine.pluralize_name("category"), "categories");
        assert_eq!(engine.pluralize_name("box"), "boxes");
    }
    
    #[test]
    fn test_singularization() {
        let engine = RelationshipInferenceEngine::<TestUser>::new();
        
        assert_eq!(engine.singularize_table_name("users"), "user");
        assert_eq!(engine.singularize_table_name("posts"), "post");
        assert_eq!(engine.singularize_table_name("categories"), "category");
        assert_eq!(engine.singularize_table_name("boxes"), "box");
    }
    
    #[test]
    fn test_case_conversion() {
        let engine = RelationshipInferenceEngine::<TestUser>::new();
        
        assert_eq!(engine.to_camel_case("user_id"), "userId");
        assert_eq!(engine.to_camel_case("user"), "user");
        assert_eq!(engine.to_pascal_case("user_id"), "UserId");
        assert_eq!(engine.to_pascal_case("user"), "User");
    }
    
    #[test]
    fn test_type_inference_helper() {
        assert_eq!(
            TypeInferenceHelper::infer_from_type_name("Option<Post>"),
            Some(RelationshipType::HasOne)
        );
        assert_eq!(
            TypeInferenceHelper::infer_from_type_name("Vec<Post>"),
            Some(RelationshipType::HasMany)
        );
        assert_eq!(
            TypeInferenceHelper::infer_from_field_name("user_id"),
            Some(RelationshipType::BelongsTo)
        );
        assert_eq!(
            TypeInferenceHelper::infer_from_field_name("role_ids"),
            Some(RelationshipType::ManyToMany)
        );
    }
    
    #[test]
    fn test_relationship_type_suggestion() {
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