//! Test that middleware macro usage compiles successfully

use elif_http_derive::{controller, get, middleware};

#[controller("/api")]
#[middleware("logging")]
pub struct ApiController;

impl ApiController {
    #[get("/hello")]
    #[middleware("auth")]
    pub async fn hello(&self) -> String {
        "Hello".to_string()
    }
    
    #[get("/public")]
    pub async fn public(&self) -> String {
        "Public".to_string()
    }
}

// Test middleware on standalone function
#[middleware("timing")]
pub async fn standalone_handler() -> String {
    "Standalone".to_string()
}

fn main() {}