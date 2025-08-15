//! Relationship Registry - Runtime metadata storage and access system

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use dashmap::DashMap;

use crate::error::{ModelError, ModelResult};
use super::metadata::{RelationshipMetadata, RelationshipType, ForeignKeyConfig};

/// Thread-safe relationship registry for storing and accessing metadata at runtime
#[derive(Debug, Clone)]
pub struct RelationshipRegistry {
    /// Map of model name -> relationship name -> metadata
    relationships: Arc<DashMap<String, HashMap<String, RelationshipMetadata>>>,
    
    /// Reverse lookup: foreign key table -> local table -> relationship metadata
    foreign_key_index: Arc<DashMap<String, HashMap<String, Vec<RelationshipMetadata>>>>,
    
    /// Index of polymorphic relationships by morph name
    polymorphic_index: Arc<DashMap<String, Vec<RelationshipMetadata>>>,
    
    /// Eager loading relationships index
    eager_index: Arc<DashMap<String, Vec<String>>>,
}

impl Default for RelationshipRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationshipRegistry {
    /// Create a new empty relationship registry
    pub fn new() -> Self {
        Self {
            relationships: Arc::new(DashMap::new()),
            foreign_key_index: Arc::new(DashMap::new()),
            polymorphic_index: Arc::new(DashMap::new()),
            eager_index: Arc::new(DashMap::new()),
        }
    }

    /// Register a relationship for a model
    pub fn register(
        &self,
        model_name: &str,
        relationship_name: &str,
        metadata: RelationshipMetadata,
    ) -> ModelResult<()> {
        // Validate metadata before registration
        metadata.validate()?;

        // Insert into main registry
        let mut model_relationships = self.relationships
            .entry(model_name.to_string())
            .or_insert_with(HashMap::new);
        
        model_relationships.insert(relationship_name.to_string(), metadata.clone());

        // Update foreign key index for reverse lookups
        self.update_foreign_key_index(&metadata);

        // Update polymorphic index if applicable
        if metadata.relationship_type.is_polymorphic() {
            if let Some(ref poly_config) = metadata.polymorphic_config {
                let mut poly_relationships = self.polymorphic_index
                    .entry(poly_config.name.clone())
                    .or_insert_with(Vec::new);
                poly_relationships.push(metadata.clone());
            }
        }

        // Update eager loading index
        if metadata.eager_load {
            let mut eager_relationships = self.eager_index
                .entry(model_name.to_string())
                .or_insert_with(Vec::new);
            eager_relationships.push(relationship_name.to_string());
        }

        Ok(())
    }

    /// Get relationship metadata by model and relationship name
    pub fn get(&self, model_name: &str, relationship_name: &str) -> Option<RelationshipMetadata> {
        self.relationships
            .get(model_name)?
            .get(relationship_name)
            .cloned()
    }

    /// Get all relationships for a model
    pub fn get_all_for_model(&self, model_name: &str) -> Option<HashMap<String, RelationshipMetadata>> {
        self.relationships.get(model_name).map(|entry| entry.clone())
    }

    /// Check if a relationship exists
    pub fn has_relationship(&self, model_name: &str, relationship_name: &str) -> bool {
        self.relationships
            .get(model_name)
            .map(|relationships| relationships.contains_key(relationship_name))
            .unwrap_or(false)
    }

