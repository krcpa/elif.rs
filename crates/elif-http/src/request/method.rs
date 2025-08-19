//! HTTP method utilities and wrappers

use std::fmt;
use std::str::FromStr;

/// Framework-native HTTP method wrapper that hides Axum internals
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ElifMethod(axum::http::Method);

impl ElifMethod {
    /// Common HTTP methods as constants
    pub const GET: Self = Self(axum::http::Method::GET);
    pub const POST: Self = Self(axum::http::Method::POST);
    pub const PUT: Self = Self(axum::http::Method::PUT);
    pub const DELETE: Self = Self(axum::http::Method::DELETE);
    pub const PATCH: Self = Self(axum::http::Method::PATCH);
    pub const HEAD: Self = Self(axum::http::Method::HEAD);
    pub const OPTIONS: Self = Self(axum::http::Method::OPTIONS);
    pub const TRACE: Self = Self(axum::http::Method::TRACE);
    pub const CONNECT: Self = Self(axum::http::Method::CONNECT);

    /// Create method from string
    pub fn from_str(method: &str) -> Result<Self, axum::http::method::InvalidMethod> {
        axum::http::Method::from_str(method).map(Self)
    }

    /// Get method as string
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Check if method is safe (GET, HEAD, OPTIONS, TRACE)
    pub fn is_safe(&self) -> bool {
        matches!(self.0, axum::http::Method::GET | axum::http::Method::HEAD | 
                          axum::http::Method::OPTIONS | axum::http::Method::TRACE)
    }

    /// Check if method is idempotent (GET, HEAD, PUT, DELETE, OPTIONS, TRACE)
    pub fn is_idempotent(&self) -> bool {
        matches!(self.0, axum::http::Method::GET | axum::http::Method::HEAD | 
                          axum::http::Method::PUT | axum::http::Method::DELETE |
                          axum::http::Method::OPTIONS | axum::http::Method::TRACE)
    }

    /// Internal method to convert to axum Method (for framework internals only)
    pub(crate) fn to_axum(&self) -> &axum::http::Method {
        &self.0
    }

    /// Internal method to create from axum Method (for framework internals only)
    pub(crate) fn from_axum(method: axum::http::Method) -> Self {
        Self(method)
    }
}

impl FromStr for ElifMethod {
    type Err = axum::http::method::InvalidMethod;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s)
    }
}

impl fmt::Display for ElifMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}