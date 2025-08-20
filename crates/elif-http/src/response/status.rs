//! HTTP status code utilities

use std::fmt;
use crate::errors::ParseError;

/// Framework-native status code wrapper that hides Axum internals
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElifStatusCode(axum::http::StatusCode);

impl ElifStatusCode {
    // Common status codes as constants
    pub const OK: Self = Self(axum::http::StatusCode::OK);
    pub const CREATED: Self = Self(axum::http::StatusCode::CREATED);
    pub const ACCEPTED: Self = Self(axum::http::StatusCode::ACCEPTED);
    pub const NO_CONTENT: Self = Self(axum::http::StatusCode::NO_CONTENT);
    pub const MOVED_PERMANENTLY: Self = Self(axum::http::StatusCode::MOVED_PERMANENTLY);
    pub const FOUND: Self = Self(axum::http::StatusCode::FOUND);
    pub const SEE_OTHER: Self = Self(axum::http::StatusCode::SEE_OTHER);
    pub const NOT_MODIFIED: Self = Self(axum::http::StatusCode::NOT_MODIFIED);
    pub const BAD_REQUEST: Self = Self(axum::http::StatusCode::BAD_REQUEST);
    pub const UNAUTHORIZED: Self = Self(axum::http::StatusCode::UNAUTHORIZED);
    pub const FORBIDDEN: Self = Self(axum::http::StatusCode::FORBIDDEN);
    pub const NOT_FOUND: Self = Self(axum::http::StatusCode::NOT_FOUND);
    pub const METHOD_NOT_ALLOWED: Self = Self(axum::http::StatusCode::METHOD_NOT_ALLOWED);
    pub const PRECONDITION_FAILED: Self = Self(axum::http::StatusCode::PRECONDITION_FAILED);
    pub const CONFLICT: Self = Self(axum::http::StatusCode::CONFLICT);
    pub const LOCKED: Self = Self(axum::http::StatusCode::LOCKED);
    pub const UNPROCESSABLE_ENTITY: Self = Self(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    pub const REQUEST_TIMEOUT: Self = Self(axum::http::StatusCode::REQUEST_TIMEOUT);
    pub const PAYLOAD_TOO_LARGE: Self = Self(axum::http::StatusCode::PAYLOAD_TOO_LARGE);
    pub const TOO_MANY_REQUESTS: Self = Self(axum::http::StatusCode::TOO_MANY_REQUESTS);
    pub const INTERNAL_SERVER_ERROR: Self = Self(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    pub const NOT_IMPLEMENTED: Self = Self(axum::http::StatusCode::NOT_IMPLEMENTED);
    pub const BAD_GATEWAY: Self = Self(axum::http::StatusCode::BAD_GATEWAY);
    pub const SERVICE_UNAVAILABLE: Self = Self(axum::http::StatusCode::SERVICE_UNAVAILABLE);

    /// Create status code from u16
    pub fn from_u16(src: u16) -> Result<Self, ParseError> {
        axum::http::StatusCode::from_u16(src)
            .map(Self)
            .map_err(ParseError::from)
    }

    /// Get status code as u16
    pub fn as_u16(&self) -> u16 {
        self.0.as_u16()
    }

    /// Check if status code is informational (1xx)
    pub fn is_informational(&self) -> bool {
        self.0.is_informational()
    }

    /// Check if status code is success (2xx)
    pub fn is_success(&self) -> bool {
        self.0.is_success()
    }

    /// Check if status code is redirection (3xx)
    pub fn is_redirection(&self) -> bool {
        self.0.is_redirection()
    }

    /// Check if status code is client error (4xx)
    pub fn is_client_error(&self) -> bool {
        self.0.is_client_error()
    }

    /// Check if status code is server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.0.is_server_error()
    }

    /// Internal method to convert to axum StatusCode (for framework internals only)
    pub(crate) fn to_axum(&self) -> axum::http::StatusCode {
        self.0
    }

    /// Internal method to create from axum StatusCode (for framework internals only)
    pub(crate) fn from_axum(status: axum::http::StatusCode) -> Self {
        Self(status)
    }
}

impl From<u16> for ElifStatusCode {
    fn from(src: u16) -> Self {
        Self::from_u16(src).expect("Invalid status code")
    }
}

impl From<ElifStatusCode> for u16 {
    fn from(status: ElifStatusCode) -> u16 {
        status.as_u16()
    }
}

impl fmt::Display for ElifStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}