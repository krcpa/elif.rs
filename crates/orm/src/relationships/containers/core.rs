//! Core Relationship Container Types - Foundation types for all relationship containers
//!
//! Provides the base types and traits for relationship loading states,
//! container functionality, and type-safe relationship storage.

use std::marker::PhantomData;
use std::fmt::Debug;
use crate::error::{ModelError, ModelResult};
use super::super::metadata::RelationshipMetadata;

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