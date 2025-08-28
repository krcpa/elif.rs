//! Demo DSL sugar syntax test - should compile successfully

use elif_http_derive::demo_module;

// Mock services
#[derive(Default)]
pub struct UserService;
#[derive(Default)]
pub struct EmailService;
#[derive(Default)]
pub struct UserController;
#[derive(Default)]
pub struct PostController;

fn main() {
    // Demo DSL basic usage
    let _descriptor = demo_module! {
        services: [UserService, EmailService],
        controllers: [UserController, PostController]
    };
    
    // Demo DSL with middleware
    let _descriptor_with_middleware = demo_module! {
        services: [UserService],
        controllers: [UserController],
        middleware: ["cors", "logging", "auth"]
    };
    
    // Demo DSL minimal
    let _minimal = demo_module! {
        services: [UserService]
    };
    
    println!("Demo DSL tests completed successfully!");
}