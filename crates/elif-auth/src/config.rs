//! Authentication configuration types and utilities

use serde::{Deserialize, Serialize};

/// Main authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    /// JWT configuration
    pub jwt: JwtConfig,

    /// Session configuration
    pub session: SessionConfig,

    /// Password policy configuration
    pub password: PasswordConfig,

    /// Multi-factor authentication configuration
    pub mfa: MfaConfig,

    /// Rate limiting for authentication attempts
    pub rate_limit: AuthRateLimitConfig,
}

/// JWT token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Secret key for JWT signing (HS256) or path to private key (RS256)
    pub secret: String,

    /// JWT signing algorithm (HS256, HS384, HS512, RS256, RS384, RS512)
    #[serde(default = "default_jwt_algorithm")]
    pub algorithm: String,

    /// Access token expiration time in seconds
    #[serde(default = "default_access_token_expiry")]
    pub access_token_expiry: u64,

    /// Refresh token expiration time in seconds  
    #[serde(default = "default_refresh_token_expiry")]
    pub refresh_token_expiry: u64,

    /// JWT issuer
    #[serde(default = "default_jwt_issuer")]
    pub issuer: String,

    /// JWT audience
    pub audience: Option<String>,

    /// Allow token refresh
    #[serde(default = "default_true")]
    pub allow_refresh: bool,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session storage backend (memory, database, redis)
    #[serde(default = "default_session_storage")]
    pub storage: String,

    /// Session expiration time in seconds
    #[serde(default = "default_session_expiry")]
    pub expiry: u64,

    /// Session cookie name
    #[serde(default = "default_session_cookie_name")]
    pub cookie_name: String,

    /// Session cookie domain
    pub cookie_domain: Option<String>,

    /// Session cookie path
    #[serde(default = "default_session_cookie_path")]
    pub cookie_path: String,

    /// Session cookie secure flag
    #[serde(default = "default_false")]
    pub cookie_secure: bool,

    /// Session cookie HTTP-only flag
    #[serde(default = "default_true")]
    pub cookie_http_only: bool,

    /// Session cookie SameSite policy
    #[serde(default = "default_session_cookie_same_site")]
    pub cookie_same_site: String,

    /// Session cleanup interval in seconds
    #[serde(default = "default_session_cleanup_interval")]
    pub cleanup_interval: u64,
}

/// Password policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordConfig {
    /// Minimum password length
    #[serde(default = "default_min_password_length")]
    pub min_length: usize,

    /// Maximum password length
    #[serde(default = "default_max_password_length")]
    pub max_length: usize,

    /// Require uppercase letters
    #[serde(default = "default_true")]
    pub require_uppercase: bool,

    /// Require lowercase letters  
    #[serde(default = "default_true")]
    pub require_lowercase: bool,

    /// Require numbers
    #[serde(default = "default_true")]
    pub require_numbers: bool,

    /// Require special characters
    #[serde(default = "default_false")]
    pub require_special: bool,

    /// Password hashing algorithm (argon2, bcrypt)
    #[serde(default = "default_hash_algorithm")]
    pub hash_algorithm: String,

    /// Bcrypt cost factor (if using bcrypt)
    #[serde(default = "default_bcrypt_cost")]
    pub bcrypt_cost: u32,

    /// Argon2 memory cost in KB (if using argon2)
    #[serde(default = "default_argon2_memory")]
    pub argon2_memory: u32,

    /// Argon2 time cost (iterations)
    #[serde(default = "default_argon2_iterations")]
    pub argon2_iterations: u32,

    /// Argon2 parallelism factor
    #[serde(default = "default_argon2_parallelism")]
    pub argon2_parallelism: u32,
}

