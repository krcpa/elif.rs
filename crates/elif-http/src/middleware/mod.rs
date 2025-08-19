//! # Middleware
//!
//! Comprehensive middleware system for processing requests and responses.
//! Provides async middleware trait, pipeline composition, and built-in middleware.

pub mod pipeline;
pub mod core;
pub mod utils;
pub mod v2;
pub mod versioning;

// Re-export core middleware functionality
pub use pipeline::*;

// Re-export all core middleware
pub use core::*;

// Re-export utility middleware
pub use utils::*;

// Re-export versioning middleware
pub use versioning::*;

// Legacy middleware system has been removed - use V2 middleware system instead
// All middleware should implement the v2::Middleware trait with handle(request, next) pattern

// Legacy tests removed - see v2.rs for V2 middleware system tests