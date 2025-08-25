//! Specialized Relationship Types - Type aliases for common relationship patterns
//!
//! Provides convenient type aliases for the most common relationship types:
//! HasOne, HasMany, BelongsTo, and ManyToMany.

use super::core::TypeSafeRelationship;

/// HasOne relationship - holds Option<T> for optional single related model
pub type HasOne<T> = TypeSafeRelationship<Option<T>>;

/// HasMany relationship - holds Vec<T> for collection of related models  
pub type HasMany<T> = TypeSafeRelationship<Vec<T>>;

/// BelongsTo relationship - holds Option<T> for optional parent model
pub type BelongsTo<T> = TypeSafeRelationship<Option<T>>;

/// ManyToMany relationship - holds Vec<T> for many-to-many collection
pub type ManyToMany<T> = TypeSafeRelationship<Vec<T>>;
