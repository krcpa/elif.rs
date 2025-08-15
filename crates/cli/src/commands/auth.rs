use elif_core::ElifError;
use std::fs;
use std::path::Path;

use crate::{AuthProvider};

pub async fn setup(provider: AuthProvider, mfa: bool, rbac: bool) -> Result<(), ElifError> {
    println!("🔐 Setting up authentication configuration...");
    
    // Check if we're in an elif project
    if !Path::new("Cargo.toml").exists() {
        return Err(ElifError::Validation("Not in an elif project directory".to_string()));
    }
    
    // Create config directory if it doesn't exist
    fs::create_dir_all("config")?;
    
    match provider {
        AuthProvider::Jwt => setup_jwt_auth(mfa, rbac).await?,
        AuthProvider::Session => setup_session_auth(mfa, rbac).await?,
        AuthProvider::Both => {
            setup_jwt_auth(mfa, rbac).await?;
            setup_session_auth(mfa, rbac).await?;
        }
    }
    
    println!("✅ Authentication setup complete!");
    println!();
    println!("Next steps:");
    println!("1. Update your .env file with the authentication configuration");
    println!("2. Run `elifrs auth scaffold` to generate authentication controllers");
    println!("3. Run `elifrs migrate run` to apply authentication database changes");
    
    Ok(())
}

async fn setup_jwt_auth(mfa: bool, rbac: bool) -> Result<(), ElifError> {
    println!("📝 Configuring JWT authentication...");
    
    let mut config = String::from("# JWT Authentication Configuration\n");
    config.push_str("JWT_SECRET=your-secret-key-here  # Run `elifrs auth generate-key` to generate\n");
    config.push_str("JWT_EXPIRES_IN=3600  # 1 hour\n");
    config.push_str("JWT_REFRESH_EXPIRES_IN=604800  # 7 days\n");
    config.push_str("JWT_ALGORITHM=HS256\n");
    config.push_str("JWT_ISSUER=elif-app\n");
    config.push_str("\n");
    
    if mfa {
        config.push_str("# Multi-Factor Authentication\n");
        config.push_str("MFA_ENABLED=true\n");
        config.push_str("MFA_ISSUER=YourApp\n");
        config.push_str("MFA_BACKUP_CODES_COUNT=8\n");
        config.push_str("\n");
    }
    
    if rbac {
        config.push_str("# Role-Based Access Control\n");
        config.push_str("RBAC_ENABLED=true\n");
        config.push_str("\n");
    }
    
    fs::write("config/auth_jwt.env", config)?;
    println!("📄 Created config/auth_jwt.env");
    
    Ok(())
}

async fn setup_session_auth(mfa: bool, rbac: bool) -> Result<(), ElifError> {
    println!("📝 Configuring session authentication...");
    
    let mut config = String::from("# Session Authentication Configuration\n");
    config.push_str("SESSION_SECRET=your-session-secret-here  # Run `elifrs auth generate-key` to generate\n");
    config.push_str("SESSION_NAME=elif_session\n");
    config.push_str("SESSION_EXPIRES_IN=86400  # 24 hours\n");
    config.push_str("SESSION_SECURE=false  # Set to true in production\n");
    config.push_str("SESSION_HTTP_ONLY=true\n");
    config.push_str("SESSION_SAME_SITE=Lax\n");
    config.push_str("\n");
    
    if mfa {
        config.push_str("# Multi-Factor Authentication\n");
        config.push_str("MFA_ENABLED=true\n");
        config.push_str("MFA_ISSUER=YourApp\n");
        config.push_str("MFA_BACKUP_CODES_COUNT=8\n");
        config.push_str("\n");
    }
    
    if rbac {
        config.push_str("# Role-Based Access Control\n");
        config.push_str("RBAC_ENABLED=true\n");
        config.push_str("\n");
    }
    
    fs::write("config/auth_session.env", config)?;
    println!("📄 Created config/auth_session.env");
    
    Ok(())
}

