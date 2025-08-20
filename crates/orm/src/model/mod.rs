//! Model System - Modular model trait system for database entities
//!
//! This module provides a decomposed model system with focused traits for
//! different aspects of model functionality:
//!
//! - `core_trait`: Core Model trait definition
//! - `primary_key`: Primary key types and utilities
//! - `crud_operations`: Create, Read, Update, Delete operations
//! - `query_methods`: Collection and batch query operations
//! - `extensions`: Utility methods and convenience functions
//! - `abstraction`: Database-agnostic operations

pub mod core_trait;
pub mod primary_key;
pub mod crud_operations;
pub mod query_methods;
pub mod extensions;
pub mod abstraction;
pub mod lifecycle;

// Re-export main types and traits for convenience
pub use core_trait::Model;
pub use primary_key::PrimaryKey;
pub use crud_operations::CrudOperations;
pub use query_methods::QueryMethods;
pub use extensions::ModelExtensions;
pub use abstraction::ModelAbstracted;
pub use lifecycle::ModelLifecycle;

// Re-export all traits in a single composite trait for easy importing
/// Composite trait that includes all model functionality
pub trait FullModel: Model + CrudOperations + QueryMethods + ModelExtensions + ModelAbstracted {}

// Implement FullModel for all types that implement the component traits
impl<T> FullModel for T 
where 
    T: Model + CrudOperations + QueryMethods + ModelExtensions + ModelAbstracted 
{}