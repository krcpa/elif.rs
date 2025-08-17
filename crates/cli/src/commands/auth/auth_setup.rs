use elif_core::ElifError;
use std::fs;
use std::path::Path;

use crate::AuthProvider;

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