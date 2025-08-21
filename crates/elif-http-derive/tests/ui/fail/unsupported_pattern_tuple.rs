//! Test that tuple destructuring patterns are properly rejected

use elif_http_derive::{get, controller};

// Mock the required types
pub struct ElifRequest;
pub struct ElifResponse;
pub type HttpResult<T> = Result<T, Box<dyn std::error::Error>>;

#[controller("/api/test")]
pub struct TestController;

impl TestController {
    // This should fail because tuple destructuring is not supported
    #[get("/coords/{x}/{y}")]
    #[param(x: int)]
    #[param(y: int)]
    pub async fn get_coords(&self, (x, y): (u32, u32)) -> String {
        format!("Coords: {}, {}", x, y)
    }
}

fn main() {}