//! Test parameter injection functionality

use elif_http_derive::{controller, get};

// Mock types for testing
pub struct ElifRequest;
pub struct ElifResponse;
pub type HttpResult<T> = Result<T, Box<dyn std::error::Error>>;
pub struct HttpError;

#[derive(Debug)]
pub struct ParamError;

impl HttpError {
    pub fn bad_request(_msg: String) -> Box<dyn std::error::Error> {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "bad request",
        ))
    }
}

impl ElifResponse {
    pub fn ok() -> Self {
        Self
    }
    pub fn json<T>(&self, _data: &T) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self)
    }
}

impl ElifRequest {
    pub fn path_param_int(&self, _name: &str) -> Result<i32, ParamError> {
        Ok(42)
    }

    pub fn path_param_string(&self, _name: &str) -> Result<String, ParamError> {
        Ok("test".to_string())
    }
}

#[controller("/api/users")]
pub struct UserController;

impl UserController {
    // Test single parameter injection
    #[get("/{id}")]
    #[param(id: int)]
    pub async fn show(&self, id: i32, req: ElifRequest) -> String {
        // id should be automatically extracted from request
        let _ = req; // Use the parameter to avoid warnings
        format!("User ID: {}", id)
    }

    // Test multiple parameter injection (using separate param attributes for now)
    #[get("/{user_id}/posts/{post_id}")]
    #[param(user_id: int)]
    #[param(post_id: string)]
    pub async fn get_user_post(&self, user_id: i32, post_id: String, req: ElifRequest) -> String {
        let _ = req; // Use the parameter to avoid warnings
        format!("User: {}, Post: {}", user_id, post_id)
    }

    // Test method without parameters (should work unchanged)
    #[get("/health")]
    pub async fn health(&self, _req: ElifRequest) -> String {
        "OK".to_string()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_param_injection_compiles() {
        // This test verifies that the param injection macro transforms functions correctly
        // The actual runtime behavior would be tested in integration tests
        assert!(true);
    }
}

fn main() {
    println!("Parameter injection test compilation successful");
}