/// Multi-factor authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaConfig {
    /// Enable MFA
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// TOTP issuer name
    #[serde(default = "default_totp_issuer")]
    pub totp_issuer: String,

    /// TOTP time step in seconds
    #[serde(default = "default_totp_step")]
    pub totp_step: u64,

    /// TOTP code length
    #[serde(default = "default_totp_digits")]
    pub totp_digits: usize,

    /// TOTP time window tolerance
    #[serde(default = "default_totp_window")]
    pub totp_window: u8,

    /// Number of backup codes to generate
    #[serde(default = "default_backup_codes_count")]
    pub backup_codes_count: usize,

    /// Backup code length
    #[serde(default = "default_backup_code_length")]
    pub backup_code_length: usize,
}

/// Authentication rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRateLimitConfig {
    /// Maximum login attempts per IP
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Time window for rate limiting in seconds
    #[serde(default = "default_rate_limit_window")]
    pub window_seconds: u64,

    /// Lockout duration in seconds after max attempts
    #[serde(default = "default_lockout_duration")]
    pub lockout_duration: u64,
}

// Default value functions
fn default_jwt_algorithm() -> String {
    "HS256".to_string()
}
fn default_access_token_expiry() -> u64 {
    15 * 60
} // 15 minutes
fn default_refresh_token_expiry() -> u64 {
    7 * 24 * 60 * 60
} // 7 days
fn default_jwt_issuer() -> String {
    "elif.rs".to_string()
}
fn default_session_storage() -> String {
    "memory".to_string()
}
fn default_session_expiry() -> u64 {
    24 * 60 * 60
} // 24 hours
fn default_session_cookie_name() -> String {
    "elif_session".to_string()
}
fn default_session_cookie_path() -> String {
    "/".to_string()
}
fn default_session_cookie_same_site() -> String {
    "Lax".to_string()
}
fn default_session_cleanup_interval() -> u64 {
    60 * 60
} // 1 hour
fn default_min_password_length() -> usize {
    8
}
fn default_max_password_length() -> usize {
    128
}
fn default_hash_algorithm() -> String {
    "argon2".to_string()
}
fn default_bcrypt_cost() -> u32 {
    12
}
fn default_argon2_memory() -> u32 {
    65536
} // 64MB
fn default_argon2_iterations() -> u32 {
    3
}
fn default_argon2_parallelism() -> u32 {
    4
}
fn default_totp_issuer() -> String {
    "elif.rs".to_string()
}
fn default_totp_step() -> u64 {
    30
}
fn default_totp_digits() -> usize {
    6
}
fn default_totp_window() -> u8 {
    1
}
fn default_backup_codes_count() -> usize {
    10
}
fn default_backup_code_length() -> usize {
    8
}
fn default_max_attempts() -> u32 {
    5
}
fn default_rate_limit_window() -> u64 {
    15 * 60
} // 15 minutes
fn default_lockout_duration() -> u64 {
    30 * 60
} // 30 minutes
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "default-secret-key-change-in-production-32-chars-long".to_string(), // 32+ chars for validation
            algorithm: default_jwt_algorithm(),
            access_token_expiry: default_access_token_expiry(),
            refresh_token_expiry: default_refresh_token_expiry(),
            issuer: default_jwt_issuer(),
            audience: None,
            allow_refresh: default_true(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            storage: default_session_storage(),
            expiry: default_session_expiry(),
            cookie_name: default_session_cookie_name(),
            cookie_domain: None,
            cookie_path: default_session_cookie_path(),
            cookie_secure: default_false(),
            cookie_http_only: default_true(),
            cookie_same_site: default_session_cookie_same_site(),
            cleanup_interval: default_session_cleanup_interval(),
        }
    }
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            min_length: default_min_password_length(),
            max_length: default_max_password_length(),
            require_uppercase: default_true(),
            require_lowercase: default_true(),
            require_numbers: default_true(),
            require_special: default_false(),
            hash_algorithm: default_hash_algorithm(),
            bcrypt_cost: default_bcrypt_cost(),
            argon2_memory: default_argon2_memory(),
            argon2_iterations: default_argon2_iterations(),
            argon2_parallelism: default_argon2_parallelism(),
        }
    }
}

