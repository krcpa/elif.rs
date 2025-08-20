//! Test that #[routes] macro generates correct route registration code

use elif_http_derive::{routes, get, post, resource};

struct ApiRoutes;

#[routes]
impl ApiRoutes {
    #[get("/health")]
    pub fn health() -> String {
        "OK".to_string()
    }
    
    #[post("/data")]
    pub fn create_data() -> String {
        "Created".to_string()
    }
    
    #[resource("/users")]
    pub fn users() -> String {
        "UserController".to_string()
    }
}

fn main() {}