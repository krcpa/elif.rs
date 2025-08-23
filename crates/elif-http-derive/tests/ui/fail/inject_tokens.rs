//! Test that token injection detection works correctly with convention-based approach
//! 
//! This test verifies that the macro correctly identifies token references
//! using the "Token" suffix convention and generates appropriate code.

use elif_http_derive::inject;

// Mock services for testing
struct UserService;

// Service tokens (must end with "Token") 
struct EmailServiceToken;
struct NotificationToken;

// Test struct with token-based injection
#[inject(
    // Regular service injection
    user_service: UserService,
    
    // Token-based injection (should be detected by "Token" suffix)
    email_service: &EmailServiceToken,
    notifications: &NotificationToken
)]
struct TokenController;

fn main() {
    // This should compile without errors, demonstrating that:
    // 1. &EmailServiceToken is correctly identified as a token (ends with "Token")
    // 2. &NotificationToken is correctly identified as a token (ends with "Token")
    // 3. The macro generates appropriate field types and resolution code
    println!("Token injection macro compilation test passed!");
}