pub async fn generate_key(length: usize) -> Result<(), ElifError> {
    use rand::Rng;
    
    println!("🔑 Generating secure key...");
    
    if length < 32 {
        return Err(ElifError::Validation("Key length must be at least 32 bytes".to_string()));
    }
    
    let mut rng = rand::thread_rng();
    let key: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
    let key_hex = hex::encode(&key);
    
    println!("Generated {}-byte key:", length);
    println!("{}", key_hex);
    println!();
    println!("⚠️  Keep this key secure and never commit it to version control!");
    println!("💡 Add this to your .env file as JWT_SECRET or SESSION_SECRET");
    
    Ok(())
}

pub async fn scaffold(registration: bool, reset_password: bool) -> Result<(), ElifError> {
    println!("🏗️  Generating authentication scaffold...");
    
    // Check if we're in an elif project
    if !Path::new("Cargo.toml").exists() {
        return Err(ElifError::Validation("Not in an elif project directory".to_string()));
    }
    
    // Create necessary directories
    fs::create_dir_all("src/controllers/auth")?;
    fs::create_dir_all("src/models")?;
    fs::create_dir_all("migrations")?;
    
    // Generate User model
    generate_user_model().await?;
    
    // Generate authentication controllers
    generate_auth_controller().await?;
    
    if registration {
        generate_registration_controller().await?;
    }
    
    if reset_password {
        generate_password_reset_controller().await?;
    }
    
    // Generate migrations
    generate_auth_migrations().await?;
    
    println!("✅ Authentication scaffold generated!");
    println!();
    println!("Generated files:");
    println!("- src/models/user.rs");
    println!("- src/controllers/auth/login.rs");
    if registration {
        println!("- src/controllers/auth/register.rs");
    }
    if reset_password {
        println!("- src/controllers/auth/password_reset.rs");
    }
    println!("- migrations/*_create_users_table.sql");
    println!("- migrations/*_create_user_sessions_table.sql");
    
    Ok(())
}

async fn generate_user_model() -> Result<(), ElifError> {
    let content = r#"use elif_orm::prelude::*;
use elif_auth::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
#[table_name = "users"]
pub struct User {
    #[primary_key]
    pub id: Uuid,
    
    #[unique]
    pub email: String,
    
    pub password_hash: String,
    
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    
    pub email_verified_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    
    // MFA support
    pub mfa_secret: Option<String>,
    pub mfa_backup_codes: Option<String>, // JSON array
    pub mfa_enabled: bool,
    
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Default for User {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email: String::new(),
            password_hash: String::new(),
            first_name: None,
            last_name: None,
            email_verified_at: None,
            is_active: true,
            mfa_secret: None,
            mfa_backup_codes: None,
            mfa_enabled: false,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }
}

// <<<ELIF:BEGIN agent-editable:user-jwt-trait>>>
impl JwtUser for User {
    fn get_id(&self) -> String {
        self.id.to_string()
    }
    
    fn get_email(&self) -> String {
        self.email.clone()
    }
    
    fn get_roles(&self) -> Vec<String> {
        // TODO: Implement role fetching from database
        vec![]
    }
    
    fn get_permissions(&self) -> Vec<String> {
        // TODO: Implement permission fetching from database
        vec![]
    }
}
// <<<ELIF:END agent-editable:user-jwt-trait>>>

// <<<ELIF:BEGIN agent-editable:user-session-trait>>>
impl SessionUser for User {
    fn get_session_id(&self) -> String {
        self.id.to_string()
    }
    
    fn get_session_data(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "email": self.email,
            "name": format!("{} {}", 
                self.first_name.as_deref().unwrap_or(""), 
                self.last_name.as_deref().unwrap_or("")
            ).trim(),
            "mfa_enabled": self.mfa_enabled
        })
    }
}
// <<<ELIF:END agent-editable:user-session-trait>>>

