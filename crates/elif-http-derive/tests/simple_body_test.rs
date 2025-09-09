// Simple test to verify body macro works without controller
#![allow(unused)]

use elif_http_derive::{post, body};

// Mock types for testing
#[derive(Debug)]
pub struct ElifRequest;

#[derive(Debug)]  
pub struct ElifResponse;

pub type HttpResult<T> = Result<T, String>;

pub struct HttpError;

impl HttpError {
    pub fn bad_request(msg: String) -> String {
        msg
    }
    
    pub fn internal_server_error(msg: String) -> String {
        msg
    }
}

impl ElifRequest {
    pub fn json<T>(&self) -> Result<T, String>
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
    
    pub fn json<T>(&self, _data: &T) -> Result<Self, String> {
        Ok(Self)
    }
}

#[derive(Default, Debug)]
pub struct CreateUsersDto {
    pub name: String,
}

pub struct UsersController;

impl UsersController {
    // Test body injection without controller macro
    #[post("")]
    #[body(data: CreateUsersDto)]
    pub async fn create(&self, data: CreateUsersDto) -> HttpResult<ElifResponse> {
        println!("Creating user: {:?}", data);
        Ok(ElifResponse::created().json(&data)?)
    }
}

fn main() {
    println!("Simple body test");
}
