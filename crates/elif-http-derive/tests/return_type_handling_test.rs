//! Test return type handling in HTTP method macros

use elif_http_derive::{get, controller, param};

// Mock the required types
pub struct ElifRequest;
pub struct ElifResponse;
pub type HttpResult<T> = Result<T, Box<dyn std::error::Error>>;
pub struct HttpError;

#[derive(Debug)]
pub struct ParamError;

impl HttpError {
    pub fn bad_request(_msg: String) -> Box<dyn std::error::Error> {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "bad request"))
    }
    
    pub fn internal_server_error(_msg: String) -> Box<dyn std::error::Error> {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "internal server error"))
    }
}

impl ElifResponse {
    pub fn ok() -> Self { Self }
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

#[controller("/api/test")]
pub struct ReturnTypeController;

impl ReturnTypeController {
    // Test 1: Returns HttpResult<ElifResponse> - should pass through directly
    #[get("/{id}/http-result")]
    #[param(id: int)]
    pub async fn returns_http_result(&self, id: i32) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().json(&format!("ID: {}", id))?)
    }
    
    // Test 2: Returns ElifResponse - should wrap in Ok()
    #[get("/{id}/elif-response")]
    #[param(id: int)]
    pub async fn returns_elif_response(&self, id: i32) -> ElifResponse {
        ElifResponse::ok().json(&format!("ID: {}", id)).unwrap()
    }
    
    // Test 3: Returns Result<T, E> - should handle Ok/Err cases
    #[get("/{id}/result")]
    #[param(id: int)]
    pub async fn returns_result(&self, id: i32) -> Result<String, &'static str> {
        if id > 0 {
            Ok(format!("Positive ID: {}", id))
        } else {
            Err("ID must be positive")
        }
    }
    
    // Test 4: Returns unit () - should return empty OK response
    #[get("/{id}/unit")]
    #[param(id: int)]
    pub async fn returns_unit(&self, id: i32) {
        // Just a side effect, no return value - id parameter consumed but not used
        let _ = id;
    }
    
    // Test 5: Returns serializable type - should wrap in JSON
    #[get("/{id}/string")]
    #[param(id: int)]
    pub async fn returns_string(&self, id: i32) -> String {
        format!("String result: {}", id)
    }
}

fn main() {
    println!("Return type handling test compilation successful");
}