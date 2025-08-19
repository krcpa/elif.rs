//! Authentication logic for middleware integration
//! 
//! This module provides authentication logic that can be integrated into HTTP middleware

pub mod jwt;
pub mod session;
pub mod guards;

// Re-exports for convenient access
pub use jwt::{JwtMiddleware, JwtMiddlewareBuilder, JwtMiddlewareConfig};
pub use session::{SessionMiddleware, SessionMiddlewareBuilder, SessionMiddlewareConfig, CookieSameSite};
pub use guards::{RequireAuth, OptionalAuth, AuthGuard, AuthGuardConfig};

