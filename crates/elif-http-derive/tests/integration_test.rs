//! Integration tests for the derive macros
//!
//! These tests verify that the macros can be used as intended.

use elif_http_derive::{controller, get, post};

// Test that the controller macro works with string literal arguments
#[controller("/api/test")]
pub struct TestController;

impl TestController {
    // Test that HTTP method macros work with string literal arguments
    #[get("/hello")]
    pub async fn hello(&self) -> String {
        "Hello World!".to_string()
    }

    #[get("")] // Empty string for root route under controller
    pub async fn index(&self) -> String {
        "Index".to_string()
    }

    #[post("/data")]
    pub async fn create_data(&self) -> String {
        "Data created".to_string()
    }
}

#[test]
fn test_controller_macro_constants() {
    // Verify the controller macro generates the expected constants
    assert_eq!(TestController::BASE_PATH, "/api/test");
    assert_eq!(TestController::CONTROLLER_NAME, "TestController");
}

#[test]
fn test_controller_compilation() {
    // If this test compiles and runs, the macros are working
    let _controller = TestController;
    // The methods should be callable (though they're async)
    // This is mainly a compilation test
}
