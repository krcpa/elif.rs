//! Relationships Module - Complete relationship system with eager loading

pub mod traits;
pub mod has_one;
pub mod has_many;
pub mod belongs_to;
pub mod eager_loading;
pub mod loader;
pub mod cache;

// Phase 6.2.1: Relationship Metadata System
pub mod metadata;
pub mod types;
pub mod registry;

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

// Re-export main types
pub use traits::*;
pub use has_one::*;
pub use has_many::*;
pub use belongs_to::*;
pub use eager_loading::*;
pub use loader::*;
pub use cache::*;

// Re-export metadata system types
pub use metadata::*;
pub use types::*;
pub use registry::*;

// Re-export constraint system types
pub use constraints::*;

// Re-export type-safe relationship types
// Use the new modular containers
pub use containers::{
    RelationshipLoadingState,
    RelationshipContainer,
    TypeSafeRelationship,
    HasOne,
    HasMany,
    BelongsTo,
    ManyToMany,
    MorphOne,
    MorphMany,
    TypeSafeRelationshipLoader,
    type_safe_utils,
};
pub use hydration::*;
pub use inference::*;
pub use type_safe_eager_loading::*;