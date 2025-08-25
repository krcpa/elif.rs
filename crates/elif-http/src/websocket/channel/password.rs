//! Secure password hashing for channel authentication
//!
//! This module provides production-ready password hashing using Argon2id,
//! the winner of the Password Hashing Competition and recommended by OWASP.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::fmt;

/// Secure password hash using Argon2id
#[derive(Debug, Clone, PartialEq)]
pub struct SecurePasswordHash(String);

/// Errors that can occur during password hashing operations
#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("Failed to hash password: {0}")]
    HashError(String),
    #[error("Failed to verify password: {0}")]
    VerifyError(String),
    #[error("Invalid password hash format")]
    InvalidHash,
}

impl SecurePasswordHash {
    /// Hash a plaintext password using Argon2id with secure defaults
    ///
    /// Uses Argon2id variant which provides resistance against both
    /// side-channel and GPU-based attacks.
    pub fn hash_password(password: &str) -> Result<Self, PasswordError> {
        let salt = SaltString::generate(&mut OsRng);

        // Use Argon2id with OWASP recommended parameters
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| PasswordError::HashError(e.to_string()))?;

        Ok(Self(password_hash.to_string()))
    }

    /// Verify a plaintext password against this hash
    pub fn verify_password(&self, password: &str) -> Result<bool, PasswordError> {
        let parsed_hash = PasswordHash::new(&self.0).map_err(|_| PasswordError::InvalidHash)?;

        let argon2 = Argon2::default();

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(PasswordError::VerifyError(e.to_string())),
        }
    }

    /// Get the hash string (for storage)
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create from a stored hash string
    pub fn from_hash_string(hash: String) -> Result<Self, PasswordError> {
        // Validate the hash format
        PasswordHash::new(&hash).map_err(|_| PasswordError::InvalidHash)?;

        Ok(Self(hash))
    }
}

impl fmt::Display for SecurePasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SecurePasswordHash> for String {
    fn from(hash: SecurePasswordHash) -> String {
        hash.0
    }
}

impl TryFrom<String> for SecurePasswordHash {
    type Error = PasswordError;

    fn try_from(hash: String) -> Result<Self, Self::Error> {
        Self::from_hash_string(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "secure_password_123!";

        // Hash the password
        let hash = SecurePasswordHash::hash_password(password).unwrap();

        // Verify correct password
        assert!(hash.verify_password(password).unwrap());

        // Verify incorrect password
        assert!(!hash.verify_password("wrong_password").unwrap());
    }

    #[test]
    fn test_hash_uniqueness() {
        let password = "same_password";

        let hash1 = SecurePasswordHash::hash_password(password).unwrap();
        let hash2 = SecurePasswordHash::hash_password(password).unwrap();

        // Hashes should be different due to random salts
        assert_ne!(hash1.as_str(), hash2.as_str());

        // But both should verify the same password
        assert!(hash1.verify_password(password).unwrap());
        assert!(hash2.verify_password(password).unwrap());
    }

    #[test]
    fn test_hash_string_conversion() {
        let password = "test_password";
        let hash = SecurePasswordHash::hash_password(password).unwrap();

        let hash_string = hash.to_string();
        let reconstructed = SecurePasswordHash::from_hash_string(hash_string).unwrap();

        assert!(reconstructed.verify_password(password).unwrap());
    }

    #[test]
    fn test_invalid_hash_format() {
        let result = SecurePasswordHash::from_hash_string("invalid_hash".to_string());
        assert!(matches!(result, Err(PasswordError::InvalidHash)));
    }
}
