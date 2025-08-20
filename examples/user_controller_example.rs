//! Example demonstrating the new ElifController system for route organization
//! 
//! This example shows how to:
//! - Define a controller with automatic route registration
//! - Handle different HTTP methods in controller methods
//! - Use the controller_dispatch! macro for method dispatch
//! - Register controllers with the router

use elif_http::{
    ElifController, ControllerRoute, RouteParam as ControllerRouteParam, HttpMethod,
    ElifRequest, ElifResponse, HttpResult,
    routing::ParamType,
    Router as ElifRouter,
    Server,
    controller_dispatch,
};
use serde::{Serialize, Deserialize};
use std::{sync::Arc, pin::Pin, future::Future};

/// User data structure
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// User controller implementing the ElifController trait
pub struct UserController;

impl ElifController for UserController {
    fn name(&self) -> &str {
        "UserController"
    }
    
    fn base_path(&self) -> &str {
        "/users"
    }
    
    fn routes(&self) -> Vec<ControllerRoute> {
        vec![
            // GET /users - List all users
            ControllerRoute::new(HttpMethod::GET, "", "list"),
            
            // GET /users/{id} - Get user by ID
            ControllerRoute::new(HttpMethod::GET, "/{id}", "show")
                .add_param(ControllerRouteParam::new("id", ParamType::Integer)),
            
            // POST /users - Create new user
            ControllerRoute::new(HttpMethod::POST, "", "create"),
            
            // PUT /users/{id} - Update existing user
            ControllerRoute::new(HttpMethod::PUT, "/{id}", "update")
                .add_param(ControllerRouteParam::new("id", ParamType::Integer)),
            
            // DELETE /users/{id} - Delete user
            ControllerRoute::new(HttpMethod::DELETE, "/{id}", "delete")
                .add_param(ControllerRouteParam::new("id", ParamType::Integer)),
        ]
    }
    
    fn handle_request(
        &self,
        method_name: String,
        request: ElifRequest,
    ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> {
        controller_dispatch!(self, &method_name, request, {
            "list" => Self::list,
            "show" => Self::show,
            "create" => Self::create,
            "update" => Self::update,
            "delete" => Self::delete
        })
    }
}

impl UserController {
    /// GET /users - List all users
    async fn list(&self, _request: ElifRequest) -> HttpResult<ElifResponse> {
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
    async fn show(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
        let id: u32 = request.path_param_int("id")?;
        
        let user = User {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        };
        
        Ok(ElifResponse::ok().json(&user)?)
    }
    
    /// POST /users - Create new user
    async fn create(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
        let body = request.body_string().await?;
        
        let new_user = User {
            id: 999, // In real app, this would be generated
            name: "New User".to_string(),
            email: "newuser@example.com".to_string(),
        };
        
        Ok(ElifResponse::created().json(&new_user)?)
    }
    
    /// PUT /users/{id} - Update user
    async fn update(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
        let id: u32 = request.path_param_int("id")?;
        
        let updated_user = User {
            id,
            name: format!("Updated User {}", id),
            email: format!("updated{}@example.com", id),
        };
        
        Ok(ElifResponse::ok().json(&updated_user)?)
    }
    
    /// DELETE /users/{id} - Delete user
    async fn delete(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
        let id: u32 = request.path_param_int("id")?;
        
        Ok(ElifResponse::ok().json(&serde_json::json!({
            "message": format!("User {} deleted successfully", id),
            "deleted_id": id
        }))?)
    }
}

/// Posts controller to demonstrate multiple controllers
pub struct PostController;

impl ElifController for PostController {
    fn name(&self) -> &str {
        "PostController"
    }
    
    fn base_path(&self) -> &str {
        "/posts"
    }
    
    fn routes(&self) -> Vec<ControllerRoute> {
        vec![
            ControllerRoute::new(HttpMethod::GET, "", "list"),
            ControllerRoute::new(HttpMethod::GET, "/{id}", "show")
                .add_param(ControllerRouteParam::new("id", ParamType::Integer)),
        ]
    }
    
    fn handle_request(
        &self,
        method_name: String,
        request: ElifRequest,
    ) -> Pin<Box<dyn Future<Output = HttpResult<ElifResponse>> + Send>> {
        controller_dispatch!(self, &method_name, request, {
            "list" => Self::list,
            "show" => Self::show
        })
    }
}

impl PostController {
    async fn list(&self, _request: ElifRequest) -> HttpResult<ElifResponse> {
        Ok(ElifResponse::ok().json(&serde_json::json!({
            "posts": [
                {"id": 1, "title": "Hello World", "content": "First post"},
                {"id": 2, "title": "Controller System", "content": "Using the new controller system"}
            ]
        }))?)
    }
    
    async fn show(&self, request: ElifRequest) -> HttpResult<ElifResponse> {
        let id: u32 = request.path_param_int("id")?;
        
        Ok(ElifResponse::ok().json(&serde_json::json!({
            "id": id,
            "title": format!("Post {}", id),
            "content": format!("Content of post {}", id)
        }))?)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create router with controllers
    let router = ElifRouter::<()>::new()
        .controller(UserController)
        .controller(PostController);
    
    // Create server with the router
    let server = Server::new()
        .use_router(router);
    
    println!("🚀 Server starting with controller system!");
    println!("Available routes:");
    println!("  GET    /users          - List all users");
    println!("  GET    /users/{{id}}    - Get user by ID");
    println!("  POST   /users          - Create new user");
    println!("  PUT    /users/{{id}}    - Update user");
    println!("  DELETE /users/{{id}}    - Delete user");
    println!("  GET    /posts          - List all posts");
    println!("  GET    /posts/{{id}}    - Get post by ID");
    println!("");
    println!("Example requests:");
    println!("  curl http://localhost:3000/users");
    println!("  curl http://localhost:3000/users/1");
    println!("  curl -X POST http://localhost:3000/users");
    println!("  curl http://localhost:3000/posts");
    
    // Start the server
    server.listen("127.0.0.1:3000").await?;
    
    Ok(())
}