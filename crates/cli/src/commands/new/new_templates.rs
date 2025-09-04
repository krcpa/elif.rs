use elif_core::ElifError;
use std::path::Path;
use tokio::fs;
use crate::generators::TemplateEngine;
use tera::Context;

pub async fn create_config_files(app_dir: &Path, name: &str) -> Result<(), ElifError> {
    let template_engine = TemplateEngine::new()?;
    
    // Create context for templates
    let mut context = Context::new();
    context.insert("project_name", name);
    context.insert("http_enabled", &true);
    context.insert("modules_enabled", &true);
    context.insert("database_enabled", &true);
    context.insert("database_type", "postgresql");
    
    // Render Cargo.toml from template
    let cargo_toml = template_engine.render_with_context("cargo_toml.stub", &context)?;
    
    fs::write(app_dir.join("Cargo.toml"), cargo_toml).await?;
    
    // .elif/manifest.yaml
    let manifest = format!(r#"name: {}
version: "0.1.0" 
database:
  url_env: DATABASE_URL
  migrations_dir: migrations
server:
  host: "0.0.0.0"
  port: 3000
routes:
  prefix: "/api/v1"
"#, name);
    
    fs::write(app_dir.join(".elif/manifest.yaml"), manifest).await?;
    
    // .elif/errors.yaml - Standardized error codes
    let errors_yaml = r#"# Standardized error codes for consistent API responses
# Use these codes in your controllers for uniform error handling

# Authentication & Authorization
- code: INVALID_CREDENTIALS
  http: 401
  message: "Invalid email or password"
  hint: "Check your login credentials and try again"

- code: UNAUTHORIZED
  http: 401
  message: "Authentication required"
  hint: "Please provide valid authentication credentials"

- code: FORBIDDEN
  http: 403
  message: "Access denied"
  hint: "You don't have permission to access this resource"

# Validation Errors
- code: VALIDATION_FAILED
  http: 400
  message: "Request validation failed"
  hint: "Check the request payload and try again"

- code: REQUIRED_FIELD_MISSING
  http: 400
  message: "Required field is missing"
  hint: "Include all required fields in your request"

# Resource Errors
- code: RESOURCE_NOT_FOUND
  http: 404
  message: "Resource not found"
  hint: "The requested resource may have been deleted or moved"

- code: RESOURCE_ALREADY_EXISTS
  http: 409
  message: "Resource already exists"
  hint: "Use a different identifier or update the existing resource"

# Server Errors
- code: INTERNAL_SERVER_ERROR
  http: 500
  message: "Internal server error"
  hint: "Please try again later or contact support"

- code: DATABASE_ERROR
  http: 503
  message: "Database temporarily unavailable"
  hint: "Please try again in a few moments"

# Rate Limiting
- code: RATE_LIMIT_EXCEEDED
  http: 429
  message: "Rate limit exceeded"
  hint: "Please wait before making more requests"
"#;
    
    fs::write(app_dir.join(".elif/errors.yaml"), errors_yaml).await?;
    
    // .env
    let env_content = r#"DATABASE_URL=postgresql://localhost/elif_dev
RUST_LOG=info
"#;
    
    fs::write(app_dir.join(".env"), env_content).await?;
    
    Ok(())
}

pub async fn create_source_files(app_dir: &Path, name: &str) -> Result<(), ElifError> {
    let template_engine = TemplateEngine::new()?;
    
    // Create context for templates
    let mut context = Context::new();
    context.insert("project_name", name);
    context.insert("http_enabled", &true);
    context.insert("modules_enabled", &true);
    context.insert("database_enabled", &true);
    context.insert("auth_enabled", &false);
    
    // Render main.rs from bootstrap template (Laravel-style one-liner)
    let main_rs = template_engine.render_with_context("main_bootstrap.stub", &context)?;
    fs::write(app_dir.join("src/main.rs"), main_rs).await?;
    
    // Create controllers/mod.rs
    let controllers_mod = template_engine.render_with_context("controllers_mod.stub", &context)?;
    fs::write(app_dir.join("src/controllers/mod.rs"), controllers_mod).await?;
    
    // Create controllers/user_controller.rs
    let user_controller = template_engine.render_with_context("user_controller.stub", &context)?;
    fs::write(app_dir.join("src/controllers/user_controller.rs"), user_controller).await?;
    
    // Create services/mod.rs
    let services_mod = template_engine.render_with_context("services_mod.stub", &context)?;
    fs::write(app_dir.join("src/services/mod.rs"), services_mod).await?;
    
    // Create services/user_service.rs
    let user_service = template_engine.render_with_context("user_service.stub", &context)?;
    fs::write(app_dir.join("src/services/user_service.rs"), user_service).await?;
    
    // Create modules/mod.rs
    let modules_mod = "pub mod app_module;";
    fs::write(app_dir.join("src/modules/mod.rs"), modules_mod).await?;
    
    // Create modules/app_module.rs with bootstrap-ready module
    context.insert("controller_name", "UserController");
    context.insert("service_name", "UserService");
    let app_module = template_engine.render_with_context("app_module_bootstrap.stub", &context)?;
    fs::write(app_dir.join("src/modules/app_module.rs"), app_module).await?;
    
    // Create minimal placeholder files for directories
    let minimal_controller = "// Add your controllers here";
    fs::write(app_dir.join("src/middleware/mod.rs"), minimal_controller).await?;
    
    let minimal_model = "// Add your models here";
    fs::write(app_dir.join("src/models/mod.rs"), minimal_model).await?;
    
    let minimal_routes = "// Add your routes here";
    fs::write(app_dir.join("src/routes/mod.rs"), minimal_routes).await?;
    
    Ok(())
}
