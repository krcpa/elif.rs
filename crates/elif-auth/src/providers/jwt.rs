//! JWT (JSON Web Token) authentication provider
//!
//! Provides JWT token generation, validation, and refresh capabilities

#[cfg(feature = "jwt")]
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    config::JwtConfig,
    traits::{AuthProvider, Authenticatable, AuthenticationResult, UserContext},
    AuthError, AuthResult,
};

/// JWT token structure  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtToken {
    /// The raw JWT token string
    pub token: String,

    /// Token expiration time
    pub expires_at: DateTime<Utc>,

    /// Optional refresh token
    pub refresh_token: Option<String>,
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,

    /// Username/email
    pub username: String,

    /// User roles
    pub roles: Vec<String>,

    /// User permissions  
    pub permissions: Vec<String>,

    /// Issued at timestamp
    pub iat: i64,

    /// Expiration timestamp
    pub exp: i64,

    /// Not before timestamp
    pub nbf: i64,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: Option<String>,

    /// JWT ID
    pub jti: String,

    /// Token type (access/refresh)
    pub token_type: String,

    /// Additional user data
    pub user_data: HashMap<String, serde_json::Value>,
}

/// JWT authentication provider
pub struct JwtProvider<User> {
    /// JWT configuration
    config: JwtConfig,

    /// Encoding key for signing tokens
    #[cfg(feature = "jwt")]
    encoding_key: EncodingKey,

    /// Decoding key for verifying tokens
    #[cfg(feature = "jwt")]
    decoding_key: DecodingKey,

    /// JWT header configuration
    #[cfg(feature = "jwt")]
    header: Header,

    /// JWT validation configuration  
    #[cfg(feature = "jwt")]
    validation: Validation,

    /// User type marker
    _marker: std::marker::PhantomData<User>,
}

