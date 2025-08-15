//! Pure Framework Usage Demo
//! 
//! This example shows how to use elif.rs framework-native types
//! instead of raw Axum re-exports. This ensures proper framework
//! abstractions and better consistency.

use elif_http::{
    // ✅ Use framework-native Router (not axum::Router)
    Router,
    // ✅ Use framework-native extractors (not axum::extract)
    ElifQuery, ElifPath, ElifState,
    // ✅ Use framework-native request/response (not axum types)
    ElifRequest, ElifResponse, ElifJson, 
    // ✅ Use framework-native HTTP types (not axum::http)
    ElifStatusCode, ElifHeaderMap,
    // ✅ Use framework-native error handling
    HttpResult, HttpError,
    // ✅ Use framework-native JSON responses
    JsonResponse, ApiResponse
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct UserQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserPath {
    id: u32,
}

#[derive(Debug, Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Clone)]
struct AppState {
    users: Vec<User>,
}

/// ❌ OLD WAY (using raw Axum re-exports):
/// ```rust,ignore
/// use elif_http::{Router, Json, Query, Path, State, StatusCode};
/// 
/// async fn get_user(
///     axum::extract::State(state): axum::extract::State<AppState>,
///     axum::extract::Path(path): axum::extract::Path<UserPath>,
///     axum::extract::Query(query): axum::extract::Query<UserQuery>
/// ) -> axum::response::Json<User> {
///     // This bypasses our framework abstractions!
/// }
/// ```

/// ✅ NEW WAY (using framework-native types):
async fn get_user_pure_framework(request: &ElifRequest) -> HttpResult<ElifJson<User>> {
    // Extract path parameters using framework-native extractor
    let path = ElifPath::<UserPath>::from_request(request)?;
    let user_id = path.0.id;
    
    // Extract query parameters using framework-native extractor
    let query = ElifQuery::<UserQuery>::from_request(request)?;
    let page = query.0.page.unwrap_or(1);
    let search = query.0.search;
    
    // Create a mock user (in real app, would come from database)
    let user = User {
        id: user_id,
        name: format!("User {}", user_id),
        email: format!("user{}@example.com", user_id),
    };
    
    // Return framework-native JSON response
    Ok(ElifJson(user))
}

/// Example of using framework-native response builder
async fn create_user_response() -> HttpResult<ElifResponse> {
    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };
    
    // ✅ Use framework-native response builder
    let response = ElifResponse::with_status(ElifStatusCode::CREATED)
        .json(&user)?
        .header("X-Custom-Header", "created-user")?
        .build();
    
    Ok(response)
}

/// Example of using framework-native JSON responses
async fn list_users_with_pagination() -> HttpResult<JsonResponse<Vec<User>>> {
    let users = vec![
        User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
        User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
    ];
    
    // ✅ Use framework-native paginated JSON response
    JsonResponse::paginated(&users, 1, 10, 25)
}

/// Example of framework-native error handling
async fn handle_user_error() -> HttpResult<JsonResponse<User>> {
    // ✅ Use framework-native error types
    Err(HttpError::not_found("User not found".to_string()))
}

/// Example of framework-native API responses
async fn api_response_example() -> HttpResult<ApiResponse> {
    let user = User {
        id: 1,
        name: "Jane".to_string(), 
        email: "jane@example.com".to_string(),
    };
    
    // ✅ Use framework-native API response
    Ok(ApiResponse::success(user))
}

/// Setting up router with framework-native types
pub fn create_router() -> Router<AppState> {
    let state = AppState {
        users: vec![
            User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
            User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
        ],
    };
    
    // ✅ Use framework-native Router (not axum::Router)
    Router::with_state(state)
        .get("/users/{id}", mock_handler) // In real app, use get_user_pure_framework 
        .post("/users", mock_handler)     // In real app, use create_user_response
        .get("/api/users", mock_handler)  // In real app, use list_users_with_pagination
}

// Mock handler for compilation (in real app, integrate with your handler system)
async fn mock_handler() -> &'static str {
    "Framework-native handler placeholder"
}

/// Key Benefits of Framework-Native Approach:
/// 
/// 1. **Consistency**: All types follow elif.rs conventions
/// 2. **Abstraction**: Framework can evolve without breaking user code
/// 3. **Features**: Framework-native types have additional elif.rs features
/// 4. **Documentation**: Clear separation between framework and Axum implementation  
/// 5. **Testing**: Framework-native types are easier to mock and test
/// 
/// Migration Guide:
/// - ❌ `axum::Router` → ✅ `Router` (from elif_http::routing)
/// - ❌ `axum::Json` → ✅ `ElifJson`
/// - ❌ `axum::extract::Query` → ✅ `ElifQuery::from_request()`
/// - ❌ `axum::extract::Path` → ✅ `ElifPath::from_request()`
/// - ❌ `axum::extract::State` → ✅ `ElifState` 
/// - ❌ `axum::http::StatusCode` → ✅ `ElifStatusCode`
/// - ❌ `axum::http::HeaderMap` → ✅ `ElifHeaderMap`

fn main() {
    println!("✅ Pure elif.rs Framework Demo");
    println!("This example shows proper usage of framework-native types");
    println!("See source code for detailed examples and migration guide");
}