//! Multi-factor authentication provider
//!
//! Provides TOTP (Time-based One-Time Password) functionality and backup codes
//! for enhanced security in the authentication flow.

use crate::{AuthError, AuthResult};
use chrono::{DateTime, Duration, Utc};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "mfa")]
use base32;
#[cfg(feature = "mfa")]
use qrcode::{QrCode, render::unicode};
#[cfg(feature = "mfa")]
use totp_lite::{totp, Sha1};
#[cfg(feature = "mfa")]
use urlencoding;

/// MFA configuration
#[derive(Debug, Clone)]
pub struct MfaConfig {
    /// TOTP time step in seconds (usually 30)
    pub time_step: u64,
    /// Number of time windows to check (for time tolerance)
    pub window_tolerance: u8,
    /// Secret key length in bytes (recommended: 20 for SHA-1)
    pub secret_length: usize,
    /// Issuer name for TOTP URIs
    pub issuer: String,
    /// Number of backup codes to generate
    pub backup_codes_count: usize,
}

impl Default for MfaConfig {
    fn default() -> Self {
        Self {
            time_step: 30,
            window_tolerance: 1,
            secret_length: 20,
            issuer: "elif.rs".to_string(),
            backup_codes_count: 10,
        }
    }
}

/// MFA secret containing TOTP secret and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSecret {
    /// User ID this secret belongs to
    pub user_id: Uuid,
    /// Base32-encoded TOTP secret
    pub secret: String,
    /// Backup codes (hashed)
    pub backup_codes: Vec<String>,
    /// Used backup codes (to prevent reuse)
    pub used_backup_codes: Vec<String>,
    /// MFA setup completion timestamp
    pub setup_completed_at: Option<DateTime<Utc>>,
    /// Last successful verification timestamp
    pub last_verified_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// MFA setup information for user enrollment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaSetup {
    /// QR code as text/ASCII art for terminal display
    pub qr_code: String,
    /// Manual entry key (base32 secret)
    pub manual_key: String,
    /// TOTP URI for QR code
    pub totp_uri: String,
    /// Generated backup codes (plaintext - show only once)
    pub backup_codes: Vec<String>,
}

/// MFA verification result
#[derive(Debug, Clone, PartialEq)]
pub enum MfaVerificationResult {
    /// Verification successful with TOTP
    TotpSuccess,
    /// Verification successful with backup code
    BackupCodeSuccess,
    /// Verification failed
    Failed,
    /// MFA not set up for this user
    NotSetup,
}

/// Multi-factor authentication provider
pub struct MfaProvider {
    config: MfaConfig,
    #[allow(dead_code)]
    secrets: HashMap<Uuid, MfaSecret>,
}

impl MfaProvider {
    /// Create a new MFA provider with default configuration
    pub fn new() -> AuthResult<Self> {
        Self::with_config(MfaConfig::default())
    }

    /// Create a new MFA provider with custom configuration
    pub fn with_config(config: MfaConfig) -> AuthResult<Self> {
        Ok(Self {
            config,
            secrets: HashMap::new(),
        })
    }

    /// Generate a new MFA setup for a user
    pub fn generate_setup(&self, _user_id: Uuid, username: &str) -> AuthResult<MfaSetup> {
        #[cfg(not(feature = "mfa"))]
        {
            let _ = (_user_id, username);
            return Err(AuthError::generic_error("MFA feature not enabled - compile with 'mfa' feature"));
        }

        #[cfg(feature = "mfa")]
        {
            // Generate random secret
            let secret_bytes = self.generate_secret();
            let secret_base32 = base32::encode(base32::Alphabet::Rfc4648 { padding: true }, &secret_bytes);

            // Generate backup codes
            let backup_codes = self.generate_backup_codes();

            // Create TOTP URI
            let totp_uri = format!(
                "otpauth://totp/{}:{}?secret={}&issuer={}&digits=6&period={}",
                urlencoding::encode(&self.config.issuer),
                urlencoding::encode(username),
                secret_base32,
                urlencoding::encode(&self.config.issuer),
                self.config.time_step
            );

            // Generate QR code
            let qr_code = self.generate_qr_code(&totp_uri)?;

            Ok(MfaSetup {
                qr_code,
                manual_key: secret_base32,
                totp_uri,
                backup_codes,
            })
        }
    }

