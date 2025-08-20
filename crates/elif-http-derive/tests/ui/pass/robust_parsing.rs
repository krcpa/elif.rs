//! Test robust parsing of various macro syntaxes

use elif_http_derive::{routes, resource, group, get, post};

// Test robust parsing with different path formats
struct TestRoutes;

#[routes]
impl TestRoutes {
    #[get("")]
    pub fn empty_path() -> String {
        "empty".to_string()
    }
    
    #[get("/simple")]
    pub fn simple_path() -> String {
        "simple".to_string()
    }
    
    #[post("/users/{id}")]
    pub fn with_params(id: u32) -> String {
        format!("user {}", id)
    }
}

// Test group with just prefix
struct SimpleGroup;

#[group("/api")]
impl SimpleGroup {
    #[get("/health")]
    pub fn health() -> String {
        "OK".to_string()
    }
}

// Test individual resource definitions
#[resource("/orders")]
pub fn orders() -> String {
    "OrderController".to_string()
}

fn main() {}