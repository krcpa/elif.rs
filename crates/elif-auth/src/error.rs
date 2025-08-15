//! Authentication and authorization error types

use thiserror::Error;
use serde::{Deserialize, Serialize};

/// Authentication and authorization errors
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthError {
    /// Invalid credentials provided
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Token-related errors
    #[error("Token error: {message}")]
    TokenError { message: String },

    /// Session-related errors  
    #[error("Session error: {message}")]
    SessionError { message: String },

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// User account is disabled
    #[error("User account is disabled")]
    UserDisabled,

    /// Account is locked due to failed attempts
    #[error("Account locked due to failed login attempts")]
    AccountLocked,

    /// Multi-factor authentication required
    #[error("Multi-factor authentication required")]
    MfaRequired,

    /// Invalid MFA code
    #[error("Invalid multi-factor authentication code")]
    InvalidMfaCode,

    /// Authorization/permission errors
    #[error("Access denied: {message}")]
    AccessDenied { message: String },

    /// Role not found
    #[error("Role not found: {role}")]
    RoleNotFound { role: String },

    /// Permission not found
    #[error("Permission not found: {permission}")]
    PermissionNotFound { permission: String },

    /// Configuration errors
    #[error("Authentication configuration error: {message}")]
    ConfigurationError { message: String },

    /// Cryptographic errors
    #[error("Cryptographic error: {message}")]
    CryptographicError { message: String },

    /// Database errors
    #[error("Database error during authentication: {message}")]
    DatabaseError { message: String },

    /// Generic authentication error
    #[error("Authentication error: {message}")]
    Generic { message: String },
}

impl AuthError {
    /// Get the error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            AuthError::InvalidCredentials => "INVALID_CREDENTIALS",
            AuthError::TokenError { .. } => "TOKEN_ERROR",
            AuthError::SessionError { .. } => "SESSION_ERROR", 
            AuthError::UserNotFound => "USER_NOT_FOUND",
            AuthError::UserDisabled => "USER_DISABLED",
            AuthError::AccountLocked => "ACCOUNT_LOCKED",
            AuthError::MfaRequired => "MFA_REQUIRED",
            AuthError::InvalidMfaCode => "INVALID_MFA_CODE",
            AuthError::AccessDenied { .. } => "ACCESS_DENIED",
            AuthError::RoleNotFound { .. } => "ROLE_NOT_FOUND",
            AuthError::PermissionNotFound { .. } => "PERMISSION_NOT_FOUND",
            AuthError::ConfigurationError { .. } => "CONFIGURATION_ERROR",
            AuthError::CryptographicError { .. } => "CRYPTOGRAPHIC_ERROR",
            AuthError::DatabaseError { .. } => "DATABASE_ERROR",
            AuthError::Generic { .. } => "AUTHENTICATION_ERROR",
        }
    }

    /// Get HTTP status code for the error
    pub fn status_code(&self) -> u16 {
        match self {
            AuthError::InvalidCredentials => 401,
            AuthError::TokenError { .. } => 401,
            AuthError::SessionError { .. } => 401,
            AuthError::UserNotFound => 401, // Don't reveal user existence
            AuthError::UserDisabled => 401,
            AuthError::AccountLocked => 429, // Too Many Requests
            AuthError::MfaRequired => 202,   // Accepted, but MFA needed
            AuthError::InvalidMfaCode => 401,
            AuthError::AccessDenied { .. } => 403,
            AuthError::RoleNotFound { .. } => 403,
            AuthError::PermissionNotFound { .. } => 403,
            AuthError::ConfigurationError { .. } => 500,
            AuthError::CryptographicError { .. } => 500,
            AuthError::DatabaseError { .. } => 500,
            AuthError::Generic { .. } => 500,
        }
    }

    /// Create a token error
    pub fn token_error(message: impl Into<String>) -> Self {
        Self::TokenError { message: message.into() }
    }

    /// Create a session error
    pub fn session_error(message: impl Into<String>) -> Self {
        Self::SessionError { message: message.into() }
    }

    /// Create an access denied error
    pub fn access_denied(message: impl Into<String>) -> Self {
        Self::AccessDenied { message: message.into() }
    }

    /// Create a configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError { message: message.into() }
    }

    /// Create a cryptographic error
    pub fn crypto_error(message: impl Into<String>) -> Self {
        Self::CryptographicError { message: message.into() }
    }

    /// Create a database error
    pub fn database_error(message: impl Into<String>) -> Self {
        Self::DatabaseError { message: message.into() }
    }

    /// Create a generic error
    pub fn generic_error(message: impl Into<String>) -> Self {
        Self::Generic { message: message.into() }
    }
    
    /// Create an authentication failed error (alias for InvalidCredentials with message)
    pub fn authentication_failed(message: impl Into<String>) -> Self {
        Self::Generic { message: format!("Authentication failed: {}", message.into()) }
    }
    
    /// Create a configuration error (alias for config_error)
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::config_error(message)
    }
}

// Conversion from common error types
#[cfg(feature = "jwt")]
impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::token_error(err.to_string())
    }
}

#[cfg(feature = "argon2")]
impl From<argon2::Error> for AuthError {
    fn from(err: argon2::Error) -> Self {
        Self::crypto_error(err.to_string())
    }
}

#[cfg(feature = "bcrypt")]
impl From<bcrypt::BcryptError> for AuthError {
    fn from(err: bcrypt::BcryptError) -> Self {
        Self::crypto_error(err.to_string())
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(err: sqlx::Error) -> Self {
        Self::database_error(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(AuthError::InvalidCredentials.error_code(), "INVALID_CREDENTIALS");
        assert_eq!(AuthError::token_error("test").error_code(), "TOKEN_ERROR");
        assert_eq!(AuthError::access_denied("test").error_code(), "ACCESS_DENIED");
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(AuthError::InvalidCredentials.status_code(), 401);
        assert_eq!(AuthError::access_denied("test").status_code(), 403);
        assert_eq!(AuthError::AccountLocked.status_code(), 429);
        assert_eq!(AuthError::MfaRequired.status_code(), 202);
        assert_eq!(AuthError::config_error("test").status_code(), 500);
    }

    #[test]
    fn test_error_creation_helpers() {
        let token_err = AuthError::token_error("Invalid token");
        assert_eq!(token_err, AuthError::TokenError { message: "Invalid token".to_string() });

        let access_err = AuthError::access_denied("No permission");
        assert_eq!(access_err, AuthError::AccessDenied { message: "No permission".to_string() });
    }

    #[test]
    fn test_error_display() {
        let err = AuthError::token_error("JWT expired");
        assert_eq!(err.to_string(), "Token error: JWT expired");

        let err = AuthError::access_denied("Insufficient privileges");
        assert_eq!(err.to_string(), "Access denied: Insufficient privileges");
    }
}