    /// Complete MFA setup for a user by verifying the first TOTP code
    pub fn complete_setup(&mut self, user_id: Uuid, setup: &MfaSetup, totp_code: &str) -> AuthResult<()> {
        #[cfg(not(feature = "mfa"))]
        {
            let _ = (user_id, setup, totp_code);
            return Err(AuthError::generic_error("MFA feature not enabled - compile with 'mfa' feature"));
        }

        #[cfg(feature = "mfa")]
        {
            // Verify the TOTP code
            if !self.verify_totp_code(&setup.manual_key, totp_code)? {
                return Err(AuthError::invalid_credentials("Invalid TOTP code"));
            }

            // Hash backup codes for storage
            let hashed_backup_codes = setup.backup_codes.iter()
                .map(|code| self.hash_backup_code(code))
                .collect::<Result<Vec<_>, _>>()?;

            // Store the secret
            let secret = MfaSecret {
                user_id,
                secret: setup.manual_key.clone(),
                backup_codes: hashed_backup_codes,
                used_backup_codes: Vec::new(),
                setup_completed_at: Some(Utc::now()),
                last_verified_at: Some(Utc::now()),
                created_at: Utc::now(),
            };

            self.secrets.insert(user_id, secret);
            Ok(())
        }
    }

    /// Verify MFA for a user
    pub fn verify_mfa(&mut self, user_id: Uuid, code: &str) -> AuthResult<MfaVerificationResult> {
        #[cfg(not(feature = "mfa"))]
        {
            let _ = (user_id, code);
            return Ok(MfaVerificationResult::NotSetup);
        }

        #[cfg(feature = "mfa")]
        {
            if !self.secrets.contains_key(&user_id) {
                return Ok(MfaVerificationResult::NotSetup);
            }

            // Try TOTP verification first - use a separate scope to avoid borrowing conflicts
            let totp_secret = {
                let secret = self.secrets.get(&user_id).unwrap();
                secret.secret.clone()
            };

            if self.verify_totp_code(&totp_secret, code)? {
                // Update last verified time after verification succeeds
                if let Some(secret) = self.secrets.get_mut(&user_id) {
                    secret.last_verified_at = Some(Utc::now());
                }
                return Ok(MfaVerificationResult::TotpSuccess);
            }

            // Try backup code verification - need to separate the operations to avoid borrowing conflicts
            let backup_code_valid = if let Some(secret) = self.secrets.get_mut(&user_id) {
                // Clone the necessary data to avoid borrowing conflicts
                let backup_codes = secret.backup_codes.clone();
                let used_codes = secret.used_backup_codes.clone();
                
                let code_hash = self.hash_backup_code(code)?;
                
                // Check if this backup code exists and hasn't been used
                backup_codes.contains(&code_hash) && !used_codes.contains(&code_hash)
            } else {
                false
            };

            if backup_code_valid {
                // Compute hash before mutable borrow
                let code_hash = self.hash_backup_code(code)?;
                
                // Update the secret after verification
                if let Some(secret) = self.secrets.get_mut(&user_id) {
                    secret.used_backup_codes.push(code_hash);
                    secret.last_verified_at = Some(Utc::now());
                }
                return Ok(MfaVerificationResult::BackupCodeSuccess);
            }

            Ok(MfaVerificationResult::Failed)
        }
    }

    /// Check if MFA is enabled for a user
    pub fn is_mfa_enabled(&self, user_id: Uuid) -> bool {
        self.secrets.get(&user_id)
            .map(|secret| secret.setup_completed_at.is_some())
            .unwrap_or(false)
    }

    /// Disable MFA for a user
    pub fn disable_mfa(&mut self, user_id: Uuid) -> AuthResult<bool> {
        Ok(self.secrets.remove(&user_id).is_some())
    }

    /// Get remaining backup codes count for a user
    pub fn get_remaining_backup_codes_count(&self, user_id: Uuid) -> AuthResult<usize> {
        match self.secrets.get(&user_id) {
            Some(secret) => {
                // Use saturating_sub to avoid potential underflow if used codes exceed total
                let remaining = secret
                    .backup_codes
                    .len()
                    .saturating_sub(secret.used_backup_codes.len());
                Ok(remaining)
            }
            None => Ok(0),
        }
    }

