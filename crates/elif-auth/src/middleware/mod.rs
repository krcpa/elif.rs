//! Authentication middleware for HTTP requests
//! 
//! Provides middleware for JWT and session-based authentication

pub mod jwt;
pub mod session;

// Re-exports for convenient access
pub use jwt::{JwtMiddleware, JwtMiddlewareBuilder, JwtMiddlewareConfig};
pub use session::{SessionMiddleware, SessionMiddlewareBuilder, SessionMiddlewareConfig, CookieSameSite};

use crate::{AuthError, AuthResult};

/// Generic authentication middleware (placeholder for Phase 5.4)
/// 
/// This will be expanded in Phase 5.4: User Authentication Middleware
pub struct AuthMiddleware {
    // Implementation will be expanded in Phase 5.4
    _placeholder: (),
}

impl AuthMiddleware {
    /// Create a new authentication middleware
    pub fn new() -> AuthResult<Self> {
        // TODO: Expand implementation in Phase 5.4
        Err(AuthError::generic_error("Generic auth middleware not yet implemented - will be completed in Phase 5.4. Use JwtMiddleware for JWT authentication."))
    }
}