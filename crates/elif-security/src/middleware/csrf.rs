//! CSRF (Cross-Site Request Forgery) protection middleware
//!
//! Placeholder implementation - will be completed in Phase 3.2

pub use crate::config::CsrfConfig;

/// CSRF middleware (placeholder for Phase 3.2)
#[derive(Debug, Clone)]
pub struct CsrfMiddleware {
    _config: CsrfConfig,
}

impl CsrfMiddleware {
    /// Create new CSRF middleware with configuration
    pub fn new(config: CsrfConfig) -> Self {
        Self { _config: config }
    }
}