impl<User> JwtProvider<User> {
    /// Create a new JWT provider
    #[cfg(feature = "jwt")]
    pub fn new(config: JwtConfig) -> AuthResult<Self> {
        // Parse algorithm
        let algorithm = Self::parse_algorithm(&config.algorithm)?;

        // Create keys based on algorithm
        let (encoding_key, decoding_key) = Self::create_keys(&config.secret, &algorithm)?;

        // Create header
        let header = Header::new(algorithm);

        // Create validation config
        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[&config.issuer]);
        if let Some(ref audience) = config.audience {
            validation.set_audience(&[audience]);
        }

        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            header,
            validation,
            _marker: std::marker::PhantomData,
        })
    }

    /// Create a new JWT provider (fallback when jwt feature is disabled)
    #[cfg(not(feature = "jwt"))]
    pub fn new(_config: JwtConfig) -> AuthResult<Self> {
        Err(AuthError::generic_error(
            "JWT support not enabled. Enable the 'jwt' feature",
        ))
    }

    /// Parse algorithm string to jsonwebtoken Algorithm
    #[cfg(feature = "jwt")]
    fn parse_algorithm(algorithm: &str) -> AuthResult<Algorithm> {
        match algorithm {
            "HS256" => Ok(Algorithm::HS256),
            "HS384" => Ok(Algorithm::HS384),
            "HS512" => Ok(Algorithm::HS512),
            "RS256" => Ok(Algorithm::RS256),
            "RS384" => Ok(Algorithm::RS384),
            "RS512" => Ok(Algorithm::RS512),
            _ => Err(AuthError::configuration_error(format!(
                "Unsupported JWT algorithm: {}",
                algorithm
            ))),
        }
    }

    /// Create encoding and decoding keys
    #[cfg(feature = "jwt")]
    fn create_keys(secret: &str, algorithm: &Algorithm) -> AuthResult<(EncodingKey, DecodingKey)> {
        match algorithm {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                // HMAC algorithms use shared secret
                let encoding_key = EncodingKey::from_secret(secret.as_bytes());
                let decoding_key = DecodingKey::from_secret(secret.as_bytes());
                Ok((encoding_key, decoding_key))
            }
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
                // RSA algorithms need key files (for now, return an error with instructions)
                Err(AuthError::configuration_error(
                    "RSA algorithms require private/public key files. Use HS256/HS384/HS512 for shared secret authentication"
                ))
            }
            _ => Err(AuthError::configuration_error("Unsupported algorithm")),
        }
    }

    /// Generate JWT token for user
    #[cfg(feature = "jwt")]
    pub fn generate_token(&self, user: &User, token_type: &str) -> AuthResult<JwtToken>
    where
        User: Authenticatable,
        User::Id: std::fmt::Display,
    {
        let now = Utc::now();
        let expiry_duration = match token_type {
            "access" => Duration::seconds(self.config.access_token_expiry as i64),
            "refresh" => Duration::seconds(self.config.refresh_token_expiry as i64),
            _ => return Err(AuthError::token_error("Invalid token type")),
        };

        let expires_at = now + expiry_duration;

        let claims = JwtClaims {
            sub: user.id().to_string(),
            username: user.username().to_string(),
            roles: user.roles(),
            permissions: user.permissions(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            nbf: now.timestamp(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: token_type.to_string(),
            user_data: user.additional_data(),
        };

        let token = encode(&self.header, &claims, &self.encoding_key)
            .map_err(|e| AuthError::token_error(format!("Failed to generate JWT token: {}", e)))?;

        Ok(JwtToken {
            token,
            expires_at,
            refresh_token: None,
        })
    }

    /// Generate token pair (access + refresh)
    #[cfg(feature = "jwt")]
    pub fn generate_token_pair(&self, user: &User) -> AuthResult<(JwtToken, JwtToken)>
    where
        User: Authenticatable,
        User::Id: std::fmt::Display,
    {
        let access_token = self.generate_token(user, "access")?;
        let refresh_token = self.generate_token(user, "refresh")?;

        Ok((access_token, refresh_token))
    }

    /// Validate and decode JWT token
    #[cfg(feature = "jwt")]
    pub fn decode_token(&self, token: &str) -> AuthResult<TokenData<JwtClaims>> {
        decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| AuthError::token_error(format!("Invalid JWT token: {}", e)))
    }

    /// Validate token and extract claims
    #[cfg(feature = "jwt")]
    pub fn validate_token_claims(&self, token: &JwtToken) -> AuthResult<JwtClaims> {
        let token_data = self.decode_token(&token.token)?;

        // Check if token has expired
        let now = Utc::now().timestamp();
        if token_data.claims.exp < now {
            return Err(AuthError::token_error("Token has expired"));
        }

        // Check not before
        if token_data.claims.nbf > now {
            return Err(AuthError::token_error("Token not yet valid"));
        }

        Ok(token_data.claims)
    }

    /// Create user context from JWT claims
    pub fn claims_to_user_context(&self, claims: &JwtClaims) -> UserContext {
        let mut context = UserContext::new(
            claims.sub.clone(),
            claims.username.clone(),
            "jwt".to_string(),
        );

        context.roles = claims.roles.clone();
        context.permissions = claims.permissions.clone();
        context.authenticated_at = DateTime::from_timestamp(claims.iat, 0).unwrap_or(Utc::now());
        context.expires_at = Some(DateTime::from_timestamp(claims.exp, 0).unwrap_or(Utc::now()));
        context.additional_data = claims.user_data.clone();

        context
    }

    /// Fallback methods when jwt feature is disabled
    #[cfg(not(feature = "jwt"))]
    pub fn generate_token(&self, _user: &User, _token_type: &str) -> AuthResult<JwtToken>
    where
        User: Authenticatable,
        User::Id: std::fmt::Display,
    {
        Err(AuthError::generic_error("JWT support not enabled"))
    }

    #[cfg(not(feature = "jwt"))]
    pub fn generate_token_pair(&self, _user: &User) -> AuthResult<(JwtToken, JwtToken)>
    where
        User: Authenticatable,
        User::Id: std::fmt::Display,
    {
        Err(AuthError::generic_error("JWT support not enabled"))
    }

    #[cfg(not(feature = "jwt"))]
    pub fn validate_token_claims(&self, _token: &JwtToken) -> AuthResult<JwtClaims> {
        Err(AuthError::generic_error("JWT support not enabled"))
    }
}

