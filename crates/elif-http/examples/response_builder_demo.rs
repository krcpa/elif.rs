//! Response Builder Pattern Demo
//! 
//! Demonstrates the new Laravel-style response() builder pattern
//! introduced in Issue #244

use elif_http::response::{response, json_response, text_response, redirect_response};
use elif_http::{ElifResponse, ElifStatusCode, HttpResult};
use serde_json::json;

// Mock controller to show usage patterns
struct UserController;

impl UserController {
    /// List users - Clean Laravel-style syntax
    pub async fn list(&self) -> HttpResult<ElifResponse> {
        let users = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];
        
        Ok(response().json(users).into())
    }
    
    /// Show user - with cache control
    pub async fn show(&self, _id: u32) -> HttpResult<ElifResponse> {
        let user = json!({"id": 1, "name": "Alice", "email": "alice@example.com"});
        
        Ok(response()
            .json(user)
            .cache_control("public, max-age=3600")
            .header("x-cached", "true").into())
    }
    
    /// Create user - with location header and 201 status
    pub async fn create(&self) -> HttpResult<ElifResponse> {
        let user = json!({"id": 123, "name": "Charlie", "created_at": "2024-01-01T00:00:00Z"});
        
        Ok(response()
            .json(user)
            .created()
            .location("/users/123")
            .header("x-request-id", "abc-123").into())
    }
    
    /// Update user - using text response
    pub async fn update(&self) -> HttpResult<ElifResponse> {
        Ok(response()
            .text("User updated successfully")
            .header("x-updated", "true").into())
    }
    
    /// Delete user - no content response
    pub async fn delete(&self) -> HttpResult<ElifResponse> {
        Ok(response().no_content().into())
    }
    
    /// Redirect after login
    pub async fn redirect_after_login(&self) -> HttpResult<ElifResponse> {
        Ok(response().redirect("/dashboard").into())
    }
    
    /// Permanent redirect from old URL
    pub async fn redirect_permanent(&self) -> HttpResult<ElifResponse> {
        Ok(response().redirect("/users").permanent().into())
    }
    
    /// Error response - not found
    pub async fn user_not_found(&self) -> HttpResult<ElifResponse> {
        Ok(response()
            .error("User not found")
            .not_found().into())
    }
    
    /// Validation error response
    pub async fn validation_error(&self) -> HttpResult<ElifResponse> {
        let errors = json!({
            "name": ["Name is required"],
            "email": ["Email must be valid"]
        });
        
        Ok(response()
            .validation_error(errors)
            .unprocessable_entity().into())
    }
    
    /// Complex response with CORS and security headers
    pub async fn api_response(&self) -> HttpResult<ElifResponse> {
        let data = json!({
            "message": "API response",
            "version": "1.0",
            "timestamp": "2024-01-01T00:00:00Z"
        });
        
        Ok(response()
            .json(data)
            .cors("*")
            .with_security_headers()
            .cache_control("no-cache").into())
    }
}

/// Comparison: Before vs After
#[allow(dead_code)]
mod comparison {
    use super::*;
    
    pub struct OldStyleController;
    
    impl OldStyleController {
        /// OLD WAY (verbose, error-prone)
        pub async fn list_old(&self) -> HttpResult<ElifResponse> {
            let users = vec!["Alice", "Bob"];
            let response = ElifResponse::ok()
                .json(&users)?;
            Ok(response)
        }
        
        /// OLD WAY (more complex)
        pub async fn create_old(&self) -> HttpResult<ElifResponse> {
            let user = json!({"id": 1, "name": "Alice"});
            let response = ElifResponse::with_status(ElifStatusCode::CREATED)
                .json(&user)?
                .header("location", "/users/1")?
                .header("cache-control", "no-cache")?;
            Ok(response)
        }
    }
    
    pub struct NewStyleController;
    
    impl NewStyleController {
        /// NEW WAY (clean, Laravel-like)
        pub async fn list_new(&self) -> HttpResult<ElifResponse> {
            let users = vec!["Alice", "Bob"];
            Ok(response().json(users).into())
        }
        
        /// NEW WAY (fluent, readable)
        pub async fn create_new(&self) -> HttpResult<ElifResponse> {
            let user = json!({"id": 1, "name": "Alice"});
            Ok(response()
                .json(user)
                .created()
                .location("/users/1")
                .cache_control("no-cache").into())
        }
    }
}

/// Global helper function usage examples
#[allow(dead_code)]
mod global_helpers {
    use super::*;
    
    pub async fn using_global_helpers() -> Vec<ElifResponse> {
        vec![
            // JSON response
            json_response(json!({"message": "Hello"})).into(),
            
            // Text response
            text_response("Hello World").ok().into(),
            
            // Redirect response
            redirect_response("/home").into(),
        ]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = UserController;
    
    println!("=== Response Builder Pattern Demo ===\n");
    
    // Test various response patterns
    let responses = vec![
        ("List users", controller.list().await?),
        ("Show user", controller.show(1).await?),
        ("Create user", controller.create().await?),
        ("Update user", controller.update().await?),
        ("Delete user", controller.delete().await?),
        ("Redirect after login", controller.redirect_after_login().await?),
        ("Permanent redirect", controller.redirect_permanent().await?),
        ("User not found", controller.user_not_found().await?),
        ("Validation error", controller.validation_error().await?),
        ("API response", controller.api_response().await?),
    ];
    
    for (name, response) in responses {
        println!("{}: Status = {:?}", name, response.status_code());
        if response.has_header("location") {
            println!("  Location: {:?}", response.get_header("location"));
        }
        if response.has_header("cache-control") {
            println!("  Cache-Control: {:?}", response.get_header("cache-control"));
        }
        if response.has_header("access-control-allow-origin") {
            println!("  CORS: {:?}", response.get_header("access-control-allow-origin"));
        }
        println!();
    }
    
    println!("✅ All response patterns work correctly!");
    println!("\n🎯 Key Benefits:");
    println!("  • Laravel-style fluent API: response().json(data).created()");
    println!("  • Method chaining: .location().cache_control().header()");
    println!("  • No explicit error handling needed");
    println!("  • Intuitive status helpers: .ok(), .created(), .not_found()");
    println!("  • Built-in CORS and security helpers");
    
    Ok(())
}