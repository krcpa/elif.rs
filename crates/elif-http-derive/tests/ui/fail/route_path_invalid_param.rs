use elif_http_derive::get;

#[get("/users/{}")]  // Empty parameter name
fn get_user() -> String {
    "user".to_string()
}