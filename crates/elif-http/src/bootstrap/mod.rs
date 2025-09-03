//! Bootstrap engine for automatic module discovery and server setup
//!
//! This module provides the core bootstrap functionality that enables the
//! Laravel/NestJS-style one-line app initialization:
//!
//! ```rust
//! #[elif::main]
//! async fn main() -> Result<(), ElifError> {
//!     AppModule::bootstrap().listen("127.0.0.1:3000").await
//! }
//! ```

pub mod app_module;
pub mod engine;

pub use app_module::*;
pub use engine::*;