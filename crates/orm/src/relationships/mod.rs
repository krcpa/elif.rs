//! Relationships Module - Complete relationship system with eager loading

pub mod belongs_to;
pub mod cache;
pub mod eager_loading;
pub mod has_many;
pub mod has_one;
pub mod loader;
pub mod traits;

// Phase 6.2.1: Relationship Metadata System
pub mod metadata;
pub mod registry;
pub mod types;

// Phase 6.2.3: Constraint System
pub mod constraints;

// Phase 6.2.5: Type-Safe Relationship Loading
pub mod containers;
pub mod hydration;
pub mod inference;
pub mod type_safe_eager_loading;

#[cfg(test)]
pub mod eager_loading_tests;

#[cfg(test)]
pub mod type_safe_tests;

#[cfg(test)]
pub mod lazy_loading_tests;

// Re-export main types (minimal exports to avoid conflicts)
pub use eager_loading::EagerLoader;
pub use loader::{RelationshipCache, RelationshipLoader};
pub use traits as relationship_traits;

// Re-export metadata system types
pub use metadata::{RelationshipConstraint, RelationshipMetadata, RelationshipType};
pub use registry::RelationshipRegistry;

// Re-export constraint system types
pub use constraints::{ConstraintType, RelationshipConstraintBuilder};

// Re-export type-safe relationship types
// Use the new modular containers
pub use containers::{
    type_safe_utils, BelongsTo, HasMany, HasOne, ManyToMany, MorphMany, MorphOne,
    RelationshipContainer, RelationshipLoadingState, TypeSafeRelationship,
    TypeSafeRelationshipLoader,
};
pub use hydration::*;
pub use inference::*;
pub use type_safe_eager_loading::*;
