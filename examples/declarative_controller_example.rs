//! Example demonstrating the new declarative controller macros
//! 
//! This example shows how to:
//! - Use #[controller] macro to define controller base path
//! - Use HTTP method macros (#[get], #[post], etc.) for route definition
//! - Apply middleware to controllers and individual methods
//! - Use parameter specification macros
//! 
//! Compare this with user_controller_example.rs to see the difference
//! in boilerplate reduction.

use elif_http::{
    ElifRequest, ElifResponse, HttpResult,
    Server,
    Router as ElifRouter,
};

// Enable the derive feature for the macros
#[cfg(feature = "derive")]
use elif_http::{controller, get, post, put, delete, middleware, param};

use serde::{Serialize, Deserialize};
use std::sync::Arc;

/// User data structure
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// Create user request structure
#[derive(Debug, Serialize, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[cfg(feature = "derive")]
mod declarative_controllers {
    use super::*;
    
    /// User controller with declarative routing macros
    /// 
    /// Using a unit struct for simplicity in this example.
    /// In real applications, controllers typically contain service dependencies
    /// injected through dependency injection or passed as fields.
    #[controller("/users")]
    #[middleware("logging", "cors")]
    pub struct UserController;

    impl UserController {
        /// GET /users - List all users
        #[get("")]
        #[middleware("cache")]
        pub async fn list(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
            let users = vec![
                User {
                    id: 1,
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                },
                User {
                    id: 2,
                    name: "Bob".to_string(),
                    email: "bob@example.com".to_string(),
                },
            ];
            
            Ok(ElifResponse::ok().json(&users)?)
        }
        
        /// GET /users/{id} - Get user by ID
        #[get("/{id}")]
        #[middleware("auth", "cache")]
        #[param(id: int)]
        pub async fn show(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
            let id: u32 = request.path_param_int("id")?;
            
            let user = User {
                id,
                name: format!("User {}", id),
                email: format!("user{}@example.com", id),
            };
            
            Ok(ElifResponse::ok().json(&user)?)
        }
        
        /// POST /users - Create new user
        #[post("")]
        #[middleware("auth", "validation")]
        pub async fn create(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
            // In the macro version, we could potentially auto-inject the JSON parsing
            let _data: CreateUserRequest = request.json().await?;
            
            let new_user = User {
                id: 999, // In real app, this would be generated
                name: "New User".to_string(),
                email: "newuser@example.com".to_string(),
            };
            
            Ok(ElifResponse::created().json(&new_user)?)
        }
        
        /// PUT /users/{id} - Update user
        #[put("/{id}")]
        #[middleware("auth", "validation")]
        #[param(id: int)]
        pub async fn update(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
            let id: u32 = request.path_param_int("id")?;
            let _data: CreateUserRequest = request.json().await?;
            
            let updated_user = User {
                id,
                name: format!("Updated User {}", id),
                email: format!("updated{}@example.com", id),
            };
            
            Ok(ElifResponse::ok().json(&updated_user)?)
        }
        
        /// DELETE /users/{id} - Delete user
        #[delete("/{id}")]
        #[middleware("auth")]
        #[param(id: int)]
        pub async fn delete(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
            let id: u32 = request.path_param_int("id")?;
            
            Ok(ElifResponse::ok().json(&serde_json::json!({
                "message": format!("User {} deleted successfully", id),
                "deleted_id": id
            }))?)
        }
    }
    
    /// Posts controller demonstrating additional routes
    #[controller("/posts")]
    #[middleware("logging")]
    pub struct PostController;

    impl PostController {
        /// GET /posts - List all posts
        #[get("")]
        pub async fn list(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
            Ok(ElifResponse::ok().json(&serde_json::json!({
                "posts": [
                    {"id": 1, "title": "Hello World", "content": "First post"},
                    {"id": 2, "title": "Declarative Routing", "content": "Using the new macro system"}
                ]
            }))?)
        }
        
        /// GET /posts/{id} - Get post by ID
        #[get("/{id}")]
        #[param(id: int)]
        pub async fn show(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
            let id: u32 = request.path_param_int("id")?;
            
            Ok(ElifResponse::ok().json(&serde_json::json!({
                "id": id,
                "title": format!("Post {}", id),
                "content": format!("Content of post {}", id)
            }))?)
        }
    }
    
    /// API information controller
    #[controller("/api")]
    pub struct ApiController;

    impl ApiController {
        /// GET /api/info - API information
        #[get("/info")]
        pub async fn info(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
            Ok(ElifResponse::ok().json(&serde_json::json!({
                "name": "elif.rs Declarative Controller API",
                "version": "0.1.0",
                "features": [
                    "Declarative controller macros",
                    "Automatic route registration",
                    "Middleware composition",
                    "Parameter validation"
                ]
            }))?)
        }
        
        /// GET /api/health - Health check
        #[get("/health")]
        pub async fn health(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {
            Ok(ElifResponse::ok().json(&serde_json::json!({
                "status": "healthy",
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))?)
        }
    }
}

#[cfg(not(feature = "derive"))]
mod fallback_message {
    use super::*;
    
    pub async fn show_message() {
        println!("❌ Derive feature is not enabled!");
        println!("To run this example with declarative macros, use:");
        println!("cargo run --example declarative_controller_example --features derive");
        println!("");
        println!("This example demonstrates the new #[controller] and HTTP method macros");
        println!("that significantly reduce boilerplate compared to manual route registration.");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "derive"))]
    {
        fallback_message::show_message().await;
        return Ok(());
    }
    
    #[cfg(feature = "derive")]
    {
        use declarative_controllers::*;
        
        // Create router with declarative controllers
        // Note: This would need to be implemented in the macro system
        // For now, this is a demonstration of the intended API
        let router = ElifRouter::<()>::new()
            .controller(UserController)
            .controller(PostController)
            .controller(ApiController);
        
        // Create server with the router
        let server = Server::new()
            .use_router(router);
        
        println!("🚀 Server starting with declarative controller macros!");
        println!("Available routes:");
        println!("  GET    /users          - List all users");
        println!("  GET    /users/{{id}}    - Get user by ID");
        println!("  POST   /users          - Create new user");
        println!("  PUT    /users/{{id}}    - Update user");
        println!("  DELETE /users/{{id}}    - Delete user");
        println!("  GET    /posts          - List all posts");
        println!("  GET    /posts/{{id}}    - Get post by ID");
        println!("  GET    /api/info       - API information");
        println!("  GET    /api/health     - Health check");
        println!("");
        println!("Middleware applied:");
        println!("  Users:  logging + cors + method-specific");
        println!("  Posts:  logging");
        println!("  API:    none");
        println!("");
        println!("Example requests:");
        println!("  curl http://localhost:3000/users");
        println!("  curl http://localhost:3000/users/1");
        println!("  curl -X POST -H 'Content-Type: application/json' -d '{{\"name\":\"John\",\"email\":\"john@example.com\"}}' http://localhost:3000/users");
        println!("  curl http://localhost:3000/api/info");
        
        // Start the server
        server.listen("127.0.0.1:3000").await?;
    }
    
    Ok(())
}