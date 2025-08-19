use super::TemplateEngine;
use super::resource_generator::{GeneratedFile, GeneratedFileType};
use elif_core::ElifError;
use std::collections::HashMap;
use std::path::PathBuf;
use serde_json::{json, Value};

#[allow(dead_code)]
pub struct AuthGenerator {
    template_engine: TemplateEngine,
    project_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AuthOptions {
    pub jwt: bool,
    pub session: bool,
    pub mfa: bool,
    pub password_reset: bool,
    pub registration: bool,
    pub rbac: bool,
}

impl Default for AuthOptions {
    fn default() -> Self {
        Self {
            jwt: true,
            session: false,
            mfa: false,
            password_reset: false,
            registration: true,
            rbac: false,
        }
    }
}

impl AuthGenerator {
    pub fn new(project_root: PathBuf) -> Result<Self, ElifError> {
        Ok(Self {
            template_engine: TemplateEngine::new()?,
            project_root,
        })
    }

    pub fn generate_auth_system(&self, options: &AuthOptions) -> Result<Vec<GeneratedFile>, ElifError> {
        let mut generated_files = Vec::new();
        let context = self.build_auth_context(options)?;

        // Generate User model if not exists
        let user_model = self.generate_user_model(&context)?;
        generated_files.push(user_model);

        // Generate authentication controller
        let auth_controller = self.generate_auth_controller(&context)?;
        generated_files.push(auth_controller);

        // Generate authentication requests
        let auth_requests = self.generate_auth_requests(&context)?;
        generated_files.extend(auth_requests);

        // Generate authentication middleware setup
        let auth_middleware = self.generate_auth_middleware(&context)?;
        generated_files.push(auth_middleware);

        // Generate migration for users table
        let user_migration = self.generate_user_migration(&context)?;
        generated_files.push(user_migration);

        // Generate authentication tests
        let auth_tests = self.generate_auth_tests(&context)?;
        generated_files.push(auth_tests);

        if options.mfa {
            let mfa_files = self.generate_mfa_files(&context)?;
            generated_files.extend(mfa_files);
        }

        if options.rbac {
            let rbac_files = self.generate_rbac_files(&context)?;
            generated_files.extend(rbac_files);
        }

        Ok(generated_files)
    }

    fn build_auth_context(&self, options: &AuthOptions) -> Result<HashMap<String, Value>, ElifError> {
        let mut context = HashMap::new();

        context.insert("jwt".to_string(), json!(options.jwt));
        context.insert("session".to_string(), json!(options.session));
        context.insert("mfa".to_string(), json!(options.mfa));
        context.insert("password_reset".to_string(), json!(options.password_reset));
        context.insert("registration".to_string(), json!(options.registration));
        context.insert("rbac".to_string(), json!(options.rbac));

        Ok(context)
    }

