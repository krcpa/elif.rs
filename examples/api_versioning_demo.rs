use elif_http::{
    Server, ElifRouter,
    middleware::versioning::{VersioningConfig, VersionStrategy, ApiVersion, RequestVersionExt},
    routing::{versioned_router, VersionedRouter},
    response::ElifJson,
    errors::{HttpError, VersionedErrorExt},
    request::ElifRequest,
};
use serde_json::json;
use std::collections::HashMap;

/// API Versioning Demo
/// 
/// This example demonstrates how to implement API versioning in elif.rs with:
/// - Multiple versioning strategies (URL path, headers, query parameters)
/// - Version deprecation and sunset dates
/// - Backward compatibility
/// - Version-aware error responses
/// - Automatic migration guides

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting API Versioning Demo Server...");
    
    // Create different versions of the API
    let api = create_versioned_api().await;
    
    // Start server
    let server = Server::new()
        .router(api.build())
        .bind("127.0.0.1:3000");
    
    println!("📡 Server running on http://127.0.0.1:3000");
    println!("\n📖 Try these endpoints:");
    println!("   GET /api/v1/users     - Users API v1 (deprecated)");
    println!("   GET /api/v2/users     - Users API v2 (current)");
    println!("   GET /api/v3/users     - Users API v3 (beta)");
    println!("   GET /health           - Health check (unversioned)");
    println!("\n🔄 Headers to try:");
    println!("   Api-Version: v1       - For header-based versioning");
    println!("   Accept: application/vnd.api+json;version=2");
    println!("\n📝 Query parameters:");
    println!("   ?version=v2           - For query-based versioning");
    println!("\n⚡ All strategies now work due to proper middleware integration!");
    
    server.run().await?;
    Ok(())
}

async fn create_versioned_api() -> VersionedRouter<()> {
    // Version 1 - Legacy API (deprecated)
    let v1_router = ElifRouter::new()
        .get("/users", handle_users_v1)
        .get("/users/:id", handle_user_by_id_v1)
        .post("/users", handle_create_user_v1);

    // Version 2 - Current stable API
    let v2_router = ElifRouter::new()
        .get("/users", handle_users_v2)
        .get("/users/:id", handle_user_by_id_v2)
        .post("/users", handle_create_user_v2)
        .put("/users/:id", handle_update_user_v2);

    // Version 3 - Beta API with new features
    let v3_router = ElifRouter::new()
        .get("/users", handle_users_v3)
        .get("/users/:id", handle_user_by_id_v3)
        .post("/users", handle_create_user_v3)
        .put("/users/:id", handle_update_user_v3)
        .delete("/users/:id", handle_delete_user_v3);

    // Global routes (not versioned)
    let global_router = ElifRouter::new()
        .get("/health", handle_health)
        .get("/docs/migration/:version", handle_migration_docs);

    // Create versioned router with URL path strategy
    versioned_router::<()>()
        .version("v1", v1_router)
        .version("v2", v2_router) 
        .version("v3", v3_router)
        .global(global_router)
        .default_version("v2")
        .strategy(VersionStrategy::UrlPath)
        .deprecate_version(
            "v1", 
            Some("API v1 is deprecated. Please migrate to v2. See /docs/migration/v1"),
            Some("2024-12-31")
        )
}

// Version 1 handlers (legacy, simple responses)
async fn handle_users_v1(_req: ElifRequest) -> Result<ElifJson, HttpError> {
    Ok(ElifJson(json!([
        {"id": 1, "name": "John Doe", "email": "john@example.com"},
        {"id": 2, "name": "Jane Smith", "email": "jane@example.com"}
    ])))
}

async fn handle_user_by_id_v1(req: ElifRequest) -> Result<ElifJson, HttpError> {
    let id = req.path_param("id")
        .ok_or_else(|| HttpError::bad_request("User ID is required".to_string()))?;
    
    Ok(ElifJson(json!({
        "id": id,
        "name": "John Doe",
        "email": "john@example.com"
    })))
}

