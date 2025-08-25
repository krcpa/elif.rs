//! Authentication providers implementations
//!
//! This module contains implementations of different authentication providers:
//! - JWT (JSON Web Token) provider
//! - Session-based provider
//! - MFA (Multi-Factor Authentication) provider

pub mod jwt;
pub mod mfa;
pub mod session;

// Re-exports for convenience
pub use jwt::*;
pub use mfa::*;
pub use session::*;
