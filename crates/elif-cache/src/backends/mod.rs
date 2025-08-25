//! Cache backend implementations

pub mod memory;

#[cfg(feature = "redis-backend")]
pub mod redis;

pub use memory::*;

#[cfg(feature = "redis-backend")]
pub use redis::*;