impl Default for MfaConfig {
    fn default() -> Self {
        Self {
            enabled: default_false(),
            totp_issuer: default_totp_issuer(),
            totp_step: default_totp_step(),
            totp_digits: default_totp_digits(),
            totp_window: default_totp_window(),
            backup_codes_count: default_backup_codes_count(),
            backup_code_length: default_backup_code_length(),
        }
    }
}

impl Default for AuthRateLimitConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            window_seconds: default_rate_limit_window(),
            lockout_duration: default_lockout_duration(),
        }
    }
}

impl AuthConfig {
    /// Create a development configuration with secure defaults
    pub fn development() -> Self {
        let mut config = Self::default();
        config.jwt.secret = "dev-secret-key-change-in-production".to_string();
        config.session.cookie_secure = false; // Allow HTTP in development
        config.password.require_special = false; // Relaxed for dev
        config
    }

    /// Create a production configuration with strict security
    pub fn production() -> Self {
        let mut config = Self::default();
        config.session.cookie_secure = true;
        config.session.cookie_same_site = "Strict".to_string();
        config.password.require_special = true;
        config.password.min_length = 12;
        config.mfa.enabled = true;
        config
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate JWT configuration
        if self.jwt.secret.len() < 32 {
            return Err("JWT secret must be at least 32 characters".to_string());
        }

        if !["HS256", "HS384", "HS512", "RS256", "RS384", "RS512"]
            .contains(&self.jwt.algorithm.as_str())
        {
            return Err("Invalid JWT algorithm".to_string());
        }

        // Validate password policy
        if self.password.min_length > self.password.max_length {
            return Err("Password min_length cannot be greater than max_length".to_string());
        }

        if self.password.min_length < 1 {
            return Err("Password min_length must be at least 1".to_string());
        }

        // Validate session configuration
        if !["memory", "database", "redis"].contains(&self.session.storage.as_str()) {
            return Err("Invalid session storage backend".to_string());
        }

        if !["Strict", "Lax", "None"].contains(&self.session.cookie_same_site.as_str()) {
            return Err("Invalid session cookie SameSite policy".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AuthConfig::default();
        assert_eq!(config.jwt.algorithm, "HS256");
        assert_eq!(config.session.storage, "memory");
        assert_eq!(config.password.hash_algorithm, "argon2");
        assert!(!config.mfa.enabled);
    }

    #[test]
    fn test_development_config() {
        let config = AuthConfig::development();
        assert!(!config.session.cookie_secure);
        assert!(!config.password.require_special);
        assert_eq!(config.jwt.secret, "dev-secret-key-change-in-production");
    }

    #[test]
    fn test_production_config() {
        let config = AuthConfig::production();
        assert!(config.session.cookie_secure);
        assert!(config.password.require_special);
        assert_eq!(config.password.min_length, 12);
        assert!(config.mfa.enabled);
        assert_eq!(config.session.cookie_same_site, "Strict");
    }

    #[test]
    fn test_config_validation() {
        let mut config = AuthConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid JWT secret
        config.jwt.secret = "short".to_string();
        assert!(config.validate().is_err());

        // Test invalid JWT algorithm
        config.jwt.secret = "long-enough-secret-key-for-validation".to_string();
        config.jwt.algorithm = "INVALID".to_string();
        assert!(config.validate().is_err());

        // Test invalid password policy
        config.jwt.algorithm = "HS256".to_string();
        config.password.min_length = 20;
        config.password.max_length = 10;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_durations() {
        let config = AuthConfig::default();
        assert_eq!(config.jwt.access_token_expiry, 15 * 60); // 15 minutes
        assert_eq!(config.jwt.refresh_token_expiry, 7 * 24 * 60 * 60); // 7 days
        assert_eq!(config.session.expiry, 24 * 60 * 60); // 24 hours
    }
}
