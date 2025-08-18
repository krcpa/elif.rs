//! Relationship Container Utilities - Helper functions for working with containers
//!
//! Provides utility functions for common operations on collections of
//! relationship containers such as extracting loaded data, counting,
//! and bulk state management.

use std::fmt::Debug;
use super::core::{TypeSafeRelationship, RelationshipLoadingState, RelationshipContainer};

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

/// Check if any relationships in a collection are loaded
pub fn any_loaded<T>(relationships: &[TypeSafeRelationship<T>]) -> bool
where
    T: Clone + Debug + Send + Sync,
{
    relationships.iter().any(|rel| rel.is_loaded())
}

/// Reset all relationships to not loaded state
pub fn reset_all<T>(relationships: &mut [TypeSafeRelationship<T>])
where
    T: Clone + Debug + Send + Sync,
{
    for rel in relationships {
        rel.reset();
    }
}

/// Get relationships that need loading (not loaded and not failed)
pub fn get_pending_relationships<T>(relationships: &[TypeSafeRelationship<T>]) -> Vec<&TypeSafeRelationship<T>>
where
    T: Clone + Debug + Send + Sync,
{
    relationships
        .iter()
        .filter(|rel| {
            matches!(rel.loading_state(), RelationshipLoadingState::NotLoaded | RelationshipLoadingState::Loading)
        })
        .collect()
}

/// Group relationships by their loading state
pub fn group_by_state<T>(relationships: &[TypeSafeRelationship<T>]) -> (Vec<&TypeSafeRelationship<T>>, Vec<&TypeSafeRelationship<T>>, Vec<&TypeSafeRelationship<T>>, Vec<&TypeSafeRelationship<T>>)
where
    T: Clone + Debug + Send + Sync,
{
    let mut not_loaded = Vec::new();
    let mut loading = Vec::new();
    let mut loaded = Vec::new();
    let mut failed = Vec::new();
    
    for rel in relationships {
        match rel.loading_state() {
            RelationshipLoadingState::NotLoaded => not_loaded.push(rel),
            RelationshipLoadingState::Loading => loading.push(rel),
            RelationshipLoadingState::Loaded => loaded.push(rel),
            RelationshipLoadingState::Failed(_) => failed.push(rel),
        }
    }
    
    (not_loaded, loading, loaded, failed)
}