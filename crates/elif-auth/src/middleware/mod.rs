//! Authentication logic for middleware integration
//!
//! This module provides authentication logic that can be integrated into HTTP middleware

pub mod guards;
pub mod jwt;
pub mod session;

// Re-exports for convenient access
pub use guards::{AuthGuard, AuthGuardConfig, OptionalAuth, RequireAuth};
pub use jwt::{JwtMiddleware, JwtMiddlewareBuilder, JwtMiddlewareConfig};
pub use session::{
    CookieSameSite, SessionMiddleware, SessionMiddlewareBuilder, SessionMiddlewareConfig,
};
