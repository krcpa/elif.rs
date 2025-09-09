// Test async body method with both body and request parameters
#![allow(unused)]

use elif_http_derive::{post, body};

// Mock types that exactly match elif-http
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
    // This should work with body parameter injection
    #[post("")]
    #[body(data: CreateUsersDto)]
    pub async fn create(&self, data: CreateUsersDto) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::created().json(&data)?)
    }
}

// Test function to try calling the method
async fn test_method_call() {
    let controller = UsersController;
    let request = ElifRequest;
    
    // This should work if the wrapper is generated correctly
    let result = controller.create(request).await;
    match result {
        Ok(_) => println!("Method call succeeded"),
        Err(_) => println!("Method call failed"),
    }
}

fn main() {
    println!("Async body test compiled");
}
