//! Test that basic controller usage compiles successfully

use elif_http_derive::{controller, get, post};

#[controller("/users")]
pub struct UserController;

impl UserController {
    #[get("/")]
    pub async fn index(&self) -> String {
        "Index".to_string()
    }
    
    #[get("/{id}")]
    pub async fn show(&self, id: u32) -> String {
        format!("User {}", id)
    }
    
    #[post("/")]
    pub async fn create(&self) -> String {
        "Created".to_string()
    }
}

fn main() {}