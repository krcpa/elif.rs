//! Test UUID parameter extraction

use elif_http_derive::{get, controller};

// Mock the required types
pub struct ElifRequest;
pub struct ElifResponse;
pub type HttpResult<T> = Result<T, Box<dyn std::error::Error>>;
pub struct HttpError;

#[derive(Debug)]
pub struct ParamError;

// Mock UUID type for testing
#[derive(Debug, Clone)]
pub struct Uuid(String);

impl Uuid {
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl HttpError {
    pub fn bad_request(_msg: String) -> Box<dyn std::error::Error> {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "bad request"))
    }
}

impl ElifResponse {
    pub fn ok() -> Self { Self }
    pub fn json<T>(&self, _data: &T) -> Result<Self, Box<dyn std::error::Error>> { 
        Ok(Self) 
    }
}

impl ElifRequest {
    pub fn path_param_uuid(&self, _name: &str) -> Result<Uuid, ParamError> {
        Ok(Uuid("123e4567-e89b-12d3-a456-426614174000".to_string()))
    }
}

#[controller("/api/users")]
pub struct UuidController;

impl UuidController {
    // Test UUID parameter extraction using the proper path_param_uuid method
    #[get("/{user_id}")]
    #[param(user_id: uuid)]
    pub async fn get_user(&self, user_id: Uuid) -> String {
        format!("User UUID: {}", user_id.to_string())
    }
    
    // Test multiple UUID parameters
    #[get("/{user_id}/posts/{post_id}")]
    #[param(user_id: uuid)]
    #[param(post_id: uuid)]
    pub async fn get_user_post(&self, user_id: Uuid, post_id: Uuid) -> String {
        format!("User: {}, Post: {}", user_id.to_string(), post_id.to_string())
    }
}

fn main() {
    println!("UUID parameter test compilation successful");
}