    /// Generate new backup codes (invalidates old ones)
    pub fn regenerate_backup_codes(&mut self, user_id: Uuid) -> AuthResult<Vec<String>> {
        #[cfg(not(feature = "mfa"))]
        {
            let _ = user_id;
            return Err(AuthError::generic_error("MFA feature not enabled - compile with 'mfa' feature"));
        }

        #[cfg(feature = "mfa")]
        {
            if !self.secrets.contains_key(&user_id) {
                return Err(AuthError::not_found("MFA not setup for user"));
            }

            // Generate new backup codes
            let new_backup_codes = self.generate_backup_codes();
            let hashed_codes = new_backup_codes.iter()
                .map(|code| self.hash_backup_code(code))
                .collect::<Result<Vec<_>, _>>()?;

            // Replace old backup codes
            if let Some(secret) = self.secrets.get_mut(&user_id) {
                secret.backup_codes = hashed_codes;
                secret.used_backup_codes.clear();
            }

            Ok(new_backup_codes)
        }
    }

    // Private helper methods

    #[cfg(feature = "mfa")]
    fn generate_secret(&self) -> Vec<u8> {
        let mut secret = vec![0u8; self.config.secret_length];
        thread_rng().fill(&mut secret[..]);
        secret
    }

    #[cfg(feature = "mfa")]
    fn generate_backup_codes(&self) -> Vec<String> {
        let mut codes = Vec::with_capacity(self.config.backup_codes_count);
        let mut rng = thread_rng();
        
        for _ in 0..self.config.backup_codes_count {
            // Generate 8-digit backup code
            let code = format!("{:08}", rng.gen_range(10000000..99999999));
            codes.push(code);
        }
        
        codes
    }

    #[cfg(feature = "mfa")]
    fn generate_qr_code(&self, totp_uri: &str) -> AuthResult<String> {
        let qr_code = QrCode::new(totp_uri)
            .map_err(|e| AuthError::generic_error(&format!("Failed to generate QR code: {}", e)))?;
        
        let qr_string = qr_code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();
            
        Ok(qr_string)
    }

    #[cfg(feature = "mfa")]
    fn verify_totp_code(&self, secret_base32: &str, code: &str) -> AuthResult<bool> {
        // Decode the base32 secret
        let secret = base32::decode(base32::Alphabet::Rfc4648 { padding: true }, secret_base32)
            .ok_or_else(|| AuthError::generic_error("Invalid secret format"))?;

        // Validate code format (should be 6 digits)
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(AuthError::invalid_credentials("Invalid TOTP code format"));
        }

        let current_time = Utc::now().timestamp() as u64;
        
        // Check current time window and tolerance windows
        for i in 0..=(self.config.window_tolerance * 2) {
            let time_offset = (i as i64) - (self.config.window_tolerance as i64);
            let time_window = ((current_time as i64) + (time_offset * self.config.time_step as i64)) as u64;
            
            // Generate TOTP code for this time window
            let expected_code = totp::<Sha1>(&secret, time_window);
            
            // The standard totp function returns 8 digits, but we need 6
            // So we need to take the last 6 digits to match standard authenticator apps
            let expected_code_6digits = if expected_code.len() >= 6 {
                &expected_code[expected_code.len()-6..]
            } else {
                &expected_code
            };
            
            if expected_code_6digits == code {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    #[cfg(feature = "mfa")]
    fn verify_backup_code(&mut self, secret: &mut MfaSecret, code: &str) -> AuthResult<bool> {
        let code_hash = self.hash_backup_code(code)?;
        
        // Check if this backup code exists and hasn't been used
        if secret.backup_codes.contains(&code_hash) && !secret.used_backup_codes.contains(&code_hash) {
            secret.used_backup_codes.push(code_hash);
            return Ok(true);
        }
        
        Ok(false)
    }

    #[cfg(feature = "mfa")]
    fn hash_backup_code(&self, code: &str) -> AuthResult<String> {
        // Simple hash for backup codes - in production, use proper hashing
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        code.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }
}

impl Default for MfaProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create default MFA provider")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mfa_provider_creation() {
        let provider = MfaProvider::new();
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_mfa_config_defaults() {
        let config = MfaConfig::default();
        assert_eq!(config.time_step, 30);
        assert_eq!(config.window_tolerance, 1);
        assert_eq!(config.secret_length, 20);
        assert_eq!(config.issuer, "elif.rs");
        assert_eq!(config.backup_codes_count, 10);
    }

    #[tokio::test]
    async fn test_mfa_provider_with_custom_config() {
        let config = MfaConfig {
            time_step: 60,
            window_tolerance: 2,
            secret_length: 32,
            issuer: "test-app".to_string(),
            backup_codes_count: 12,
        };

        let provider = MfaProvider::with_config(config.clone());
        assert!(provider.is_ok());
        
        let provider = provider.unwrap();
        assert_eq!(provider.config.time_step, 60);
        assert_eq!(provider.config.issuer, "test-app");
    }

    #[cfg(feature = "mfa")]
    #[tokio::test]
    async fn test_mfa_setup_generation() {
        let provider = MfaProvider::new().unwrap();
        let user_id = Uuid::new_v4();
        let username = "testuser";

        let setup = provider.generate_setup(user_id, username);
        assert!(setup.is_ok());

        let setup = setup.unwrap();
        assert!(!setup.qr_code.is_empty());
        assert!(!setup.manual_key.is_empty());
        assert!(setup.totp_uri.contains("otpauth://totp/"));
        assert!(setup.totp_uri.contains(username));
        assert_eq!(setup.backup_codes.len(), 10);
    }

    #[tokio::test]
    async fn test_mfa_not_enabled_by_default() {
        let mut provider = MfaProvider::new().unwrap();
        let user_id = Uuid::new_v4();
        
        assert!(!provider.is_mfa_enabled(user_id));
        
        let result = provider.verify_mfa(user_id, "123456");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MfaVerificationResult::NotSetup);
    }

