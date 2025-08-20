//! Test that all HTTP method macros compile successfully

use elif_http_derive::{controller, get, post, put, delete, patch, head, options};

#[controller("/rest")]
pub struct RestController;

impl RestController {
    #[get("/resource")]
    pub async fn get_resource(&self) -> String { "GET".to_string() }
    
    #[post("/resource")]
    pub async fn post_resource(&self) -> String { "POST".to_string() }
    
    #[put("/resource")]
    pub async fn put_resource(&self) -> String { "PUT".to_string() }
    
    #[delete("/resource")]
    pub async fn delete_resource(&self) -> String { "DELETE".to_string() }
    
    #[patch("/resource")]
    pub async fn patch_resource(&self) -> String { "PATCH".to_string() }
    
    #[head("/resource")]
    pub async fn head_resource(&self) -> String { "HEAD".to_string() }
    
    #[options("/resource")]
    pub async fn options_resource(&self) -> String { "OPTIONS".to_string() }
}

fn main() {}