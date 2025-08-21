//! Test that handlers with unsupported parameters produce clear compile-time errors

use elif_http_derive::{get, controller, param};

// Mock the required types
pub struct ElifRequest;
pub struct ElifResponse;
pub type HttpResult<T> = Result<T, Box<dyn std::error::Error>>;

#[controller("/api/users")]
pub struct UserController;

impl UserController {
    // This should fail because `extra_param` is neither a path parameter nor ElifRequest
    #[get("/{id}")]
    #[param(id: int)]
    pub async fn show(&self, id: i32, extra_param: String, req: ElifRequest) -> String {
        format!("User ID: {}, Extra: {}", id, extra_param)
    }
}

fn main() {}