    #[tokio::test]
    async fn test_mfa_disable() {
        let mut provider = MfaProvider::new().unwrap();
        let user_id = Uuid::new_v4();
        
        // Initially no MFA setup
        let disabled = provider.disable_mfa(user_id).unwrap();
        assert!(!disabled);
        
        // TODO: Test with actual MFA setup once setup is working
    }

    #[tokio::test]
    async fn test_backup_codes_count() {
        let provider = MfaProvider::new().unwrap();
        let user_id = Uuid::new_v4();

        let count = provider.get_remaining_backup_codes_count(user_id);
        assert!(count.is_ok());
        assert_eq!(count.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_backup_codes_count_underflow() {
        let mut provider = MfaProvider::new().unwrap();
        let user_id = Uuid::new_v4();

        // Simulate corrupted state where used codes exceed total backup codes
        provider.secrets.insert(user_id, MfaSecret {
            user_id,
            secret: "dummy".to_string(),
            backup_codes: Vec::new(),
            used_backup_codes: vec!["used".to_string()],
            setup_completed_at: None,
            last_verified_at: None,
            created_at: Utc::now(),
        });

        let count = provider.get_remaining_backup_codes_count(user_id);
        assert!(count.is_ok());
        assert_eq!(count.unwrap(), 0);
    }

    #[cfg(not(feature = "mfa"))]
    #[tokio::test]
    async fn test_mfa_disabled_error_messages() {
        let provider = MfaProvider::new().unwrap();
        let user_id = Uuid::new_v4();
        
        let setup_result = provider.generate_setup(user_id, "testuser");
        assert!(setup_result.is_err());
        assert!(setup_result.unwrap_err().to_string().contains("MFA feature not enabled"));
    }

    #[cfg(feature = "mfa")]
    #[tokio::test]
    async fn test_backup_code_generation() {
        let provider = MfaProvider::new().unwrap();
        let codes = provider.generate_backup_codes();
        
        assert_eq!(codes.len(), 10);
        for code in &codes {
            assert_eq!(code.len(), 8);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
        
        // All codes should be unique
        let unique_codes: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique_codes.len(), codes.len());
    }

    #[cfg(feature = "mfa")]
    #[tokio::test]
    async fn test_secret_generation() {
        let provider = MfaProvider::new().unwrap();
        let secret1 = provider.generate_secret();
        let secret2 = provider.generate_secret();
        
        assert_eq!(secret1.len(), 20);
        assert_eq!(secret2.len(), 20);
        assert_ne!(secret1, secret2); // Should generate different secrets
    }

    #[cfg(feature = "mfa")]
    #[tokio::test]
    async fn test_totp_uri_format() {
        let provider = MfaProvider::new().unwrap();
        let user_id = Uuid::new_v4();
        let username = "test@example.com";

        let setup = provider.generate_setup(user_id, username).unwrap();
        
        assert!(setup.totp_uri.starts_with("otpauth://totp/"));
        assert!(setup.totp_uri.contains("elif.rs"));
        assert!(setup.totp_uri.contains("test%40example.com")); // URL encoded
        assert!(setup.totp_uri.contains("digits=6"));
        assert!(setup.totp_uri.contains("period=30"));
    }
}