//! Example demonstrating Request/Response abstractions
//! 
//! This example shows how to use ElifRequest and ElifResponse for handling
//! HTTP requests and responses with JSON, headers, and parameters.

use axum::{
    Router,
    extract::{Path, Query},
    http::{Method, Uri, HeaderMap, StatusCode},
    body::Bytes,
    routing::get,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use elif_http::{
    ElifRequest, ElifResponse, ElifJson, JsonResponse, ApiResponse,
    RequestExtractor, HttpResult, IntoElifResponse,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct UserQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
}

// Simulate request/response handling
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Request/Response Abstractions Demo");
    println!("=====================================");

    // Example 1: Path parameter extraction
    demo_path_parameters()?;
    println!();
    
    // Example 2: Query parameter handling
    demo_query_parameters()?;
    println!();
    
    // Example 3: JSON request/response
    demo_json_handling()?;
    println!();
    
    // Example 4: Headers and authentication
    demo_headers_auth()?;
    println!();
    
    // Example 5: Response building
    demo_response_building()?;
    println!();

    // Example 6: Error responses
    demo_error_responses()?;
    
    Ok(())
}

fn demo_path_parameters() -> HttpResult<()> {
    println!("📍 Path Parameter Extraction Demo:");
    
    // Simulate a request to /users/123/posts/456
    let uri: Uri = "/users/123/posts/456?page=2&search=rust".parse().unwrap();
    let mut path_params = HashMap::new();
    path_params.insert("user_id".to_string(), "123".to_string());
    path_params.insert("post_id".to_string(), "456".to_string());
    
    let request = ElifRequest::new(Method::GET, uri, HeaderMap::new())
        .with_path_params(path_params);
    
    // Extract typed path parameters
    let user_id: u32 = request.path_param_parsed("user_id")?;
    let post_id: u32 = request.path_param_parsed("post_id")?;
    
    println!("   User ID: {}", user_id);
    println!("   Post ID: {}", post_id);
    println!("   ✅ Path parameters extracted successfully");
    
    Ok(())
}

fn demo_query_parameters() -> HttpResult<()> {
    println!("🔍 Query Parameter Handling Demo:");
    
    // Simulate request with query parameters
    let uri: Uri = "/users?page=2&per_page=25&search=rust%20developer".parse().unwrap();
    let mut query_params = HashMap::new();
    query_params.insert("page".to_string(), "2".to_string());
    query_params.insert("per_page".to_string(), "25".to_string());
    query_params.insert("search".to_string(), "rust developer".to_string());
    
    let request = ElifRequest::new(Method::GET, uri, HeaderMap::new())
        .with_query_params(query_params);
    
    // Extract typed query parameters
    let page: u32 = request.query_param_required("page")?;
    let per_page: Option<u32> = request.query_param_parsed("per_page")?;
    let search: Option<String> = request.query_param("search").cloned();
    
    println!("   Page: {}", page);
    println!("   Per page: {:?}", per_page);
    println!("   Search: {:?}", search);
    println!("   ✅ Query parameters handled successfully");
    
    Ok(())
}

fn demo_json_handling() -> HttpResult<()> {
    println!("📄 JSON Request/Response Demo:");
    
    let user = User {
        id: 123,
        name: "Jane Doe".to_string(),
        email: "jane@example.com".to_string(),
    };
    
    // Simulate JSON request body
    let json_body = serde_json::to_vec(&user).expect("Failed to serialize user");
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    
    let request = ElifRequest::new(
        Method::POST, 
        "/users".parse().unwrap(), 
        headers
    ).with_body(Bytes::from(json_body));
    
    // Parse JSON from request
    let parsed_user: User = request.json()?;
    println!("   Parsed user: {:?}", parsed_user);
    
    // Create JSON response
    let response = JsonResponse::created(&user)?;
    println!("   ✅ JSON request/response handled successfully");
    
    Ok(())
}

fn demo_headers_auth() -> HttpResult<()> {
    println!("🔐 Headers and Authentication Demo:");
    
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer abc123xyz789".parse().unwrap());
    headers.insert("user-agent", "ElifClient/1.0".parse().unwrap());
    headers.insert("x-api-version", "v1".parse().unwrap());
    
    let request = ElifRequest::new(
        Method::GET,
        "/api/protected".parse().unwrap(),
        headers,
    );
    
    // Extract authentication token
    let token = request.bearer_token()?.unwrap();
    let user_agent = request.user_agent()?.unwrap();
    let api_version = request.header_string("x-api-version")?.unwrap();
    
    println!("   Bearer Token: {}", token);
    println!("   User Agent: {}", user_agent);
    println!("   API Version: {}", api_version);
    println!("   ✅ Headers and auth extracted successfully");
    
    Ok(())
}

fn demo_response_building() -> HttpResult<()> {
    println!("🏗️  Response Building Demo:");
    
    let user = User {
        id: 456,
        name: "Bob Smith".to_string(),
        email: "bob@example.com".to_string(),
    };
    
    // Build various response types
    let json_response = ElifResponse::created()
        .header("x-resource-id", "456")?
        .header("location", "/users/456")?
        .json(&user)?
        .build()?;
    
    println!("   Created JSON response with headers");
    
    // Paginated response
    let users = vec![user.clone(), user.clone()];
    let paginated = JsonResponse::paginated(&users, 1, 10, 25)?;
    println!("   Created paginated response");
    
    // Redirect response
    let redirect = ElifResponse::redirect_permanent("/new-location")?
        .build()?;
    
    println!("   Created redirect response");
    println!("   ✅ Various response types built successfully");
    
    Ok(())
}

fn demo_error_responses() -> HttpResult<()> {
    println!("❌ Error Response Demo:");
    
    // Create various error responses
    let bad_request = JsonResponse::error(
        StatusCode::BAD_REQUEST, 
        "Invalid input data"
    )?;
    println!("   Bad request error response created");
    
    // Validation error
    let mut validation_errors = std::collections::HashMap::new();
    validation_errors.insert("email".to_string(), vec!["Email is required".to_string()]);
    validation_errors.insert("name".to_string(), vec!["Name must be at least 2 characters".to_string()]);
    
    let validation_response = JsonResponse::validation_error(&validation_errors)?;
    println!("   Validation error response created");
    
    // API error response
    let api_error = ApiResponse::<()>::error("Something went wrong".to_string());
    let api_error_response = api_error.to_response()?;
    println!("   API error response created");
    
    println!("   ✅ Error responses handled successfully");
    
    Ok(())
}