/// Simple user credentials for JWT authentication
#[derive(Debug, Clone)]
pub struct JwtCredentials {
    /// Username or email
    pub username: String,
    /// Password
    pub password: String,
}

/// Example user implementation (for testing/demonstration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub is_active: bool,
    pub is_locked: bool,
}

#[async_trait]
impl Authenticatable for JwtUser {
    type Id = String;
    type Credentials = JwtCredentials;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn username(&self) -> &str {
        &self.username
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn is_locked(&self) -> bool {
        self.is_locked
    }

    fn roles(&self) -> Vec<String> {
        self.roles.clone()
    }

    fn permissions(&self) -> Vec<String> {
        self.permissions.clone()
    }

    async fn verify_credentials(&self, credentials: &Self::Credentials) -> AuthResult<bool> {
        // In a real implementation, this would verify the password hash
        // For demo purposes, just check if credentials match expected format
        Ok(credentials.username == self.username && !credentials.password.is_empty())
    }

    fn additional_data(&self) -> HashMap<String, serde_json::Value> {
        let mut data = HashMap::new();
        data.insert(
            "email".to_string(),
            serde_json::Value::String(self.email.clone()),
        );
        data
    }
}

// Implement AuthProvider trait for JWT provider
#[async_trait]
impl<User> AuthProvider<User> for JwtProvider<User>
where
    User: Authenticatable + Send + Sync + 'static,
    User::Credentials: Send + Sync,
{
    type Token = JwtToken;
    type Credentials = User::Credentials;

    async fn authenticate(
        &self,
        _credentials: &Self::Credentials,
    ) -> AuthResult<AuthenticationResult<User, Self::Token>> {
        // Note: In a real implementation, you would:
        // 1. Look up the user by credentials
        // 2. Verify the credentials (password, etc.)
        // 3. Generate tokens for the authenticated user

        // For now, return an error indicating this needs user lookup implementation
        Err(AuthError::authentication_failed(
            "JWT authentication requires user lookup implementation. This provider handles token generation/validation but needs integration with user storage."
        ))
    }

    async fn validate_token(&self, token: &Self::Token) -> AuthResult<User> {
        // Validate the token and extract claims
        let _claims = self.validate_token_claims(token)?;

        // Note: In a real implementation, you would:
        // 1. Use the claims to look up the user from storage
        // 2. Ensure the user still exists and is active
        // 3. Return the user object

        // For now, return an error indicating this needs user storage integration
        Err(AuthError::token_error(
            "Token validation requires user storage integration. Claims are valid but user lookup is not implemented."
        ))
    }

    #[cfg(feature = "jwt")]
    async fn refresh_token(&self, token: &Self::Token) -> AuthResult<Self::Token> {
        if !self.config.allow_refresh {
            return Err(AuthError::token_error("Token refresh not allowed"));
        }

        // Validate the refresh token
        let claims = self.validate_token_claims(token)?;

        if claims.token_type != "refresh" {
            return Err(AuthError::token_error("Invalid token type for refresh"));
        }

        // Note: In a real implementation, you would:
        // 1. Look up the user using the claims
        // 2. Generate a new access token
        // 3. Optionally rotate the refresh token

        Err(AuthError::token_error(
            "Token refresh requires user storage integration",
        ))
    }

    #[cfg(not(feature = "jwt"))]
    async fn refresh_token(&self, _token: &Self::Token) -> AuthResult<Self::Token> {
        Err(AuthError::generic_error("JWT support not enabled"))
    }

    async fn revoke_token(&self, _token: &Self::Token) -> AuthResult<()> {
        // JWT tokens are stateless, so revocation would require:
        // 1. A token blacklist/revocation storage
        // 2. Middleware to check revoked tokens
        // For now, we'll just log and return success
        tracing::info!("Token revocation requested (not implemented - requires blacklist)");
        Ok(())
    }

    fn provider_name(&self) -> &str {
        "jwt"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> JwtConfig {
        JwtConfig {
            secret: "test-secret-key-that-is-long-enough-for-validation".to_string(),
            algorithm: "HS256".to_string(),
            access_token_expiry: 900,     // 15 minutes
            refresh_token_expiry: 604800, // 7 days
            issuer: "test".to_string(),
            audience: Some("test-app".to_string()),
            allow_refresh: true,
        }
    }

    fn create_test_user() -> JwtUser {
        JwtUser {
            id: "123".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read".to_string()],
            is_active: true,
            is_locked: false,
        }
    }

    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_jwt_provider_creation() {
        let config = create_test_config();
        let provider = JwtProvider::<JwtUser>::new(config);
        assert!(provider.is_ok());
    }

    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_token_generation() {
        let config = create_test_config();
        let provider = JwtProvider::<JwtUser>::new(config).unwrap();
        let user = create_test_user();

        let token = provider.generate_token(&user, "access");
        assert!(token.is_ok());

        let token = token.unwrap();
        assert!(!token.token.is_empty());
        assert!(token.expires_at > Utc::now());
    }

    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_token_validation() {
        let config = create_test_config();
        let provider = JwtProvider::<JwtUser>::new(config).unwrap();
        let user = create_test_user();

        let token = provider.generate_token(&user, "access").unwrap();
        let claims = provider.validate_token_claims(&token);
        assert!(claims.is_ok());

        let claims = claims.unwrap();
        assert_eq!(claims.sub, "123");
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.roles, vec!["user"]);
        assert_eq!(claims.token_type, "access");
    }

    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_token_pair_generation() {
        let config = create_test_config();
        let provider = JwtProvider::<JwtUser>::new(config).unwrap();
        let user = create_test_user();

        let result = provider.generate_token_pair(&user);
        assert!(result.is_ok());

        let (access_token, refresh_token) = result.unwrap();
        assert!(!access_token.token.is_empty());
        assert!(!refresh_token.token.is_empty());
        assert_ne!(access_token.token, refresh_token.token);
    }

    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn test_claims_to_user_context() {
        let config = create_test_config();
        let provider = JwtProvider::<JwtUser>::new(config).unwrap();
        let user = create_test_user();

        let token = provider.generate_token(&user, "access").unwrap();
        let claims = provider.validate_token_claims(&token).unwrap();
        let context = provider.claims_to_user_context(&claims);

        assert_eq!(context.user_id, "123");
        assert_eq!(context.username, "testuser");
        assert_eq!(context.auth_provider, "jwt");
        assert_eq!(context.roles, vec!["user"]);
        assert!(context.has_role("user"));
        assert!(!context.has_role("admin"));
    }

    #[tokio::test]
    async fn test_jwt_user_trait_implementation() {
        let user = create_test_user();
        let credentials = JwtCredentials {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        assert_eq!(user.id(), "123");
        assert_eq!(user.username(), "testuser");
        assert!(user.is_active());
        assert!(!user.is_locked());
        assert_eq!(user.roles(), vec!["user"]);
        assert_eq!(user.permissions(), vec!["read"]);

        let verification_result = user.verify_credentials(&credentials).await;
        assert!(verification_result.is_ok());
        assert!(verification_result.unwrap());
    }

    #[tokio::test]
    async fn test_invalid_algorithm() {
        let mut config = create_test_config();
        config.algorithm = "INVALID".to_string();

        #[cfg(feature = "jwt")]
        {
            let provider = JwtProvider::<JwtUser>::new(config);
            assert!(provider.is_err());
        }
    }

    #[tokio::test]
    async fn test_provider_name() {
        let config = create_test_config();

        #[cfg(feature = "jwt")]
        {
            let provider = JwtProvider::<JwtUser>::new(config).unwrap();
            assert_eq!(provider.provider_name(), "jwt");
        }
    }
}
