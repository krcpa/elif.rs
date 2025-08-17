//! Connection Management
//!
//! This module provides connection pool management, health monitoring,
//! and connection lifecycle management.

pub mod pool;
pub mod health;
pub mod statistics;

// Re-export for convenience
pub use pool::*;
pub use health::*;
pub use statistics::*;