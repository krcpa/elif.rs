use elif_http_derive::{get, param};

#[get("/users/{id}")]
#[param(id: string)]  // Mismatch: route has {id} but param declares string
fn get_user(id: u32) -> String {  // Function expects u32, param declares string
    format!("User {}", id)
}