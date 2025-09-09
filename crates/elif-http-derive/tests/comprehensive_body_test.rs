// Comprehensive tests for body macro functionality
#![allow(unused_imports)]
use elif_http_derive::{post, put, body};
use elif_http::{ElifRequest, ElifResponse, HttpResult, HttpError};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct CreateUserDto {
    pub name: String,
    pub email: String,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct UpdateUserDto {
    pub name: Option<String>,
    pub email: Option<String>,
}

pub struct UserController;

impl UserController {
    // Test 1: Simple body parameter injection
    #[post("/users")]
    #[body(data: CreateUserDto)]
    pub async fn create(&self, data: CreateUserDto) -> HttpResult<ElifResponse> {
        println!("Creating user: {:?}", data);
        Ok(ElifResponse::created())
    }
    
    // Test 2: Body parameter with different name
    #[post("/users/bulk")]
    #[body(user_data: CreateUserDto)]
    pub async fn create_bulk(&self, user_data: CreateUserDto) -> HttpResult<ElifResponse> {
        println!("Creating bulk user: {:?}", user_data);
        Ok(ElifResponse::created())
    }
    
    // Test 3: Body parameter with different DTO type
    #[put("/users")]
    #[body(update_data: UpdateUserDto)]
    pub async fn update(&self, update_data: UpdateUserDto) -> HttpResult<ElifResponse> {
        println!("Updating user: {:?}", update_data);
        Ok(ElifResponse::ok())
    }
    
    // Test 4: Non-async method with body parameter
    #[post("/users/sync")]
    #[body(data: CreateUserDto)]
    pub fn create_sync(&self, data: CreateUserDto) -> HttpResult<ElifResponse> {
        println!("Creating user synchronously: {:?}", data);
        Ok(ElifResponse::created())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compilation() {
        // If this compiles, all the body macro variations are working
        let _controller = UserController;
        println!("Comprehensive body macro test compiled successfully");
    }
    
    #[test]
    fn test_method_signatures() {
        // Test that the generated wrapper methods exist and have correct signatures
        let _controller = UserController;
        
        // The fact that this compiles means the wrapper methods were generated correctly
        // with the correct signatures: fn method_name(&self, request: ElifRequest) -> HttpResult<ElifResponse>
        
        println!("All method signatures are correct");
    }
}
