//! Test comprehensive advanced routing patterns

use elif_http_derive::{routes, resource, group, get, post, put, delete};

// Test route registration with #[routes] macro
struct AppRoutes;

#[routes]
impl AppRoutes {
    #[get("/health")]
    pub fn health() -> String {
        "OK".to_string()
    }
    
    #[post("/data")]
    pub fn create_data() -> String {
        "Created".to_string()
    }
    
    #[put("/data/{id}")]
    pub fn update_data() -> String {
        "Updated".to_string()
    }
    
    #[delete("/data/{id}")]
    pub fn delete_data() -> String {
        "Deleted".to_string()
    }
    
    #[resource("/users")]
    pub fn users() -> String {
        "UserController".to_string()
    }
    
    #[resource("/posts")]
    pub fn posts() -> String {
        "PostController".to_string()
    }
}

// Test route grouping with #[group] macro
struct AdminRoutes;

#[group("/admin")]
impl AdminRoutes {
    #[get("/dashboard")]
    pub fn dashboard() -> String {
        "Admin Dashboard".to_string()
    }
    
    #[post("/settings")]
    pub fn update_settings() -> String {
        "Settings Updated".to_string()
    }
}

// Test individual resource definitions
#[resource("/api/v1/products")]
pub fn product_resource() -> String {
    "ProductController".to_string()
}

#[resource("/api/v1/orders")]
pub fn order_resource() -> String {
    "OrderController".to_string()
}

fn main() {
    // Test generated functions
    let router_info = AppRoutes::build_router();
    println!("App router: {}", router_info);
    
    let group_info = AdminRoutes::build_group();
    println!("Admin group: {}", group_info);
    
    let product_path = product_resource_resource_path();
    println!("Product resource path: {}", product_path);
    
    let order_path = order_resource_resource_path();
    println!("Order resource path: {}", order_path);
}