// <<<ELIF:BEGIN agent-editable:user-methods>>>
impl User {
    pub fn full_name(&self) -> String {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => self.email.clone(),
        }
    }
    
    pub fn is_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }
    
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, ModelError> {
        Self::query()
            .where_eq("email", email)
            .first()
            .await
    }
    
    pub async fn create_with_password(
        email: String,
        password: &str,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<Self, ModelError> {
        use elif_auth::utils::hash_password;
        
        let password_hash = hash_password(password)
            .map_err(|_| ModelError::validation("Failed to hash password"))?;
        
        let user = Self {
            email,
            password_hash,
            first_name,
            last_name,
            ..Default::default()
        };
        
        user.save().await
    }
    
    pub fn verify_password(&self, password: &str) -> bool {
        use elif_auth::utils::verify_password;
        verify_password(password, &self.password_hash).unwrap_or(false)
    }
}
// <<<ELIF:END agent-editable:user-methods>>>
"#;
    
    fs::write("src/models/user.rs", content)?;
    println!("📄 Created src/models/user.rs");
    Ok(())
}

async fn generate_auth_controller() -> Result<(), ElifError> {
    let content = r#"use elif_http::prelude::*;
use elif_auth::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::User;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub token: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub email_verified: bool,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email.clone(),
            name: user.full_name(),
            email_verified: user.is_verified(),
        }
    }
}

// <<<ELIF:BEGIN agent-editable:login-handler>>>
pub async fn login(
    Json(request): Json<LoginRequest>,
    jwt_provider: Extension<Arc<JwtProvider>>,
) -> Result<Json<ApiResponse<LoginResponse>>, HttpError> {
    // Find user by email
    let user = User::find_by_email(&request.email)
        .await?
        .ok_or_else(|| HttpError::unauthorized("Invalid credentials"))?;
    
    // Verify password
    if !user.verify_password(&request.password) {
        return Err(HttpError::unauthorized("Invalid credentials"));
    }
    
    // Check if account is active
    if !user.is_active {
        return Err(HttpError::unauthorized("Account is disabled"));
    }
    
    // Generate JWT tokens
    let token_pair = jwt_provider.generate_token_pair(&user)?;
    
    let response = LoginResponse {
        user: UserResponse::from(&user),
        token: Some(token_pair.access_token),
        refresh_token: Some(token_pair.refresh_token),
    };
    
    Ok(Json(ApiResponse::success(response)))
}
// <<<ELIF:END agent-editable:login-handler>>>

// <<<ELIF:BEGIN agent-editable:refresh-handler>>>
pub async fn refresh_token(
    bearer_token: BearerToken,
    jwt_provider: Extension<Arc<JwtProvider>>,
) -> Result<Json<ApiResponse<LoginResponse>>, HttpError> {
    // Validate refresh token and get user
    let user_context = jwt_provider.validate_token(&bearer_token.token)?;
    
    // Find current user
    let user = User::find(user_context.user_id.parse().unwrap())
        .await?
        .ok_or_else(|| HttpError::unauthorized("User not found"))?;
    
    // Generate new token pair
    let token_pair = jwt_provider.generate_token_pair(&user)?;
    
    let response = LoginResponse {
        user: UserResponse::from(&user),
        token: Some(token_pair.access_token),
        refresh_token: Some(token_pair.refresh_token),
    };
    
    Ok(Json(ApiResponse::success(response)))
}
// <<<ELIF:END agent-editable:refresh-handler>>>

// <<<ELIF:BEGIN agent-editable:logout-handler>>>
pub async fn logout() -> Result<Json<ApiResponse<()>>, HttpError> {
    // In a real implementation, you might want to:
    // 1. Blacklist the JWT token
    // 2. Clear session data
    // 3. Log the logout event
    
    Ok(Json(ApiResponse::success(())))
}
// <<<ELIF:END agent-editable:logout-handler>>>

