//! Connection Management
//!
//! This module provides connection pool management, health monitoring,
//! and connection lifecycle management.

pub mod health;
pub mod pool;
pub mod statistics;

// Re-export for convenience
pub use health::*;
pub use pool::*;
pub use statistics::*;