    /// Get all relationship names for a model
    pub fn get_relationship_names(&self, model_name: &str) -> Vec<String> {
        self.relationships
            .get(model_name)
            .map(|relationships| relationships.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get eager loading relationships for a model
    pub fn get_eager_relationships(&self, model_name: &str) -> Vec<String> {
        self.eager_index
            .get(model_name)
            .map(|relationships| relationships.clone())
            .unwrap_or_default()
    }

    /// Find relationships that reference a specific foreign table
    pub fn find_by_foreign_table(&self, foreign_table: &str) -> Vec<RelationshipMetadata> {
        let mut results = Vec::new();
        
        for entry in self.relationships.iter() {
            for (_, metadata) in entry.value() {
                if metadata.foreign_key.table == foreign_table || metadata.related_table == foreign_table {
                    results.push(metadata.clone());
                }
            }
        }
        
        results
    }

    /// Find inverse relationships for a given relationship
    pub fn find_inverse_relationships(
        &self,
        model_name: &str,
        relationship_name: &str,
    ) -> Vec<(String, String, RelationshipMetadata)> {
        let Some(metadata) = self.get(model_name, relationship_name) else {
            return Vec::new();
        };

        let mut inverses = Vec::new();

        // Look for relationships in the related model that point back to this model
        if let Some(related_relationships) = self.get_all_for_model(&metadata.related_model) {
            for (rel_name, rel_metadata) in related_relationships {
                if self.is_inverse_relationship(&metadata, &rel_metadata) {
                    inverses.push((metadata.related_model.clone(), rel_name, rel_metadata));
                }
            }
        }

        inverses
    }

    /// Get polymorphic relationships by morph name
    pub fn get_polymorphic_relationships(&self, morph_name: &str) -> Vec<RelationshipMetadata> {
        self.polymorphic_index
            .get(morph_name)
            .map(|relationships| relationships.clone())
            .unwrap_or_default()
    }

    /// Get statistics about the registry
    pub fn stats(&self) -> RegistryStats {
        let total_models = self.relationships.len();
        let total_relationships: usize = self.relationships
            .iter()
            .map(|entry| entry.value().len())
            .sum();
        
        let eager_relationships: usize = self.eager_index
            .iter()
            .map(|entry| entry.value().len())
            .sum();

        let polymorphic_relationships: usize = self.polymorphic_index
            .iter()
            .map(|entry| entry.value().len())
            .sum();

        let relationship_type_counts = self.count_relationship_types();

        RegistryStats {
            total_models,
            total_relationships,
            eager_relationships,
            polymorphic_relationships,
            relationship_type_counts,
        }
    }

    /// Clear all registered relationships
    pub fn clear(&self) {
        self.relationships.clear();
        self.foreign_key_index.clear();
        self.polymorphic_index.clear();
        self.eager_index.clear();
    }

    /// Validate all registered relationships
    pub fn validate_all(&self) -> ModelResult<()> {
        for model_entry in self.relationships.iter() {
            for (relationship_name, metadata) in model_entry.value() {
                metadata.validate().map_err(|e| {
                    ModelError::Configuration(format!(
                        "Validation failed for relationship '{}' in model '{}': {}",
                        relationship_name, model_entry.key(), e
                    ))
                })?;
            }
        }
        Ok(())
    }

    /// Update the foreign key index for efficient reverse lookups
    fn update_foreign_key_index(&self, metadata: &RelationshipMetadata) {
        let foreign_table = &metadata.foreign_key.table;
        let local_table = &metadata.related_table;
        
        let mut foreign_key_relationships = self.foreign_key_index
            .entry(foreign_table.clone())
            .or_insert_with(HashMap::new);
        
        let relationships = foreign_key_relationships
            .entry(local_table.clone())
            .or_insert_with(Vec::new);
        
        relationships.push(metadata.clone());
    }

    /// Check if two relationships are inverses of each other
    fn is_inverse_relationship(&self, rel1: &RelationshipMetadata, rel2: &RelationshipMetadata) -> bool {
        // Basic inverse detection - can be enhanced
        match (rel1.relationship_type, rel2.relationship_type) {
            (RelationshipType::HasOne, RelationshipType::BelongsTo) |
            (RelationshipType::BelongsTo, RelationshipType::HasOne) |
            (RelationshipType::HasMany, RelationshipType::BelongsTo) |
            (RelationshipType::BelongsTo, RelationshipType::HasMany) => {
                // Check if foreign keys match appropriately
                rel1.foreign_key.primary_column() == rel2.foreign_key.primary_column()
            }
            (RelationshipType::ManyToMany, RelationshipType::ManyToMany) => {
                // For many-to-many, check if they use the same pivot table
                match (&rel1.pivot_config, &rel2.pivot_config) {
                    (Some(pivot1), Some(pivot2)) => pivot1.table == pivot2.table,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Count relationships by type
    fn count_relationship_types(&self) -> HashMap<RelationshipType, usize> {
        let mut counts = HashMap::new();
        
        for model_entry in self.relationships.iter() {
            for (_, metadata) in model_entry.value() {
                *counts.entry(metadata.relationship_type).or_insert(0) += 1;
            }
        }
        
        counts
    }
}

/// Statistics about the relationship registry
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_models: usize,
    pub total_relationships: usize,
    pub eager_relationships: usize,
    pub polymorphic_relationships: usize,
    pub relationship_type_counts: HashMap<RelationshipType, usize>,
}

impl RegistryStats {
    /// Get the most common relationship type
    pub fn most_common_relationship_type(&self) -> Option<(RelationshipType, usize)> {
        self.relationship_type_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(rel_type, count)| (*rel_type, *count))
    }

    /// Calculate the percentage of eager relationships
    pub fn eager_relationship_percentage(&self) -> f64 {
        if self.total_relationships == 0 {
            0.0
        } else {
            (self.eager_relationships as f64 / self.total_relationships as f64) * 100.0
        }
    }
}

/// Global registry instance for the application
static GLOBAL_REGISTRY: std::sync::OnceLock<RelationshipRegistry> = std::sync::OnceLock::new();

/// Get the global relationship registry
pub fn global_registry() -> &'static RelationshipRegistry {
    GLOBAL_REGISTRY.get_or_init(RelationshipRegistry::new)
}

/// Convenience macro for registering relationships
#[macro_export]
macro_rules! register_relationship {
    ($model:expr, $name:expr, $metadata:expr) => {
        $crate::relationships::registry::global_registry()
            .register($model, $name, $metadata)
            .expect("Failed to register relationship");
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::metadata::*;

    fn create_test_metadata(name: &str, rel_type: RelationshipType) -> RelationshipMetadata {
        RelationshipMetadata::new(
            rel_type,
            name.to_string(),
            format!("{}_table", name),
            format!("{}Model", name),
            ForeignKeyConfig::simple(format!("{}_id", name), format!("{}_table", name)),
        )
    }

    #[test]
    fn test_registry_creation() {
        let registry = RelationshipRegistry::new();
        assert_eq!(registry.stats().total_models, 0);
        assert_eq!(registry.stats().total_relationships, 0);
    }

    #[test]
    fn test_relationship_registration() {
        let registry = RelationshipRegistry::new();
        let metadata = create_test_metadata("posts", RelationshipType::HasMany);

        assert!(registry.register("User", "posts", metadata.clone()).is_ok());
        assert!(registry.has_relationship("User", "posts"));
        assert_eq!(registry.get("User", "posts"), Some(metadata));
    }

    #[test]
    fn test_relationship_not_found() {
        let registry = RelationshipRegistry::new();
        assert!(!registry.has_relationship("User", "nonexistent"));
        assert!(registry.get("User", "nonexistent").is_none());
    }

    #[test]
    fn test_eager_relationships() {
        let registry = RelationshipRegistry::new();
        let mut metadata = create_test_metadata("profile", RelationshipType::HasOne);
        metadata.eager_load = true;

        registry.register("User", "profile", metadata).unwrap();
        
        let eager_relationships = registry.get_eager_relationships("User");
        assert_eq!(eager_relationships, vec!["profile"]);
    }

    #[test]
    fn test_polymorphic_relationships() {
        let registry = RelationshipRegistry::new();
        let mut metadata = create_test_metadata("comments", RelationshipType::MorphMany);
        metadata.polymorphic_config = Some(PolymorphicConfig::new(
            "commentable".to_string(),
            "commentable_type".to_string(),
            "commentable_id".to_string(),
        ));

        registry.register("Post", "comments", metadata).unwrap();
        
        let poly_relationships = registry.get_polymorphic_relationships("commentable");
        assert_eq!(poly_relationships.len(), 1);
    }

    #[test]
    fn test_find_by_foreign_table() {
        let registry = RelationshipRegistry::new();
        let metadata = create_test_metadata("user", RelationshipType::BelongsTo);

        registry.register("Post", "user", metadata).unwrap();
        
        let found = registry.find_by_foreign_table("user_table");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "user");
    }

    #[test]
    fn test_registry_stats() {
        let registry = RelationshipRegistry::new();
        
        let posts_metadata = create_test_metadata("posts", RelationshipType::HasMany);
        let profile_metadata = create_test_metadata("profile", RelationshipType::HasOne);
        let mut eager_metadata = create_test_metadata("comments", RelationshipType::HasMany);
        eager_metadata.eager_load = true;

        registry.register("User", "posts", posts_metadata).unwrap();
        registry.register("User", "profile", profile_metadata).unwrap();
        registry.register("User", "comments", eager_metadata).unwrap();

        let stats = registry.stats();
        assert_eq!(stats.total_models, 1);
        assert_eq!(stats.total_relationships, 3);
        assert_eq!(stats.eager_relationships, 1);
        
        let most_common = stats.most_common_relationship_type();
        assert_eq!(most_common, Some((RelationshipType::HasMany, 2)));
        
        assert!(stats.eager_relationship_percentage() > 30.0);
    }

    #[test]
    fn test_all_relationships_for_model() {
        let registry = RelationshipRegistry::new();
        
        let posts_metadata = create_test_metadata("posts", RelationshipType::HasMany);
        let profile_metadata = create_test_metadata("profile", RelationshipType::HasOne);

        registry.register("User", "posts", posts_metadata).unwrap();
        registry.register("User", "profile", profile_metadata).unwrap();

        let all_relationships = registry.get_all_for_model("User").unwrap();
        assert_eq!(all_relationships.len(), 2);
        assert!(all_relationships.contains_key("posts"));
        assert!(all_relationships.contains_key("profile"));
    }

    #[test]
    fn test_relationship_names() {
        let registry = RelationshipRegistry::new();
        
        let posts_metadata = create_test_metadata("posts", RelationshipType::HasMany);
        let profile_metadata = create_test_metadata("profile", RelationshipType::HasOne);

        registry.register("User", "posts", posts_metadata).unwrap();
        registry.register("User", "profile", profile_metadata).unwrap();

        let mut names = registry.get_relationship_names("User");
        names.sort();
        assert_eq!(names, vec!["posts", "profile"]);
    }

    #[test]
    fn test_registry_validation() {
        let registry = RelationshipRegistry::new();
        
        // Valid relationship
        let valid_metadata = create_test_metadata("posts", RelationshipType::HasMany);
        registry.register("User", "posts", valid_metadata).unwrap();

        assert!(registry.validate_all().is_ok());
    }

    #[test]
    fn test_registry_clear() {
        let registry = RelationshipRegistry::new();
        let metadata = create_test_metadata("posts", RelationshipType::HasMany);

        registry.register("User", "posts", metadata).unwrap();
        assert_eq!(registry.stats().total_relationships, 1);

        registry.clear();
        assert_eq!(registry.stats().total_relationships, 0);
        assert!(!registry.has_relationship("User", "posts"));
    }
}