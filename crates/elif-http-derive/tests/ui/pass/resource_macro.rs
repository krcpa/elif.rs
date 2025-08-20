//! Test that #[resource] macro works on function declarations

use elif_http_derive::resource;

#[resource("/users")]
pub fn user_controller() -> String {
    "UserController".to_string()
}

#[resource("/posts")]  
pub fn post_controller() -> String {
    "PostController".to_string()
}

fn main() {}