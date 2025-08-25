//! Type-Safe Relationship Containers - Modular container system for ORM relationships
//!
//! This module provides a comprehensive system for managing relationship data
//! with compile-time type safety, loading state management, and utilities.
//!
//! ## Module Structure
//!
//! - `core` - Foundation types and traits (RelationshipLoadingState, TypeSafeRelationship)
//! - `specialized_types` - Common relationship type aliases (HasOne, HasMany, etc.)
//! - `polymorphic` - Polymorphic relationship support (MorphOne, MorphMany)
//! - `loaders` - Type-safe loading traits
//! - `utils` - Utility functions for container collections

pub mod core;
pub mod loaders;
pub mod polymorphic;
pub mod specialized_types;
pub mod utils;

// Re-export main types
pub use core::{RelationshipContainer, RelationshipLoadingState, TypeSafeRelationship};

pub use specialized_types::{BelongsTo, HasMany, HasOne, ManyToMany};

pub use polymorphic::{MorphMany, MorphOne};

pub use loaders::TypeSafeRelationshipLoader;

// Re-export utility functions under a namespace
pub mod type_safe_utils {
    pub use super::utils::*;
}

#[cfg(test)]
mod tests;
