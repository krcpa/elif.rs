//! Test that #[group] macro generates correct route group code

use elif_http_derive::{group, get, post};

struct ApiV1Routes;

#[group("/api/v1")]
impl ApiV1Routes {
    #[get("/profile")]
    pub fn profile() -> String {
        "Profile".to_string()
    }
    
    #[post("/logout")]
    pub fn logout() -> String {
        "Logged out".to_string()
    }
}

fn main() {}