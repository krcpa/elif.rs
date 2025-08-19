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

// Prelude-style re-exports for core functionality
// Only export what actually exists to avoid conflicts

// Error handling
pub use error::AuthError;

// Core authentication traits
pub use traits::{Authenticatable, AuthProvider, UserContext};

// Configuration (only existing types)
pub use config::{AuthConfig, JwtConfig, SessionConfig, PasswordConfig, MfaConfig, AuthRateLimitConfig};

// Providers (minimal specific exports)
pub use providers::jwt::JwtProvider;
pub use providers::session::SessionProvider;

// RBAC system (only existing types)
pub use rbac::{Role, Permission, UserRole};

// Note: utils module contains implementation details, not exported
/// Authentication result type alias
pub type AuthResult<T> = Result<T, AuthError>;

/// Authentication system version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");