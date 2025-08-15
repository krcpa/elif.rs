//! Authentication providers implementations
//! 
//! This module contains implementations of different authentication providers:
//! - JWT (JSON Web Token) provider
//! - Session-based provider
//! - MFA (Multi-Factor Authentication) provider

pub mod jwt;
pub mod session;
pub mod mfa;

// Re-exports for convenience
pub use jwt::*;
pub use session::*;
pub use mfa::*;