    fn generate_user_model(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let rbac = context.get("rbac").unwrap().as_bool().unwrap_or(false);

        let content = format!(
            r#"use elif_orm::prelude::*;
use elif_auth::prelude::*;
use serde::{{Serialize, Deserialize}};
use chrono::{{DateTime, Utc}};
use uuid::Uuid;

#[derive(Model, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[table_name = "users"]
pub struct User {{
    #[primary_key]
    pub id: Uuid,
    
    pub name: String,
    
    #[index]
    pub email: String,
    
    pub email_verified_at: Option<DateTime<Utc>>,
    
    pub password: String,
    
    pub remember_token: Option<String>,
    
    {}
    
    #[timestamp]
    pub created_at: DateTime<Utc>,
    
    #[timestamp]
    pub updated_at: DateTime<Utc>,
}}

impl JwtUser for User {{
    fn id(&self) -> String {{
        self.id.to_string()
    }}
    
    fn email(&self) -> &str {{
        &self.email
    }}
    
    fn additional_claims(&self) -> std::collections::HashMap<String, serde_json::Value> {{
        let mut claims = std::collections::HashMap::new();
        claims.insert("name".to_string(), serde_json::json!(self.name));
        {}
        claims
    }}
}}

impl User {{
    // <<<ELIF:BEGIN agent-editable:user-model-methods>>>
    
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, ModelError> {{
        User::query()
            .where_eq("email", email)
            .first()
            .await
    }}
    
    pub fn verify_password(&self, password: &str) -> Result<bool, AuthError> {{
        // Use password hasher from elif-auth
        PasswordHasher::verify(password, &self.password)
    }}
    
    pub fn hash_password(password: &str) -> Result<String, AuthError> {{
        // Use password hasher from elif-auth
        PasswordHasher::hash(password)
    }}
    
    // <<<ELIF:END agent-editable:user-model-methods>>>
}}

{}

#[cfg(test)]
mod tests {{
    use super::*;
    use elif_testing::prelude::*;

    #[test_database]
    async fn test_user_creation() -> TestResult<()> {{
        let user = UserFactory::new().create().await?;
        
        assert!(!user.id.is_nil());
        assert!(!user.email.is_empty());
        assert!(!user.password.is_empty());
        
        Ok(())
    }}

    #[test_database]
    async fn test_find_by_email() -> TestResult<()> {{
        let user = UserFactory::new()
            .email("test@example.com".to_string())
            .create().await?;
        
        let found_user = User::find_by_email("test@example.com").await?;
        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().id, user.id);
        
        Ok(())
    }}

    #[test]
    fn test_password_hashing() {{
        let password = "test_password";
        let hashed = User::hash_password(password).unwrap();
        
        assert_ne!(hashed, password);
        
        let user = User {{
            password: hashed.clone(),
            ..Default::default()
        }};
        
        assert!(user.verify_password(password).unwrap());
        assert!(!user.verify_password("wrong_password").unwrap());
    }}
}}

// Factory for testing
#[factory]
pub struct UserFactory {{
    pub name: String,
    pub email: String,
    pub password: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}}

impl UserFactory {{
    pub fn verified(mut self) -> Self {{
        self.email_verified_at = Some(Utc::now());
        self
    }}
    
    pub fn unverified(mut self) -> Self {{
        self.email_verified_at = None;
        self
    }}
}}
"#,
            if rbac { "pub roles: Vec<String>," } else { "" },
            if rbac { r#"claims.insert("roles".to_string(), serde_json::json!(self.roles));"# } else { "" },
            if rbac {
                r#"
impl RbacUser for User {
    fn roles(&self) -> &[String] {
        &self.roles
    }
    
    fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
    
    fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }
    
    fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}
"#
            } else { "" }
        );

        Ok(GeneratedFile {
            path: self.project_root.join("src").join("models").join("user.rs"),
            content,
            file_type: GeneratedFileType::Model,
        })
    }

