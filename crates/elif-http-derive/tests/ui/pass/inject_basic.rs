//! Basic service injection test
//! Tests the fundamental #[inject] macro functionality

use elif_http_derive::inject;
use std::sync::Arc;

// Mock concrete services for testing
struct MockUserService;
struct MockEmailService;

// Test basic service injection on unit struct
#[inject(user_service: MockUserService, email_service: MockEmailService)]
pub struct BasicController;

// Test service injection on struct with existing fields
#[inject(user_service: MockUserService)]
pub struct ControllerWithFields {
    pub name: String,
}

// Mock cache service
struct MockCacheService;

// Test optional service injection
#[inject(cache_service: Option<MockCacheService>)]
pub struct OptionalServiceController;

// Test multiple services
#[inject(user_service: MockUserService, email_service: MockEmailService, cache_service: MockCacheService)]
pub struct MultiServiceController;

fn main() {
    // This test just verifies that the macros compile successfully
    println!("inject macro compilation test passed");
}