async fn handle_create_user_v1(_req: ElifRequest) -> Result<ElifJson, HttpError> {
    Ok(ElifJson(json!({
        "id": 3,
        "name": "New User",
        "email": "new@example.com",
        "created": true
    })))
}

// Version 2 handlers (enhanced with metadata)
async fn handle_users_v2(req: ElifRequest) -> Result<ElifJson, HttpError> {
    // Check if this is a deprecated version
    if req.is_deprecated_version() {
        println!("⚠️  Client is using deprecated version: {:?}", req.api_version());
    }
    
    Ok(ElifJson(json!({
        "users": [
            {
                "id": 1, 
                "name": "John Doe", 
                "email": "john@example.com",
                "created_at": "2024-01-15T10:30:00Z",
                "updated_at": "2024-01-15T10:30:00Z"
            },
            {
                "id": 2, 
                "name": "Jane Smith", 
                "email": "jane@example.com",
                "created_at": "2024-01-16T14:20:00Z",
                "updated_at": "2024-01-16T14:20:00Z"
            }
        ],
        "meta": {
            "total": 2,
            "page": 1,
            "per_page": 10
        }
    })))
}

async fn handle_user_by_id_v2(req: ElifRequest) -> Result<ElifJson, HttpError> {
    let id = req.path_param("id")
        .ok_or_else(|| HttpError::bad_request("User ID is required".to_string()))?;
    
    // Simulate user not found with version-aware error
    if id == "999" {
        if let Some(version_info) = req.version_info() {
            return Err(HttpError::not_found("User not found".to_string()));
        }
    }
    
    Ok(ElifJson(json!({
        "id": id,
        "name": "John Doe",
        "email": "john@example.com",
        "profile": {
            "bio": "Software developer",
            "avatar_url": "https://example.com/avatar.jpg"
        },
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T10:30:00Z"
    })))
}

async fn handle_create_user_v2(_req: ElifRequest) -> Result<ElifJson, HttpError> {
    Ok(ElifJson(json!({
        "user": {
            "id": 3,
            "name": "New User",
            "email": "new@example.com",
            "created_at": "2024-01-17T09:15:00Z",
            "updated_at": "2024-01-17T09:15:00Z"
        },
        "message": "User created successfully"
    })))
}

async fn handle_update_user_v2(req: ElifRequest) -> Result<ElifJson, HttpError> {
    let id = req.path_param("id")
        .ok_or_else(|| HttpError::bad_request("User ID is required".to_string()))?;
    
    Ok(ElifJson(json!({
        "user": {
            "id": id,
            "name": "Updated User",
            "email": "updated@example.com",
            "updated_at": "2024-01-17T12:00:00Z"
        },
        "message": "User updated successfully"
    })))
}

// Version 3 handlers (with advanced features)
async fn handle_users_v3(_req: ElifRequest) -> Result<ElifJson, HttpError> {
    Ok(ElifJson(json!({
        "data": [
            {
                "id": 1,
                "type": "user",
                "attributes": {
                    "name": "John Doe",
                    "email": "john@example.com",
                    "status": "active",
                    "role": "admin"
                },
                "relationships": {
                    "posts": {"data": [{"id": 1, "type": "post"}]},
                    "team": {"data": {"id": 1, "type": "team"}}
                },
                "meta": {
                    "last_login": "2024-01-17T11:30:00Z",
                    "permissions": ["read", "write", "admin"]
                }
            }
        ],
        "included": [
            {
                "id": 1,
                "type": "post", 
                "attributes": {"title": "Hello World", "content": "..."}
            }
        ],
        "meta": {
            "pagination": {
                "page": 1,
                "pages": 1,
                "per_page": 10,
                "total": 1
            }
        },
        "jsonapi": {"version": "1.0"}
    })))
}