    fn generate_auth_controller(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let registration = context.get("registration").unwrap().as_bool().unwrap_or(false);
        let password_reset = context.get("password_reset").unwrap().as_bool().unwrap_or(false);
        let jwt = context.get("jwt").unwrap().as_bool().unwrap_or(false);

        // Build imports
        let mut imports = vec!["LoginRequest"];
        if registration {
            imports.push("RegisterRequest");
        }
        if password_reset {
            imports.push("ForgotPasswordRequest");
            imports.push("ResetPasswordRequest");
        }
        let imports_str = imports.join(", ");

        let mut content = String::new();
        content.push_str("use elif_http::prelude::*;\n");
        content.push_str("use elif_auth::prelude::*;\n");
        content.push_str("use elif_core::ServiceContainer;\n");
        content.push_str("use crate::models::user::User;\n");
        content.push_str(&format!("use crate::requests::auth::{{{}}};\n", imports_str));
        content.push_str("use std::sync::Arc;\n\n");
        
        content.push_str("#[controller]\n");
        content.push_str("pub struct AuthController {\n");
        content.push_str("    container: Arc<ServiceContainer>,\n");
        content.push_str("}\n\n");
        
        content.push_str("impl AuthController {\n");
        content.push_str("    pub fn new(container: Arc<ServiceContainer>) -> Self {\n");
        content.push_str("        Self { container }\n");
        content.push_str("    }\n\n");
        
        // Login method
        content.push_str("    // <<<ELIF:BEGIN agent-editable:auth-login>>>\n");
        content.push_str("    pub async fn login(&self, mut request: Request) -> Result<Response, HttpError> {\n");
        content.push_str("        let login_data: LoginRequest = request.validate_json()\n");
        content.push_str("            .map_err(|e| HttpError::unprocessable_entity(format!(\"Validation error: {}\", e)))?;\n");
        content.push_str("        \n");
        content.push_str("        // Find user by email\n");
        content.push_str("        let user = User::find_by_email(&login_data.email).await\n");
        content.push_str("            .map_err(|e| HttpError::internal_server_error(format!(\"Database error: {}\", e)))?\n");
        content.push_str("            .ok_or_else(|| HttpError::unauthorized(\"Invalid credentials\"))?;\n");
        content.push_str("        \n");
        content.push_str("        // Verify password\n");
        content.push_str("        if !user.verify_password(&login_data.password)\n");
        content.push_str("            .map_err(|e| HttpError::internal_server_error(format!(\"Password verification error: {}\", e)))? {\n");
        content.push_str("            return Err(HttpError::unauthorized(\"Invalid credentials\"));\n");
        content.push_str("        }\n");
        content.push_str("        \n");
        
        if jwt {
            content.push_str("        // Generate JWT token\n");
            content.push_str("        let jwt_provider = self.container.get::<JwtProvider>()\n");
            content.push_str("            .map_err(|_| HttpError::internal_server_error(\"JWT provider not available\"))?;\n");
            content.push_str("        \n");
            content.push_str("        let (access_token, refresh_token) = jwt_provider.generate_token_pair(&user)\n");
            content.push_str("            .map_err(|e| HttpError::internal_server_error(format!(\"Token generation error: {}\", e)))?;\n");
            content.push_str("        \n");
        }
        
        content.push_str("        Ok(Response::json(json!({\n");
        content.push_str("            \"message\": \"Login successful\",\n");
        content.push_str("            \"user\": {\n");
        content.push_str("                \"id\": user.id,\n");
        content.push_str("                \"name\": user.name,\n");
        content.push_str("                \"email\": user.email\n");
        content.push_str("            }");
        
        if jwt {
            content.push_str(",\n            \"access_token\": access_token,\n");
            content.push_str("            \"refresh_token\": refresh_token");
        }
        
        content.push_str("\n        })))\n");
        content.push_str("    }\n");
        content.push_str("    // <<<ELIF:END agent-editable:auth-login>>>\n\n");

        // Registration method if enabled
        if registration {
            content.push_str("    // <<<ELIF:BEGIN agent-editable:auth-register>>>\n");
            content.push_str("    pub async fn register(&self, mut request: Request) -> Result<Response, HttpError> {\n");
            content.push_str("        let register_data: RegisterRequest = request.validate_json()\n");
            content.push_str("            .map_err(|e| HttpError::unprocessable_entity(format!(\"Validation error: {}\", e)))?;\n");
            content.push_str("        \n");
            content.push_str("        // Check if user already exists\n");
            content.push_str("        if User::find_by_email(&register_data.email).await\n");
            content.push_str("            .map_err(|e| HttpError::internal_server_error(format!(\"Database error: {}\", e)))?\n");
            content.push_str("            .is_some() {\n");
            content.push_str("            return Err(HttpError::conflict(\"Email already registered\"));\n");
            content.push_str("        }\n");
            content.push_str("        \n");
            content.push_str("        // Hash password\n");
            content.push_str("        let hashed_password = User::hash_password(&register_data.password)\n");
            content.push_str("            .map_err(|e| HttpError::internal_server_error(format!(\"Password hashing error: {}\", e)))?;\n");
            content.push_str("        \n");
            content.push_str("        // Create user\n");
            content.push_str("        let user = User {\n");
            content.push_str("            name: register_data.name,\n");
            content.push_str("            email: register_data.email,\n");
            content.push_str("            password: hashed_password,\n");
            content.push_str("            email_verified_at: None,\n");
            content.push_str("            created_at: chrono::Utc::now(),\n");
            content.push_str("            updated_at: chrono::Utc::now(),\n");
            content.push_str("            ..Default::default()\n");
            content.push_str("        };\n");
            content.push_str("        \n");
            content.push_str("        let saved_user = user.save().await\n");
            content.push_str("            .map_err(|e| HttpError::internal_server_error(format!(\"Database error: {}\", e)))?;\n");
            content.push_str("        \n");
            content.push_str("        Ok(Response::json(json!({\n");
            content.push_str("            \"message\": \"Registration successful\",\n");
            content.push_str("            \"user\": {\n");
            content.push_str("                \"id\": saved_user.id,\n");
            content.push_str("                \"name\": saved_user.name,\n");
            content.push_str("                \"email\": saved_user.email\n");
            content.push_str("            }\n");
            content.push_str("        })).status(201))\n");
            content.push_str("    }\n");
            content.push_str("    // <<<ELIF:END agent-editable:auth-register>>>\n\n");
        }

        // Logout method
        content.push_str("    // <<<ELIF:BEGIN agent-editable:auth-logout>>>\n");
        content.push_str("    pub async fn logout(&self, request: Request) -> Result<Response, HttpError> {\n");
        content.push_str("        // For JWT, we might want to implement token blacklisting\n");
        content.push_str("        // For sessions, clear the session\n");
        content.push_str("        \n");
        content.push_str("        Ok(Response::json(json!({\n");
        content.push_str("            \"message\": \"Logged out successfully\"\n");
        content.push_str("        })))\n");
        content.push_str("    }\n");
        content.push_str("    // <<<ELIF:END agent-editable:auth-logout>>>\n\n");

        // Me method
        content.push_str("    // <<<ELIF:BEGIN agent-editable:auth-me>>>\n");
        content.push_str("    pub async fn me(&self, request: Request) -> Result<Response, HttpError> {\n");
        content.push_str("        let user = request.require_user()\n");
        content.push_str("            .map_err(|_| HttpError::unauthorized(\"Authentication required\"))?;\n");
        content.push_str("        \n");
        content.push_str("        Ok(Response::json(json!({\n");
        content.push_str("            \"id\": user.id(),\n");
        content.push_str("            \"email\": user.email()\n");
        content.push_str("        })))\n");
        content.push_str("    }\n");
        content.push_str("    // <<<ELIF:END agent-editable:auth-me>>>\n");
        
        content.push_str("}\n");

        Ok(GeneratedFile {
            path: self.project_root.join("src").join("controllers").join("auth_controller.rs"),
            content,
            file_type: GeneratedFileType::Controller,
        })
    }

