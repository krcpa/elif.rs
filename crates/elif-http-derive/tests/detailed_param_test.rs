//! Detailed test to verify parameter injection with ElifRequest

#[allow(unused_imports)]
use elif_http_derive::{get, post, put, param, body};

// Mock types for compilation
#[derive(Debug)]
pub struct ElifRequest;

#[derive(Debug)]
pub struct ElifResponse;

pub type HttpResult<T> = Result<T, String>;

// Define ElifError as an alias to match the framework
pub type ElifError = String;

pub struct HttpError;

impl HttpError {
    pub fn bad_request(msg: String) -> String {
        msg
    }
}

impl ElifRequest {
    pub fn path_param_u32(&self, _name: &str) -> Result<u32, String> {
        Ok(42)
    }
    
    pub async fn json<T>(&self) -> Result<T, String> 
    where 
        T: Default
    {
        Ok(T::default())
    }
}

#[derive(Default)]
pub struct TestDto {
    pub name: String,
}

pub struct DetailedController;

impl DetailedController {
    #[get("/{id}")]
    #[param(id: u32)]
    pub async fn show(&self, id: u32) -> HttpResult<ElifResponse> {
        println!("ID: {}", id);
        Ok(ElifResponse)
    }

    #[post("")]
    #[body(data: TestDto)]
    pub async fn create(&self, data: TestDto) -> HttpResult<ElifResponse> {
        println!("Data: {}", data.name);
        Ok(ElifResponse)
    }

    #[put("/{id}")]
    #[param(id: u32)]
    #[body(data: TestDto)]
    pub async fn update(&self, id: u32, data: TestDto) -> HttpResult<ElifResponse> {
        println!("ID: {}, Data: {}", id, data.name);
        Ok(ElifResponse)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_detailed_param_injection() {
        // This test just verifies that the macro expansion compiles
        println!("Detailed param injection test compiled successfully");
    }
}
