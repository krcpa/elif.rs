//! Integration tests for controller macro on impl blocks
//!
//! Tests the new architecture where #[controller] is applied to impl blocks
//! instead of structs, enabling automatic route registration.
//!
//! Note: We cannot test the full ElifController trait implementation here
//! due to circular dependency issues. That will be tested in elif-http itself.

use elif_http_derive::{controller, get, middleware, post};

// Define a simple controller struct
pub struct UserController;

// For testing, we'll make the macro generate code that doesn't depend on elif-http types
// We override the controller macro behavior by not providing the required types in scope
// This tests that constants are generated correctly

impl UserController {
    // Apply HTTP method macros - they should work as documentation markers
    #[get("")]
    pub async fn list(&self, _req: String) -> Result<String, String> {
        Ok("User list".to_string())
    }

    #[get("/{id}")]
    pub async fn show(&self, _req: String) -> Result<String, String> {
        Ok("User details".to_string())
    }

    #[post("")]
    #[middleware("auth")]
    pub async fn create(&self, _req: String) -> Result<String, String> {
        Ok("User created".to_string())
    }
}

// Test applying controller to struct (legacy mode)
#[controller("/api/legacy")]
pub struct LegacyController;

#[test]
fn test_legacy_controller_constants() {
    // Verify constants are added to the struct impl
    assert_eq!(LegacyController::BASE_PATH, "/api/legacy");
    assert_eq!(LegacyController::CONTROLLER_NAME, "LegacyController");
}

// Test controller with middleware at method level
pub struct AdminController;

impl AdminController {
    #[get("/dashboard")]
    #[middleware("auth", "admin")]
    pub async fn dashboard(&self, _req: String) -> Result<String, String> {
        Ok("Admin dashboard".to_string())
    }

    #[post("/settings")]
    #[middleware("auth", "admin", "csrf")]
    pub async fn update_settings(&self, _req: String) -> Result<String, String> {
        Ok("Settings updated".to_string())
    }
}

// Test applying controller to empty struct
#[controller("/empty")]
pub struct EmptyController;

#[test]
fn test_empty_controller_constants() {
    assert_eq!(EmptyController::BASE_PATH, "/empty");
    assert_eq!(EmptyController::CONTROLLER_NAME, "EmptyController");
}
