// Core test for body injection without auto-registration
#![allow(unused)]

use elif_http_derive::{post, body};

// Mock types that match the real elif_http types
#[derive(Debug)]
pub struct ElifRequest;

#[derive(Debug)]
pub struct ElifResponse;

#[derive(Debug)]
pub struct HttpError;

pub type HttpResult<T> = Result<T, HttpError>;

impl HttpError {
    pub fn bad_request<T: Into<String>>(_msg: T) -> Self {
        HttpError
    }
    
    pub fn internal_server_error<T: Into<String>>(_msg: T) -> Self {
        HttpError
    }
}

impl ElifRequest {
    pub fn json<T>(&self) -> Result<T, HttpError> 
    where
        T: Default
    {
        Ok(T::default())
    }
}

impl ElifResponse {
    pub fn created() -> Self {
        Self
    }
    
    pub fn ok() -> Self {
        Self
    }
    
    pub fn json<T>(self, _data: &T) -> Result<Self, HttpError> {
        Ok(self)
    }
}

#[derive(Default, Debug)]
pub struct CreateUsersDto {
    pub name: String,
}

pub struct UsersController;

impl UsersController {
    // Test method that should have body injection
    #[post("")]
    #[body(data: CreateUsersDto)]
    pub async fn create(&self, data: CreateUsersDto) -> HttpResult<ElifResponse> {
        println!("Creating user: {:?}", data);
        Ok(ElifResponse::created().json(&data)?)
    }
    
    // Test method without body injection for comparison
    #[post("/simple")]
    pub async fn create_simple(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
        println!("Simple create");
        Ok(ElifResponse::created())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compilation() {
        // If this compiles, the macros are working correctly
        let _controller = UsersController;
        println!("Body injection test compiled successfully");
    }
}