async fn handle_user_by_id_v3(req: ElifRequest) -> Result<ElifJson, HttpError> {
    let id = req.path_param("id")
        .ok_or_else(|| HttpError::bad_request("User ID is required".to_string()))?;
    
    Ok(ElifJson(json!({
        "data": {
            "id": id,
            "type": "user",
            "attributes": {
                "name": "John Doe",
                "email": "john@example.com",
                "status": "active",
                "preferences": {
                    "theme": "dark",
                    "notifications": true,
                    "language": "en"
                }
            },
            "relationships": {
                "posts": {
                    "links": {"self": format!("/api/v3/users/{}/posts", id)},
                    "data": [{"id": 1, "type": "post"}]
                }
            }
        }
    })))
}

async fn handle_create_user_v3(_req: ElifRequest) -> Result<ElifJson, HttpError> {
    Ok(ElifJson(json!({
        "data": {
            "id": 3,
            "type": "user",
            "attributes": {
                "name": "New User",
                "email": "new@example.com",
                "status": "pending_verification"
            }
        },
        "meta": {
            "verification_email_sent": true,
            "welcome_email_queued": true
        }
    })))
}

async fn handle_update_user_v3(req: ElifRequest) -> Result<ElifJson, HttpError> {
    let id = req.path_param("id")
        .ok_or_else(|| HttpError::bad_request("User ID is required".to_string()))?;
    
    Ok(ElifJson(json!({
        "data": {
            "id": id,
            "type": "user",
            "attributes": {
                "name": "Updated User",
                "email": "updated@example.com"
            }
        },
        "meta": {
            "updated_fields": ["name", "email"],
            "version": 2
        }
    })))
}

async fn handle_delete_user_v3(req: ElifRequest) -> Result<ElifJson, HttpError> {
    let id = req.path_param("id")
        .ok_or_else(|| HttpError::bad_request("User ID is required".to_string()))?;
    
    Ok(ElifJson(json!({
        "meta": {
            "deleted": true,
            "id": id,
            "soft_delete": true,
            "retention_period": "30 days"
        }
    })))
}

// Global handlers
async fn handle_health(_req: ElifRequest) -> Result<ElifJson, HttpError> {
    Ok(ElifJson(json!({
        "status": "healthy",
        "version": "1.0.0",
        "api_versions": {
            "supported": ["v1", "v2", "v3"],
            "default": "v2",
            "deprecated": ["v1"],
            "beta": ["v3"]
        },
        "timestamp": "2024-01-17T12:00:00Z"
    })))
}

async fn handle_migration_docs(req: ElifRequest) -> Result<ElifJson, HttpError> {
    let version = req.path_param("version")
        .ok_or_else(|| HttpError::bad_request("Version parameter is required".to_string()))?;
    
    let migration_info = match version.as_str() {
        "v1" => json!({
            "from_version": "v1",
            "to_version": "v2", 
            "migration_guide": {
                "overview": "Migrate from v1 to v2 for enhanced features and better error handling",
                "breaking_changes": [
                    "Response format changed to include metadata",
                    "Error responses now include error codes",
                    "Date formats changed to ISO 8601"
                ],
                "steps": [
                    "Update API endpoint URLs to /api/v2/",
                    "Update response parsing for new format",
                    "Handle new error response structure",
                    "Update date parsing logic"
                ]
            },
            "timeline": {
                "deprecation_date": "2024-01-01",
                "sunset_date": "2024-12-31"
            }
        }),
        "v2" => json!({
            "from_version": "v2",
            "to_version": "v3",
            "migration_guide": {
                "overview": "Migrate to v3 for JSON:API compliance and advanced features",
                "breaking_changes": [
                    "Full JSON:API specification compliance",
                    "New relationship structure",
                    "Enhanced metadata format"
                ],
                "steps": [
                    "Update to use JSON:API format",
                    "Modify relationship handling",
                    "Update pagination logic"
                ]
            },
            "status": "Optional - v2 is still fully supported"
        }),
        _ => {
            return Err(HttpError::not_found(format!("Migration guide for version '{}' not found", version)));
        }
    };
    
    Ok(ElifJson(migration_info))
}