//! Test that underscore patterns work correctly and are ignored during validation

use elif_http_derive::{get, controller};

// Mock the required types
pub struct ElifRequest;
pub struct ElifResponse; 
pub type HttpResult<T> = Result<T, Box<dyn std::error::Error>>;

#[controller("/api/test")]
pub struct TestController;

impl TestController {
    // This should work - underscore patterns are acceptable for simple methods
    #[get("/example")]
    pub async fn example(&self, _unused_param: String, _: bool) -> String {
        format!("Example")
    }
    
    // This should also work - underscore-prefixed names are treated as unused  
    #[get("/another")]
    pub async fn another(&self, _debug_info: Option<String>) -> String {
        format!("Another")
    }
}

fn main() {
    println!("Underscore pattern test compilation successful");
}