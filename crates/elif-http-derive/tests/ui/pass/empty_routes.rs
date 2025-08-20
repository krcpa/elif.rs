//! Test that empty route paths compile successfully

use elif_http_derive::{controller, get, post};

#[controller("/api")]
pub struct ApiController;

impl ApiController {
    // Empty route path should work
    #[get("")]
    pub async fn root(&self) -> String {
        "Root".to_string()
    }
    
    #[post("")]
    pub async fn root_post(&self) -> String {
        "Root POST".to_string()
    }
}

fn main() {}