//! Password hashing and cryptographic utilities

use crate::{AuthError, AuthResult, PasswordHasher};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::collections::HashMap;

#[cfg(feature = "argon2")]
use argon2::{
    password_hash::{PasswordHash, PasswordHasher as _, PasswordVerifier, SaltString},
    Argon2,
};

#[cfg(feature = "bcrypt")]
use bcrypt::{hash, verify, DEFAULT_COST};

/// Argon2 password hasher implementation
#[cfg(feature = "argon2")]
#[derive(Debug, Clone)]
pub struct Argon2Hasher {
    memory_cost: u32,
    time_cost: u32,
    parallelism: u32,
}

#[cfg(feature = "argon2")]
impl Argon2Hasher {
    /// Create a new Argon2 hasher with custom parameters
    pub fn new(memory_cost: u32, time_cost: u32, parallelism: u32) -> Self {
        Self {
            memory_cost,
            time_cost,
            parallelism,
        }
    }

    /// Create an Argon2 hasher with default parameters
    pub fn default() -> Self {
        Self {
            memory_cost: 65536, // 64 MB
            time_cost: 3,       // 3 iterations
            parallelism: 4,     // 4 threads
        }
    }

    /// Create an Argon2 hasher optimized for production
    pub fn production() -> Self {
        Self {
            memory_cost: 65536, // 64 MB
            time_cost: 4,       // 4 iterations
            parallelism: 4,     // 4 threads
        }
    }

    /// Create an Argon2 hasher optimized for development (faster)
    pub fn development() -> Self {
        Self {
            memory_cost: 4096, // 4 MB
            time_cost: 2,      // 2 iterations
            parallelism: 2,    // 2 threads
        }
    }
}

#[cfg(feature = "argon2")]
impl PasswordHasher for Argon2Hasher {
    fn hash_password(&self, password: &str) -> AuthResult<String> {
        let salt = SaltString::generate(&mut thread_rng());
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                self.memory_cost,
                self.time_cost,
                self.parallelism,
                None,
            ).map_err(|e| AuthError::crypto_error(e.to_string()))?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::crypto_error(e.to_string()))?;

        Ok(password_hash.to_string())
    }

    fn verify_password(&self, password: &str, hash: &str) -> AuthResult<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AuthError::crypto_error(e.to_string()))?;

        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                self.memory_cost,
                self.time_cost,
                self.parallelism,
                None,
            ).map_err(|e| AuthError::crypto_error(e.to_string()))?,
        );

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn hasher_name(&self) -> &str {
        "argon2"
    }
}

/// bcrypt password hasher implementation
#[cfg(feature = "bcrypt")]
#[derive(Debug, Clone)]
pub struct BcryptHasher {
    cost: u32,
}

#[cfg(feature = "bcrypt")]
impl BcryptHasher {
    /// Create a new bcrypt hasher with custom cost
    pub fn new(cost: u32) -> Self {
        Self { cost }
    }

    /// Create a bcrypt hasher with default cost
    pub fn default() -> Self {
        Self { cost: DEFAULT_COST }
    }

    /// Create a bcrypt hasher optimized for production
    pub fn production() -> Self {
        Self { cost: 12 }
    }

    /// Create a bcrypt hasher optimized for development (faster)
    pub fn development() -> Self {
        Self { cost: 4 }
    }
}

#[cfg(feature = "bcrypt")]
impl PasswordHasher for BcryptHasher {
    fn hash_password(&self, password: &str) -> AuthResult<String> {
        hash(password, self.cost).map_err(AuthError::from)
    }

    fn verify_password(&self, password: &str, hash: &str) -> AuthResult<bool> {
        verify(password, hash).map_err(AuthError::from)
    }

    fn hasher_name(&self) -> &str {
        "bcrypt"
    }
}

/// Password hasher factory for creating different hashers
pub struct PasswordHasherFactory;

impl PasswordHasherFactory {
    /// Create a password hasher by name
    pub fn create_hasher(
        algorithm: &str,
        config: HashMap<String, serde_json::Value>,
    ) -> AuthResult<Box<dyn PasswordHasher>> {
        match algorithm {
            #[cfg(feature = "argon2")]
            "argon2" => {
                let memory_cost = config
                    .get("memory_cost")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(65536) as u32;
                let time_cost = config
                    .get("time_cost")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(3) as u32;
                let parallelism = config
                    .get("parallelism")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4) as u32;

                Ok(Box::new(Argon2Hasher::new(memory_cost, time_cost, parallelism)))
            }
            #[cfg(feature = "bcrypt")]
            "bcrypt" => {
                use bcrypt::DEFAULT_COST;
                let cost = config
                    .get("cost")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(DEFAULT_COST as u64) as u32;

                Ok(Box::new(BcryptHasher::new(cost)))
            }
            _ => Err(AuthError::config_error(format!(
                "Unknown password hashing algorithm: {} (or feature not enabled)",
                algorithm
            ))),
        }
    }

    /// Create default hasher (Argon2)
    pub fn default_hasher() -> Box<dyn PasswordHasher> {
        #[cfg(feature = "argon2")]
        return Box::new(Argon2Hasher::default());
        
        #[cfg(all(not(feature = "argon2"), feature = "bcrypt"))]
        return Box::new(BcryptHasher::default());
        
        #[cfg(all(not(feature = "argon2"), not(feature = "bcrypt")))]
        panic!("No password hasher available. Enable either 'argon2' or 'bcrypt' feature");
    }
}