// <<<ELIF:BEGIN agent-editable:profile-handler>>>
pub async fn profile(
    user_context: UserContext,
) -> Result<Json<ApiResponse<UserResponse>>, HttpError> {
    let user = User::find(user_context.user_id.parse().unwrap())
        .await?
        .ok_or_else(|| HttpError::unauthorized("User not found"))?;
    
    Ok(Json(ApiResponse::success(UserResponse::from(&user))))
}
// <<<ELIF:END agent-editable:profile-handler>>>
"#;
    
    fs::write("src/controllers/auth/login.rs", content)?;
    println!("📄 Created src/controllers/auth/login.rs");
    Ok(())
}

async fn generate_registration_controller() -> Result<(), ElifError> {
    let content = r#"use elif_http::prelude::*;
use elif_auth::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::User;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: UserResponse,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email.clone(),
            name: user.full_name(),
        }
    }
}

// <<<ELIF:BEGIN agent-editable:register-handler>>>
pub async fn register(
    Json(request): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<RegisterResponse>>, HttpError> {
    // Validate password confirmation
    if request.password != request.password_confirmation {
        return Err(HttpError::validation(vec![
            ("password_confirmation", "Passwords do not match")
        ]));
    }
    
    // Check if email already exists
    if let Some(_) = User::find_by_email(&request.email).await? {
        return Err(HttpError::validation(vec![
            ("email", "Email address is already registered")
        ]));
    }
    
    // Create new user
    let user = User::create_with_password(
        request.email,
        &request.password,
        request.first_name,
        request.last_name,
    ).await?;
    
    let response = RegisterResponse {
        user: UserResponse::from(&user),
        message: "Registration successful. Please verify your email address.".to_string(),
    };
    
    Ok(Json(ApiResponse::success(response)))
}
// <<<ELIF:END agent-editable:register-handler>>>

// <<<ELIF:BEGIN agent-editable:verify-email-handler>>>
pub async fn verify_email(
    Path(token): Path<String>,
) -> Result<Json<ApiResponse<String>>, HttpError> {
    // TODO: Implement email verification logic
    // 1. Validate verification token
    // 2. Find user by token
    // 3. Mark email as verified
    // 4. Return success response
    
    Ok(Json(ApiResponse::success("Email verified successfully".to_string())))
}
// <<<ELIF:END agent-editable:verify-email-handler>>>

// <<<ELIF:BEGIN agent-editable:resend-verification-handler>>>
pub async fn resend_verification(
    Json(email): Json<String>,
) -> Result<Json<ApiResponse<String>>, HttpError> {
    // Find user by email
    let user = User::find_by_email(&email)
        .await?
        .ok_or_else(|| HttpError::not_found("User not found"))?;
    
    if user.is_verified() {
        return Err(HttpError::validation(vec![
            ("email", "Email is already verified")
        ]));
    }
    
    // TODO: Generate and send verification email
    
    Ok(Json(ApiResponse::success("Verification email sent".to_string())))
}
// <<<ELIF:END agent-editable:resend-verification-handler>>>
"#;
    
    fs::write("src/controllers/auth/register.rs", content)?;
    println!("📄 Created src/controllers/auth/register.rs");
    Ok(())
}

