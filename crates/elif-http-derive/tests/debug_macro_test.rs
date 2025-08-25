//! Debug macro test

use elif_http_derive::get;

// Mock the required types
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
        Ok(123)
    }
}

pub struct TestController;

// Simple test with parameter injection
impl TestController {
    #[get("/{id}")]
    #[param(id: int)]
    pub async fn show(&self, id: i32) -> String {
        format!("User ID: {}", id)
    }
}

fn main() {
    println!("Debug test compilation successful");
}
