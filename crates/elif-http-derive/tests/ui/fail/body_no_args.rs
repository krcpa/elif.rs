use elif_http_derive::{post, body};

#[post("/users")]
#[body]  // Missing body type argument
fn create_user() -> String {
    "created".to_string()
}