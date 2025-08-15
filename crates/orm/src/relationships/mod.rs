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

#[cfg(test)]
pub mod eager_loading_tests;

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