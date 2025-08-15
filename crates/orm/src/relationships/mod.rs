//! Relationships Module - Simple relationship system built on the modular query system

pub mod traits;
pub mod has_one;
pub mod has_many;
pub mod belongs_to;

// Re-export main types
pub use traits::*;
pub use has_one::*;
pub use has_many::*;
pub use belongs_to::*;