async fn generate_password_reset_controller() -> Result<(), ElifError> {
    let content = r#"use elif_http::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::User;

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// <<<ELIF:BEGIN agent-editable:forgot-password-handler>>>
pub async fn forgot_password(
    Json(request): Json<ForgotPasswordRequest>,
) -> Result<Json<ApiResponse<MessageResponse>>, HttpError> {
    // Find user by email
    let user = User::find_by_email(&request.email).await?;
    
    if user.is_none() {
        // Don't reveal if email exists or not for security
        return Ok(Json(ApiResponse::success(MessageResponse {
            message: "If the email address exists, a password reset link has been sent.".to_string(),
        })));
    }
    
    // TODO: Generate password reset token and send email
    // 1. Generate secure reset token
    // 2. Store token with expiration
    // 3. Send reset email
    
    Ok(Json(ApiResponse::success(MessageResponse {
        message: "If the email address exists, a password reset link has been sent.".to_string(),
    })))
}
// <<<ELIF:END agent-editable:forgot-password-handler>>>

// <<<ELIF:BEGIN agent-editable:reset-password-handler>>>
pub async fn reset_password(
    Json(request): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<MessageResponse>>, HttpError> {
    // Validate password confirmation
    if request.password != request.password_confirmation {
        return Err(HttpError::validation(vec![
            ("password_confirmation", "Passwords do not match")
        ]));
    }
    
    // TODO: Implement password reset logic
    // 1. Validate reset token
    // 2. Find user by token
    // 3. Update password
    // 4. Invalidate token
    // 5. Return success response
    
    Ok(Json(ApiResponse::success(MessageResponse {
        message: "Password has been reset successfully.".to_string(),
    })))
}
// <<<ELIF:END agent-editable:reset-password-handler>>>

// <<<ELIF:BEGIN agent-editable:change-password-handler>>>
pub async fn change_password(
    user_context: UserContext,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<MessageResponse>>, HttpError> {
    // Find current user
    let user = User::find(user_context.user_id.parse().unwrap())
        .await?
        .ok_or_else(|| HttpError::unauthorized("User not found"))?;
    
    // Verify current password
    if !user.verify_password(&request.current_password) {
        return Err(HttpError::validation(vec![
            ("current_password", "Current password is incorrect")
        ]));
    }
    
    // Validate new password confirmation
    if request.new_password != request.new_password_confirmation {
        return Err(HttpError::validation(vec![
            ("new_password_confirmation", "Passwords do not match")
        ]));
    }
    
    // TODO: Update password
    // 1. Hash new password
    // 2. Update user record
    // 3. Return success response
    
    Ok(Json(ApiResponse::success(MessageResponse {
        message: "Password changed successfully.".to_string(),
    })))
}
// <<<ELIF:END agent-editable:change-password-handler>>>

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
    pub new_password_confirmation: String,
}
"#;
    
    fs::write("src/controllers/auth/password_reset.rs", content)?;
    println!("📄 Created src/controllers/auth/password_reset.rs");
    Ok(())
}

async fn generate_auth_migrations() -> Result<(), ElifError> {
    use chrono::Utc;
    
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    
    // Users table migration
    let users_migration = r#"-- Create users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    email_verified_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT true,
    mfa_secret VARCHAR(255),
    mfa_backup_codes TEXT, -- JSON array
    mfa_enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

-- Create indexes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_is_active ON users(is_active);
CREATE INDEX idx_users_created_at ON users(created_at);
CREATE INDEX idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NOT NULL;

-- Create updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at 
    BEFORE UPDATE ON users 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
"#;
    
    let users_migration_file = format!("migrations/{}__create_users_table.sql", timestamp);
    fs::write(&users_migration_file, users_migration)?;
    println!("📄 Created {}", users_migration_file);
    
    // User sessions table migration
    let sessions_migration = r#"-- Create user_sessions table for session-based authentication
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token VARCHAR(255) UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    ip_address INET,
    user_agent TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_token ON user_sessions(session_token);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);
CREATE INDEX idx_user_sessions_is_active ON user_sessions(is_active);

-- Create updated_at trigger
CREATE TRIGGER update_user_sessions_updated_at 
    BEFORE UPDATE ON user_sessions 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
"#;
    
    let sessions_migration_file = format!("migrations/{}__create_user_sessions_table.sql", timestamp.parse::<i64>().unwrap_or(0) + 1);
    fs::write(&sessions_migration_file, sessions_migration)?;
    println!("📄 Created {}", sessions_migration_file);
    
    Ok(())
}