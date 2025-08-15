//! # elif-auth: Authentication and Authorization for elif.rs
//! 
//! This crate provides comprehensive authentication and authorization capabilities
//! for the elif.rs web framework, including JWT tokens, sessions, RBAC, and MFA.

pub mod config;
pub mod error;
pub mod traits;
pub mod providers;
pub mod middleware;
pub mod utils;
pub mod rbac;

// Re-exports for convenient access
pub use config::*;
pub use error::*;
pub use traits::*;
pub use providers::*;
pub use utils::*;
pub use rbac::*;

/// Authentication result type alias
pub type AuthResult<T> = Result<T, AuthError>;

/// Authentication system version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");