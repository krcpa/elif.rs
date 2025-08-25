//! Queue backend implementations

pub mod memory;

#[cfg(feature = "redis-backend")]
pub mod redis;

pub use memory::MemoryBackend;

#[cfg(feature = "redis-backend")]
pub use redis::RedisBackend;
