//! HTTP Response Caching Middleware for elif.rs
//!
//! This module provides HTTP response caching middleware that integrates
//! with the elif-http framework. It's only available when the `http-cache`
//! feature is enabled.

#[cfg(feature = "http-cache")]
pub mod response_cache;

#[cfg(feature = "http-cache")]
pub use response_cache::*;

#[cfg(not(feature = "http-cache"))]
compile_error!("The middleware module requires the 'http-cache' feature to be enabled");
