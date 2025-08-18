//! Polymorphic Relationship Types - Support for polymorphic associations
//!
//! Provides MorphOne and MorphMany relationship types that can associate
//! with multiple different model types through a polymorphic interface.

use std::fmt::Debug;
use super::core::{TypeSafeRelationship, RelationshipContainer};
use super::super::metadata::RelationshipMetadata;

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
    /// Create a new MorphOne relationship
    pub fn new(metadata: RelationshipMetadata) -> Self {
        Self {
            relationship: TypeSafeRelationship::new(metadata),
            morph_type: None,
            morph_id: None,
        }
    }
    
    /// Set the polymorphic type and ID information
    pub fn set_morph_info(&mut self, morph_type: String, morph_id: String) {
        self.morph_type = Some(morph_type);
        self.morph_id = Some(morph_id);
    }
    
    /// Get the polymorphic type
    pub fn morph_type(&self) -> Option<&str> {
        self.morph_type.as_deref()
    }
    
    /// Get the polymorphic ID
    pub fn morph_id(&self) -> Option<&str> {
        self.morph_id.as_deref()
    }
    
    /// Get the loaded data
    pub fn get(&self) -> Option<&Option<T>> {
        self.relationship.get_typed()
    }
    
    /// Check if the relationship is loaded
    pub fn is_loaded(&self) -> bool {
        self.relationship.is_loaded()
    }
    
    /// Set the loaded data
    pub fn set_loaded(&mut self, data: Option<T>) {
        self.relationship.set_loaded(data);
    }
    
    /// Mark as loading
    pub fn set_loading(&mut self) {
        self.relationship.set_loading();
    }
    
    /// Mark as failed with error
    pub fn set_failed(&mut self, error: String) {
        self.relationship.set_failed(error);
    }
    
    /// Get the relationship metadata
    pub fn metadata(&self) -> &RelationshipMetadata {
        self.relationship.metadata()
    }
    
    /// Get the relationship name
    pub fn name(&self) -> &str {
        self.relationship.name()
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
    /// Create a new MorphMany relationship
    pub fn new(metadata: RelationshipMetadata) -> Self {
        Self {
            relationship: TypeSafeRelationship::new(metadata),
            morph_type: None,
        }
    }
    
    /// Set the polymorphic type
    pub fn set_morph_type(&mut self, morph_type: String) {
        self.morph_type = Some(morph_type);
    }
    
    /// Get the polymorphic type
    pub fn morph_type(&self) -> Option<&str> {
        self.morph_type.as_deref()
    }
    
    /// Get the loaded data
    pub fn get(&self) -> Option<&Vec<T>> {
        self.relationship.get_typed()
    }
    
    /// Check if the relationship is loaded
    pub fn is_loaded(&self) -> bool {
        self.relationship.is_loaded()
    }
    
    /// Set the loaded data
    pub fn set_loaded(&mut self, data: Vec<T>) {
        self.relationship.set_loaded(data);
    }
    
    /// Mark as loading
    pub fn set_loading(&mut self) {
        self.relationship.set_loading();
    }
    
    /// Mark as failed with error
    pub fn set_failed(&mut self, error: String) {
        self.relationship.set_failed(error);
    }
    
    /// Get the relationship metadata
    pub fn metadata(&self) -> &RelationshipMetadata {
        self.relationship.metadata()
    }
    
    /// Get the relationship name
    pub fn name(&self) -> &str {
        self.relationship.name()
    }
    
    /// Get the number of loaded items (0 if not loaded)
    pub fn len(&self) -> usize {
        self.get().map(|v| v.len()).unwrap_or(0)
    }
    
    /// Check if the collection is empty (true if not loaded)
    pub fn is_empty(&self) -> bool {
        self.get().map(|v| v.is_empty()).unwrap_or(true)
    }
}