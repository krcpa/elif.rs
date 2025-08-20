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
    /// List users - Clean Laravel-style syntax with terminal method
    pub async fn list(&self) -> HttpResult<ElifResponse> {
        let users = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];
        
        response().json(users).send()
    }
    
    /// Show user - with cache control using .finish() terminal method
    pub async fn show(&self, _id: u32) -> HttpResult<ElifResponse> {
        let user = json!({"id": 1, "name": "Alice", "email": "alice@example.com"});
        
        response()
            .json(user)
            .cache_control("public, max-age=3600")
            .header("x-cached", "true")
            .finish()
    }
    
    /// Create user - with location header and 201 status using .send()
    pub async fn create(&self) -> HttpResult<ElifResponse> {
        let user = json!({"id": 123, "name": "Charlie", "created_at": "2024-01-01T00:00:00Z"});
        
        response()
            .json(user)
            .created()
            .location("/users/123")
            .header("x-request-id", "abc-123")
            .send()
    }
    
    /// Update user - using text response
    pub async fn update(&self) -> HttpResult<ElifResponse> {
        response()
            .text("User updated successfully")
            .header("x-updated", "true")
            .send()
    }
    
    /// Delete user - no content response  
    pub async fn delete(&self) -> HttpResult<ElifResponse> {
        response().no_content().send()
    }
    
    /// Redirect after login
    pub async fn redirect_after_login(&self) -> HttpResult<ElifResponse> {
        response().redirect("/dashboard").send()
    }
    
    /// Permanent redirect from old URL
    pub async fn redirect_permanent(&self) -> HttpResult<ElifResponse> {
        response().redirect("/users").permanent().send()
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

    /// Response with multiple cookies - demonstrates multi-value header support
    pub async fn login_response(&self) -> HttpResult<ElifResponse> {
        let user = json!({"id": 1, "name": "Alice", "role": "admin"});
        
        response()
            .json(user)
            .cookie("session=abc123def456; Path=/; HttpOnly; Secure")
            .cookie("csrf_token=xyz789; Path=/; SameSite=Strict")
            .cookie("preferences=theme:dark,lang:en; Path=/; Max-Age=86400")
            .header("x-auth-method", "password")
            .created()
            .send()
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
        /// NEW WAY (clean, Laravel-like with terminal methods)
        pub async fn list_new(&self) -> HttpResult<ElifResponse> {
            let users = vec!["Alice", "Bob"];
            response().json(users).send()  // <-- No Ok() wrapper needed!
        }
        
        /// NEW WAY (fluent, readable with terminal chaining)
        pub async fn create_new(&self) -> HttpResult<ElifResponse> {
            let user = json!({"id": 1, "name": "Alice"});
            response()
                .json(user)
                .created()
                .location("/users/1")
                .cache_control("no-cache")
                .send()  // <-- Terminal method returns Result directly
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
        ("Login with cookies", controller.login_response().await?),
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
        if response.has_header("set-cookie") {
            println!("  Has Cookies: Multiple Set-Cookie headers supported");
        }
        if response.has_header("x-auth-method") {
            println!("  Auth Method: {:?}", response.get_header("x-auth-method"));
        }
        println!();
    }
    
    println!("✅ All response patterns work correctly!");
    println!("\n🎯 Key Benefits:");
    println!("  • Laravel-style fluent API: response().json(data).created().send()");
    println!("  • Method chaining: .location().cache_control().header()");
    println!("  • Multi-value header support: .cookie().cookie().cookie()");
    println!("  • Terminal methods: .send() and .finish() return Result directly");
    println!("  • No explicit Ok() wrapper needed with terminal methods");
    println!("  • Intuitive status helpers: .ok(), .created(), .not_found()");
    println!("  • Built-in CORS and security helpers");
    
    println!("\n📝 Usage Patterns:");
    println!("  OLD: Ok(response().json(data).created().into())");
    println!("  NEW: response().json(data).created().send()");
    
    Ok(())
}