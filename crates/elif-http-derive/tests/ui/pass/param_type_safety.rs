use elif_http_derive::{get, post, param, body};

#[get("/users/{id}")]
#[param(id: uint)]  // Correct: function parameter matches param type (uint -> u32)
fn get_user(id: u32) -> String {
    format!("User {}", id)
}

#[get("/users/{name}")]
#[param(name: string)]  // Correct: function parameter matches param type
fn get_user_by_name(name: String) -> String {
    format!("User {}", name)
}

#[post("/users")]
#[body(UserData)]  // Type safety for request body
fn create_user(data: ElifJson<UserData>) -> String {
    "created".to_string()
}

struct UserData {
    name: String,
    email: String,
}

// Mock ElifJson for compilation
struct ElifJson<T>(T);

fn main() {
    println!("Type safety test compilation successful");
}