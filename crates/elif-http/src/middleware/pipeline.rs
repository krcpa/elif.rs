//! Legacy middleware pipeline - REMOVED
//!
//! The legacy middleware pipeline has been replaced with the V2 middleware system.
//! Use `crate::middleware::v2::MiddlewarePipelineV2` instead.
//!
//! The V2 system provides:
//! - Laravel-style `handle(request, next)` pattern
//! - Better composability and error handling
//! - No Axum type exposure in public APIs
//! - Cleaner async execution model
//!
//! Example migration:
//! ```rust
//! // Old (removed):
//! // use elif_http::middleware::MiddlewarePipeline;
//! 
//! // New:
//! use elif_http::middleware::v2::MiddlewarePipelineV2;
//! ```

// Re-export the V2 pipeline as the default
pub use super::v2::MiddlewarePipelineV2 as MiddlewarePipeline;