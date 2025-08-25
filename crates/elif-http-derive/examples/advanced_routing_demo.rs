//! Advanced Routing Demo
//!
//! This example demonstrates all the advanced routing patterns implemented in issue #254:
//! - Route registration macros
//! - Resource registration
//! - Route grouping
//! - Parameter extraction
//!
//! Note: This is a demo showing macro expansion - it doesn't create a real server.

use elif_http_derive::{delete, get, group, patch, post, put, resource, routes};

// Example 1: Main application routes with mixed patterns
struct AppRoutes;

#[routes]
impl AppRoutes {
    /// Simple health check endpoint
    #[get("/health")]
    pub fn health() -> String {
        "OK".to_string()
    }

    /// API root with JSON response
    #[get("/api")]
    pub fn api_root() -> String {
        r#"{"message": "Welcome to the API", "version": "1.0"}"#.to_string()
    }

    /// Create a new item
    #[post("/items")]
    pub fn create_item() -> String {
        "Item created".to_string()
    }

    /// Get item by ID (with parameter extraction)
    #[get("/items/{id}")]
    pub fn get_item(id: u32) -> String {
        format!("Item {}", id)
    }

    /// Update item by ID
    #[put("/items/{id}")]
    pub fn update_item(id: u32, data: String) -> String {
        format!("Updated item {} with data: {}", id, data)
    }

    /// Delete item by ID
    #[delete("/items/{id}")]
    pub fn delete_item(id: u32) -> String {
        format!("Deleted item {}", id)
    }

    /// Patch item partially
    #[patch("/items/{id}")]
    pub fn patch_item(id: u32) -> String {
        format!("Patched item {}", id)
    }

    /// RESTful resource for users
    #[resource("/users")]
    pub fn users() -> String {
        "UserController with full CRUD operations".to_string()
    }

    /// RESTful resource for posts
    #[resource("/posts")]
    pub fn posts() -> String {
        "PostController with full CRUD operations".to_string()
    }
}

// Example 2: Admin routes with grouping
struct AdminRoutes;

#[group("/admin")]
impl AdminRoutes {
    /// Admin dashboard
    #[get("/dashboard")]
    pub fn dashboard() -> String {
        "Admin Dashboard - System Overview".to_string()
    }

    /// View system settings
    #[get("/settings")]
    pub fn settings() -> String {
        "System Settings".to_string()
    }

    /// Update system settings
    #[post("/settings")]
    pub fn update_settings() -> String {
        "Settings updated successfully".to_string()
    }

    /// User management
    #[get("/users")]
    pub fn manage_users() -> String {
        "User Management Panel".to_string()
    }

    /// Create admin user
    #[post("/users")]
    pub fn create_admin_user() -> String {
        "Admin user created".to_string()
    }
}

// Example 3: API versioned routes
struct ApiV1Routes;

#[group("/api/v1")]
impl ApiV1Routes {
    /// Get user profile
    #[get("/profile")]
    pub fn profile() -> String {
        "User profile data".to_string()
    }

    /// Update user profile
    #[put("/profile")]
    pub fn update_profile() -> String {
        "Profile updated".to_string()
    }

    /// User logout
    #[post("/logout")]
    pub fn logout() -> String {
        "Successfully logged out".to_string()
    }

    /// Get user notifications
    #[get("/notifications")]
    pub fn notifications() -> String {
        "User notifications".to_string()
    }

    /// Complex nested resource access
    #[get("/organizations/{org_id}/members/{member_id}")]
    pub fn get_org_member(org_id: String, member_id: u64) -> String {
        format!("Member {} in organization {}", member_id, org_id)
    }
}

// Example 4: Individual resource definitions
#[resource("/api/v1/products")]
pub fn product_controller() -> String {
    "ProductController - handles all product operations".to_string()
}

#[resource("/api/v1/orders")]
pub fn order_controller() -> String {
    "OrderController - handles all order operations".to_string()
}

#[resource("/api/v1/payments")]
pub fn payment_controller() -> String {
    "PaymentController - handles payment processing".to_string()
}

// Example 5: Complex parameter patterns
struct ComplexRoutes;

#[routes]
impl ComplexRoutes {
    /// Multiple path parameters
    #[get("/users/{user_id}/posts/{post_id}/comments/{comment_id}")]
    pub fn get_comment(user_id: u32, post_id: u64, comment_id: u32) -> String {
        format!(
            "Comment {} on post {} by user {}",
            comment_id, post_id, user_id
        )
    }

    /// Mixed parameter types
    #[get("/categories/{category}/items/{item_id}")]
    pub fn get_categorized_item(category: String, item_id: u32) -> String {
        format!("Item {} in category '{}'", item_id, category)
    }

    /// File upload with path parameter
    #[post("/users/{id}/avatar")]
    pub fn upload_avatar(id: u32, file_data: Vec<u8>) -> String {
        format!(
            "Uploaded avatar for user {} ({} bytes)",
            id,
            file_data.len()
        )
    }
}

fn main() {
    println!("=== Advanced Routing Demo ===\n");

    // Demonstrate router generation
    let app_router = AppRoutes::build_router();
    println!("App Router: {}", app_router);

    let admin_group = AdminRoutes::build_group();
    println!("Admin Group: {}", admin_group);

    let api_v1_group = ApiV1Routes::build_group();
    println!("API v1 Group: {}", api_v1_group);

    let complex_router = ComplexRoutes::build_router();
    println!("Complex Router: {}", complex_router);

    // Demonstrate resource path extraction
    println!("\n=== Resource Paths ===");
    println!("Products: {}", product_controller_resource_path());
    println!("Orders: {}", order_controller_resource_path());
    println!("Payments: {}", payment_controller_resource_path());

    println!("\n=== Route Registration Benefits ===");
    println!("• Reduced boilerplate by ~70%");
    println!("• Automatic parameter extraction validation");
    println!("• Route grouping with shared middleware support");
    println!("• RESTful resource pattern shortcuts");
    println!("• Compile-time route validation");
    println!("• IDE autocomplete and error checking");
}