    fn generate_auth_requests(&self, context: &HashMap<String, Value>) -> Result<Vec<GeneratedFile>, ElifError> {
        let mut files = Vec::new();
        let registration = context.get("registration").unwrap().as_bool().unwrap_or(false);
        let password_reset = context.get("password_reset").unwrap().as_bool().unwrap_or(false);

        // Login request
        let login_request = GeneratedFile {
            path: self.project_root.join("src").join("requests").join("auth").join("login_request.rs"),
            content: r#"use serde::{Serialize, Deserialize};
use elif_validation::prelude::*;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    
    #[validate(required, length(min = 1))]
    pub password: String,
    
    pub remember: Option<bool>,
}
"#.to_string(),
            file_type: GeneratedFileType::Request,
        };
        files.push(login_request);

        if registration {
            // Register request
            let register_request = GeneratedFile {
                path: self.project_root.join("src").join("requests").join("auth").join("register_request.rs"),
                content: r#"use serde::{Serialize, Deserialize};
use elif_validation::prelude::*;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(required, length(min = 2, max = 255))]
    pub name: String,
    
    #[validate(email)]
    pub email: String,
    
    #[validate(required, length(min = 8))]
    pub password: String,
    
    #[validate(confirmation = "password")]
    pub password_confirmation: String,
}
"#.to_string(),
                file_type: GeneratedFileType::Request,
            };
            files.push(register_request);
        }

        if password_reset {
            // Forgot password request
            let forgot_password_request = GeneratedFile {
                path: self.project_root.join("src").join("requests").join("auth").join("forgot_password_request.rs"),
                content: r#"use serde::{Serialize, Deserialize};
use elif_validation::prelude::*;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}
"#.to_string(),
                file_type: GeneratedFileType::Request,
            };
            files.push(forgot_password_request);

            // Reset password request
            let reset_password_request = GeneratedFile {
                path: self.project_root.join("src").join("requests").join("auth").join("reset_password_request.rs"),
                content: r#"use serde::{Serialize, Deserialize};
use elif_validation::prelude::*;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    #[validate(email)]
    pub email: String,
    
    #[validate(required)]
    pub token: String,
    
    #[validate(required, length(min = 8))]
    pub password: String,
    
    #[validate(confirmation = "password")]
    pub password_confirmation: String,
}
"#.to_string(),
                file_type: GeneratedFileType::Request,
            };
            files.push(reset_password_request);
        }

        Ok(files)
    }

    fn generate_auth_middleware(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let jwt = context.get("jwt").unwrap().as_bool().unwrap_or(false);
        let session = context.get("session").unwrap().as_bool().unwrap_or(false);

        let content = format!(
            r#"use elif_http::prelude::*;
use elif_auth::prelude::*;
use elif_core::ServiceContainer;
use std::sync::Arc;

pub fn setup_auth_middleware(container: Arc<ServiceContainer>) -> impl Fn() -> MiddlewarePipeline {{
    move || {{
        let mut pipeline = MiddlewarePipeline::new();
        
        {}
        
        {}
        
        pipeline
    }}
}}
"#,
            if jwt {
                r#"// JWT Authentication Middleware
        let jwt_provider = container.get::<JwtProvider>().expect("JWT provider not configured");
        pipeline.add(JwtMiddleware::new(jwt_provider)
            .skip_paths(vec![
                "/api/auth/login".to_string(),
                "/api/auth/register".to_string(),
                "/api/auth/forgot-password".to_string(),
                "/api/auth/reset-password".to_string(),
            ]));
"#
            } else { "" },
            if session {
                r#"// Session Authentication Middleware  
        let session_provider = container.get::<SessionProvider>().expect("Session provider not configured");
        pipeline.add(SessionMiddleware::new(session_provider)
            .skip_paths(vec![
                "/api/auth/login".to_string(),
                "/api/auth/register".to_string(),
            ]));
"#
            } else { "" }
        );

        Ok(GeneratedFile {
            path: self.project_root.join("src").join("middleware").join("auth.rs"),
            content,
            file_type: GeneratedFileType::Controller, // Using Controller as general code type
        })
    }

    fn generate_user_migration(&self, _context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let content = r#"-- Create users table
-- Up
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    email_verified_at TIMESTAMPTZ,
    password VARCHAR(255) NOT NULL,
    remember_token VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users (email);

-- Down
DROP TABLE IF EXISTS users;
"#;

        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
        let filename = format!("{}_create_users_table.sql", timestamp);

        Ok(GeneratedFile {
            path: self.project_root.join("migrations").join(filename),
            content: content.to_string(),
            file_type: GeneratedFileType::Migration,
        })
    }

    fn generate_auth_tests(&self, context: &HashMap<String, Value>) -> Result<GeneratedFile, ElifError> {
        let registration = context.get("registration").unwrap().as_bool().unwrap_or(false);

        let content = format!(
            r#"use elif_testing::prelude::*;
use crate::models::user::User;
use crate::controllers::auth_controller::AuthController;

mod auth_tests {{
    use super::*;

    #[test_database]
    async fn test_login_success() -> TestResult<()> {{
        let user = UserFactory::new()
            .email("test@example.com".to_string())
            .password(User::hash_password("password123").unwrap())
            .create().await?;
        
        let response = TestClient::new()
            .post("/api/auth/login")
            .json(&json!({{
                "email": "test@example.com",
                "password": "password123"
            }}))
            .send()
            .await?;
            
        response.assert_status(200)
               .assert_json_contains(json!({{
                   "message": "Login successful",
                   "user": {{
                       "email": "test@example.com"
                   }}
               }}));
        
        Ok(())
    }}

    #[test_database]
    async fn test_login_invalid_credentials() -> TestResult<()> {{
        let response = TestClient::new()
            .post("/api/auth/login")
            .json(&json!({{
                "email": "nonexistent@example.com",
                "password": "wrongpassword"
            }}))
            .send()
            .await?;
            
        response.assert_status(401);
        
        Ok(())
    }}
    
    {}

    #[test_database]
    async fn test_me_authenticated() -> TestResult<()> {{
        let user = UserFactory::new().create().await?;
        
        let response = TestClient::new()
            .authenticated_as(&user)
            .get("/api/auth/me")
            .send()
            .await?;
            
        response.assert_status(200)
               .assert_json_contains(json!({{
                   "email": user.email
               }}));
        
        Ok(())
    }}

    #[test_database]
    async fn test_me_unauthenticated() -> TestResult<()> {{
        let response = TestClient::new()
            .get("/api/auth/me")
            .send()
            .await?;
            
        response.assert_status(401);
        
        Ok(())
    }}
}}
"#,
            if registration {
                r#"
    #[test_database]
    async fn test_register_success() -> TestResult<()> {
        let response = TestClient::new()
            .post("/api/auth/register")
            .json(&json!({
                "name": "John Doe",
                "email": "john@example.com",
                "password": "password123",
                "password_confirmation": "password123"
            }))
            .send()
            .await?;
            
        response.assert_status(201)
               .assert_json_contains(json!({
                   "message": "Registration successful",
                   "user": {
                       "name": "John Doe",
                       "email": "john@example.com"
                   }
               }));
        
        // Verify user was created in database
        assert_database_has("users", |user: User| {
            user.email == "john@example.com" && user.name == "John Doe"
        }).await?;
        
        Ok(())
    }

    #[test_database]
    async fn test_register_duplicate_email() -> TestResult<()> {
        UserFactory::new()
            .email("existing@example.com".to_string())
            .create().await?;
        
        let response = TestClient::new()
            .post("/api/auth/register")
            .json(&json!({
                "name": "Jane Doe",
                "email": "existing@example.com",
                "password": "password123",
                "password_confirmation": "password123"
            }))
            .send()
            .await?;
            
        response.assert_status(409);
        
        Ok(())
    }
"#
            } else { "" }
        );

        Ok(GeneratedFile {
            path: self.project_root.join("tests").join("auth").join("auth_test.rs"),
            content,
            file_type: GeneratedFileType::Test,
        })
    }

    fn generate_mfa_files(&self, _context: &HashMap<String, Value>) -> Result<Vec<GeneratedFile>, ElifError> {
        // Implementation for MFA files generation
        // This would include TOTP setup, backup codes, etc.
        Ok(vec![])
    }

    fn generate_rbac_files(&self, _context: &HashMap<String, Value>) -> Result<Vec<GeneratedFile>, ElifError> {
        // Implementation for RBAC files generation
        // This would include roles, permissions, etc.
        Ok(vec![])
    }
}