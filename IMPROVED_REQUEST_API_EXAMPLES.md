# ✅ IMPROVED Request API - Easy Data Access

The `ElifRequest` API has been enhanced with convenient helper methods to make accessing request data much easier.

## ✅ Path Parameters (e.g., `/users/{id}`)

### Before (Hard):
```rust
async fn get_user_by_id(request: ElifRequest) -> HttpResult<ElifResponse> {
    // Complex deserialization required
    #[derive(serde::Deserialize)]
    struct UserParams { id: u32 }
    
    let params: UserParams = request.path_params()?; // Returns error on failure
    let user_id = params.id;
    // ... rest of handler
}
```

### After (Easy):
```rust
async fn get_user_by_id(request: ElifRequest) -> HttpResult<ElifResponse> {
    // One line - gets path param and parses to u32
    let user_id: u32 = request.path_param_as("id")?;
    
    match USER_STORE.get(user_id) {
        Some(user) => Ok(ElifResponse::ok().json(&user)?),
        None => Ok(ElifResponse::not_found().text("User not found")),
    }
}
```

## ✅ JSON Body Data

### Before (Hard):
```rust
async fn create_user(request: ElifRequest) -> HttpResult<ElifResponse> {
    let create_req: CreateUserRequest = request.json()?; // Could be confusing
    // ... rest of handler
}
```

### After (Easy):
```rust
async fn create_user(request: ElifRequest) -> HttpResult<ElifResponse> {
    // Both sync and async versions available
    let create_req: CreateUserRequest = request.json()?;
    // OR for consistency with async handlers:
    let create_req: CreateUserRequest = request.json_async().await?;
    
    // Validation
    if create_req.name.trim().is_empty() {
        return Err(HttpError::bad_request("Name cannot be empty"));
    }
    
    let user = USER_STORE.create(create_req.name, create_req.email);
    Ok(ElifResponse::created().json(&user)?)
}
```

## ✅ Query Parameters (e.g., `/users?page=1&limit=10`)

### Easy Access:
```rust
async fn get_users(request: ElifRequest) -> HttpResult<ElifResponse> {
    // Get individual query params with default values
    let page: u32 = request.query_param_as("page")?.unwrap_or(1);
    let limit: u32 = request.query_param_as("limit")?.unwrap_or(10);
    
    // Or get as optional string
    let search = request.query_param("search"); // Returns Option<String>
    
    // Or deserialize all query params to a struct
    #[derive(serde::Deserialize)]
    struct UserQuery { page: Option<u32>, limit: Option<u32>, search: Option<String> }
    let query: UserQuery = request.query()?;
    
    // ... use params to filter/paginate users
}
```

## ✅ Headers

### Easy Access:
```rust
async fn protected_handler(request: ElifRequest) -> HttpResult<ElifResponse> {
    // Get any header easily
    let user_agent = request.header("user-agent");
    let custom_header = request.header("x-custom-header");
    
    // Or use built-in helpers
    let auth = request.authorization()?;
    let bearer_token = request.bearer_token()?;
    
    // ... handle request
}
```

## ✅ Complete Real-World Example

```rust
// Route: POST /api/users/{team_id}/members
async fn add_team_member(request: ElifRequest) -> HttpResult<ElifResponse> {
    // Get path parameter
    let team_id: u32 = request.path_param_as("team_id")?;
    
    // Get request body
    #[derive(serde::Deserialize)]
    struct AddMemberRequest {
        user_id: u32,
        role: String,
        notify: Option<bool>,
    }
    let add_req: AddMemberRequest = request.json()?;
    
    // Get optional query parameters
    let dry_run: bool = request.query_param_as("dry_run")?.unwrap_or(false);
    
    // Get headers for authorization
    let bearer_token = request.bearer_token()?.ok_or_else(|| 
        HttpError::unauthorized("Authentication required"))?;
        
    // Get client info
    let user_agent = request.header("user-agent").unwrap_or_default();
    let client_ip = request.client_ip()?.unwrap_or_default();
    
    // Validate authorization
    let current_user = authenticate_token(&bearer_token)?;
    if !current_user.can_manage_team(team_id) {
        return Err(HttpError::forbidden("Insufficient permissions"));
    }
    
    // Validation
    if add_req.role.is_empty() {
        return Err(HttpError::bad_request("Role cannot be empty"));
    }
    
    // Business logic
    if dry_run {
        return Ok(ElifResponse::ok().json(&json!({
            "message": "Would add user to team",
            "team_id": team_id,
            "user_id": add_req.user_id,
            "role": add_req.role
        }))?);
    }
    
    let member = TEAM_STORE.add_member(
        team_id, 
        add_req.user_id, 
        &add_req.role,
        current_user.id
    )?;
    
    // Optional notification
    if add_req.notify.unwrap_or(true) {
        NOTIFICATION_SERVICE.send_team_invite(member.user_id, team_id).await?;
    }
    
    // Log the action
    AUDIT_LOG.log_team_action(&current_user, team_id, "add_member", &user_agent, &client_ip).await?;
    
    Ok(ElifResponse::created()
        .header("Location", &format!("/api/teams/{}/members/{}", team_id, member.user_id))?
        .json(&member)?)
}
```

## ✅ New Helper Methods Added

### Path Parameters:
- `request.path_param("name")` - Get path param as String
- `request.path_param_as::<T>("name")` - Get path param parsed to type T

### Query Parameters:  
- `request.query_param("name")` - Get query param as Option<String>
- `request.query_param_as::<T>("name")` - Get query param parsed to Option<T>

### Headers:
- `request.header("name")` - Get header as Option<String>

### Body:
- `request.json_async().await` - Async version of json parsing

All these methods provide **clear error messages** and handle **type conversion automatically** with meaningful error responses.

**The API is now intuitive and requires minimal code for common tasks!**