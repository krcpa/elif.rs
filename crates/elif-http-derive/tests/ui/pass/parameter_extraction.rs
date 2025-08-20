//! Test parameter extraction from route paths and function signatures

use elif_http_derive::{get, post, put, delete};

// Test basic parameter extraction
#[get("/users/{id}")]
pub fn get_user(id: u32) -> String {
    format!("User {}", id)
}

// Test multiple parameters
#[get("/users/{user_id}/posts/{post_id}")]
pub fn get_user_post(user_id: u32, post_id: u64) -> String {
    format!("User {} Post {}", user_id, post_id)
}

// Test mixed parameters and regular arguments
#[post("/users/{id}/avatar")]
pub fn upload_avatar(id: u32, data: Vec<u8>) -> String {
    format!("Uploaded avatar for user {} with {} bytes", id, data.len())
}

// Test route with no parameters
#[get("/health")]
pub fn health_check() -> String {
    "OK".to_string()
}

// Test complex parameter names
#[put("/api/v1/organizations/{org_id}/members/{member_id}")]
pub fn update_member(org_id: String, member_id: u64) -> String {
    format!("Updated member {} in org {}", member_id, org_id)
}

fn main() {}