/// Utility functions for generating random values
pub struct CryptoUtils;

impl CryptoUtils {
    /// Generate a random string of specified length using alphanumeric characters
    pub fn generate_random_string(length: usize) -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }

    /// Generate a secure random token
    pub fn generate_token(length: usize) -> String {
        Self::generate_random_string(length)
    }

    /// Generate backup codes for MFA
    pub fn generate_backup_codes(count: usize, length: usize) -> Vec<String> {
        (0..count)
            .map(|_| Self::generate_random_string(length))
            .collect()
    }

    /// Generate a cryptographically secure session ID
    pub fn generate_session_id() -> String {
        Self::generate_token(32)
    }

    /// Generate a TOTP secret key
    pub fn generate_totp_secret() -> String {
        Self::generate_token(32)
    }

    /// Generate a JWT secret key
    pub fn generate_jwt_secret(length: Option<usize>) -> String {
        Self::generate_token(length.unwrap_or(64))
    }

    /// Validate password strength based on policy
    pub fn validate_password_strength(
        password: &str,
        min_length: usize,
        max_length: usize,
        require_uppercase: bool,
        require_lowercase: bool,
        require_numbers: bool,
        require_special: bool,
    ) -> AuthResult<()> {
        if password.len() < min_length {
            return Err(AuthError::generic_error(format!(
                "Password must be at least {} characters long",
                min_length
            )));
        }

        if password.len() > max_length {
            return Err(AuthError::generic_error(format!(
                "Password must be at most {} characters long",
                max_length
            )));
        }

        if require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(AuthError::generic_error(
                "Password must contain at least one uppercase letter".to_string(),
            ));
        }

        if require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err(AuthError::generic_error(
                "Password must contain at least one lowercase letter".to_string(),
            ));
        }

        if require_numbers && !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(AuthError::generic_error(
                "Password must contain at least one number".to_string(),
            ));
        }

        if require_special && !password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)) {
            return Err(AuthError::generic_error(
                "Password must contain at least one special character".to_string(),
            ));
        }

        Ok(())
    }

    /// Hash a password with default settings
    pub fn hash_password(password: &str) -> AuthResult<String> {
        let hasher = PasswordHasherFactory::default_hasher();
        hasher.hash_password(password)
    }

    /// Verify a password against its hash
    pub fn verify_password(password: &str, hash: &str) -> AuthResult<bool> {
        let hasher = PasswordHasherFactory::default_hasher();
        hasher.verify_password(password, hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2_hasher() {
        let hasher = Argon2Hasher::default();
        let password = "test_password_123";
        
        let hash = hasher.hash_password(password).unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password);
        
        assert!(hasher.verify_password(password, &hash).unwrap());
        assert!(!hasher.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_bcrypt_hasher() {
        let hasher = BcryptHasher::development(); // Use low cost for tests
        let password = "test_password_123";
        
        let hash = hasher.hash_password(password).unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password);
        
        assert!(hasher.verify_password(password, &hash).unwrap());
        assert!(!hasher.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_password_hasher_factory() {
        let mut config = HashMap::new();
        config.insert("cost".to_string(), serde_json::Value::Number(serde_json::Number::from(4)));
        
        let hasher = PasswordHasherFactory::create_hasher("bcrypt", config).unwrap();
        assert_eq!(hasher.hasher_name(), "bcrypt");
        
        let mut config = HashMap::new();
        config.insert("memory_cost".to_string(), serde_json::Value::Number(serde_json::Number::from(4096)));
        
        let hasher = PasswordHasherFactory::create_hasher("argon2", config).unwrap();
        assert_eq!(hasher.hasher_name(), "argon2");
        
        let result = PasswordHasherFactory::create_hasher("invalid", HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_crypto_utils_random_generation() {
        let token1 = CryptoUtils::generate_token(16);
        let token2 = CryptoUtils::generate_token(16);
        
        assert_eq!(token1.len(), 16);
        assert_eq!(token2.len(), 16);
        assert_ne!(token1, token2);
        
        let session_id = CryptoUtils::generate_session_id();
        assert_eq!(session_id.len(), 32);
        
        let backup_codes = CryptoUtils::generate_backup_codes(5, 8);
        assert_eq!(backup_codes.len(), 5);
        assert!(backup_codes.iter().all(|code| code.len() == 8));
    }

    #[test]
    fn test_password_strength_validation() {
        // Valid password
        let result = CryptoUtils::validate_password_strength(
            "Test123!",
            8,
            128,
            true,
            true,
            true,
            true,
        );
        assert!(result.is_ok());

        // Too short
        let result = CryptoUtils::validate_password_strength(
            "Test1!",
            8,
            128,
            true,
            true,
            true,
            true,
        );
        assert!(result.is_err());

        // Missing uppercase
        let result = CryptoUtils::validate_password_strength(
            "test123!",
            8,
            128,
            true,
            true,
            true,
            true,
        );
        assert!(result.is_err());

        // Missing special character
        let result = CryptoUtils::validate_password_strength(
            "Test1234",
            8,
            128,
            true,
            true,
            true,
            true,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_default_password_operations() {
        let password = "test_password_123";
        
        let hash = CryptoUtils::hash_password(password).unwrap();
        assert!(!hash.is_empty());
        
        assert!(CryptoUtils::verify_password(password, &hash).unwrap());
        assert!(!CryptoUtils::verify_password("wrong", &hash).unwrap());
    }
}