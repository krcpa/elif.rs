# API Versioning System

This document describes the comprehensive API versioning system implemented in elif.rs as part of Phase 9.7.

## Overview

The elif.rs API versioning system provides:

- **Multiple versioning strategies**: URL path, headers, query parameters, and Accept header
- **Version-specific routing**: Register different route handlers for different API versions
- **Deprecation management**: Mark versions as deprecated with sunset dates and migration guides
- **Version-aware error handling**: Enhanced error responses with migration information
- **Automatic OpenAPI documentation**: Generate version-specific API documentation
- **CLI tools**: Manage API versions, generate migration guides, and validate configurations
- **Interactive Swagger UI**: Browse API documentation for each version

## Quick Start

### Basic Versioned API

```rust
use elif_http::{
    Server, ElifRouter,
    routing::{versioned_router, VersionStrategy},
    response::ElifJson,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create version-specific routers
    let v1_router = ElifRouter::new()
        .get("/users", |_| async { Ok(ElifJson("v1 users")) });

    let v2_router = ElifRouter::new()
        .get("/users", |_| async { Ok(ElifJson("v2 users")) });

    // Create versioned API
    let api = versioned_router::<()>()
        .version("v1", v1_router)
        .version("v2", v2_router)
        .default_version("v2")
        .strategy(VersionStrategy::UrlPath)
        .deprecate_version("v1", Some("Please use v2"), Some("2024-12-31"))
        .build();

    // Start server
    Server::new()
        .router(api)
        .bind("127.0.0.1:3000")
        .run()
        .await?;

    Ok(())
}
```

### Access versioning information in handlers

```rust
use elif_http::{
    request::ElifRequest,
    middleware::versioning::RequestVersionExt,
    errors::{HttpError, VersionedErrorExt},
};

async fn handle_users(req: ElifRequest) -> Result<ElifJson, HttpError> {
    // Check current API version
    if let Some(version) = req.api_version() {
        println!("Request using API version: {}", version);
    }
    
    // Check if deprecated
    if req.is_deprecated_version() {
        println!("⚠️  Client using deprecated version");
    }
    
    // Return version-aware error if needed
    if let Some(version_info) = req.version_info() {
        if some_error_condition {
            return Err(HttpError::versioned_bad_request(
                version_info,
                "INVALID_DATA",
                "User data is invalid"
            ));
        }
    }
    
    Ok(ElifJson("users"))
}
```

## Versioning Strategies

### 1. URL Path Strategy (Recommended)

```rust
// URLs: /api/v1/users, /api/v2/users
let router = versioned_router::<()>()
    .strategy(VersionStrategy::UrlPath);
```

### 2. Header Strategy

```rust
// Header: Api-Version: v1
let router = versioned_router::<()>()
    .strategy(VersionStrategy::Header("Api-Version".to_string()));
```

### 3. Query Parameter Strategy

```rust
// URL: /api/users?version=v1
let router = versioned_router::<()>()
    .strategy(VersionStrategy::QueryParam("version".to_string()));
```

### 4. Accept Header Strategy

```rust
// Header: Accept: application/vnd.api+json;version=1
let router = versioned_router::<()>()
    .strategy(VersionStrategy::AcceptHeader);
```

## Version Management

### Creating Versions

```rust
let router = versioned_router::<()>()
    .version("v1", v1_router)
    .version("v2", v2_router)
    .default_version("v2");
```

### Deprecating Versions

```rust
let router = versioned_router::<()>()
    .deprecate_version(
        "v1",
        Some("API v1 is deprecated. Please migrate to v2"),
        Some("2024-12-31") // Sunset date
    );
```

## Error Handling

### Version-Aware Errors

```rust
use elif_http::errors::{HttpError, VersionedErrorExt, bad_request_v, not_found_v};

// Create version-aware errors
let response = bad_request_v(version_info, "INVALID_INPUT", "Invalid user data");
let response = not_found_v(version_info, "User");

// Or use the trait methods
let response = HttpError::versioned_validation_error(version_info, field_errors);
```

### Error Response Format

Version-aware errors return structured responses:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Request validation failed",
    "details": "One or more fields contain invalid values",
    "field_errors": {
      "email": ["Invalid email format"],
      "password": ["Password too short", "Must contain numbers"]
    }
  },
  "api_version": "v2",
  "migration_info": {
    "migration_guide_url": "/docs/migration/v2",
    "recommended_version": "v3",
    "deprecation_message": "This version is deprecated",
    "sunset_date": "2024-12-31"
  }
}
```

## CLI Commands

### Create New API Version

```bash
elifrs version create v3 --description "New version with enhanced features"
```

### Deprecate Version

```bash
elifrs version deprecate v1 --message "Please migrate to v2" --sunset-date "2024-12-31"
```

### List All Versions

```bash
elifrs version list
```

### Generate Migration Guide

```bash
elifrs version migrate v1 v2
```

### Validate Configuration

```bash
elifrs version validate
```

## OpenAPI Documentation

### Generate Version-Specific Documentation

```bash
# Generate OpenAPI spec for current project
elifrs openapi generate

# Export to different formats
elifrs openapi export postman api_collection.json
elifrs openapi export insomnia api_workspace.json

# Serve interactive documentation
elifrs openapi serve --port 8080
```

### Custom OpenAPI Configuration

```rust
use elif_openapi::{OpenApiGenerator, OpenApiConfig};

let config = OpenApiConfig::new("My API", "2.0.0")
    .add_server("https://api.example.com", Some("Production"))
    .add_server("https://staging-api.example.com", Some("Staging"))
    .add_tag("Users", Some("User management endpoints"))
    .add_tag("Posts", Some("Post management endpoints"));

let mut generator = OpenApiGenerator::new(config);
```

## Configuration

### API Version Configuration File

Create `api_versions.json` in your project root:

```json
{
  "versions": {
    "v1": {
      "version": "v1",
      "deprecated": true,
      "deprecation_message": "Please migrate to v2",
      "sunset_date": "2024-12-31",
      "migration_guide": "docs/api/migrations/v1.md",
      "breaking_changes": [
        "Response format changed",
        "New authentication required"
      ]
    },
    "v2": {
      "version": "v2",
      "deprecated": false,
      "deprecation_message": null,
      "sunset_date": null,
      "migration_guide": "docs/api/migrations/v2.md",
      "breaking_changes": []
    }
  },
  "default_version": "v2"
}
```

### Middleware Configuration

```rust
use elif_http::middleware::versioning::{VersioningConfig, VersioningMiddleware};

let config = VersioningConfig::build()
    .strategy(VersionStrategy::UrlPath)
    .default_version(Some("v2".to_string()))
    .include_deprecation_headers(true)
    .strict_validation(true)
    .build_with_defaults();

let middleware = VersioningMiddleware::new(config);
```

## Advanced Features

### Custom Version Resolution

```rust
impl VersionedRouter<()> {
    pub fn custom_version_resolver<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&ElifRequest) -> Option<String> + Send + Sync + 'static,
    {
        // Custom version resolution logic
        self
    }
}
```

### Version-Specific Middleware

```rust
let v1_router = ElifRouter::new()
    .layer(legacy_auth_middleware()) // v1-specific middleware
    .get("/users", handle_users_v1);

let v2_router = ElifRouter::new()
    .layer(oauth2_middleware()) // v2-specific middleware
    .get("/users", handle_users_v2);
```

## Best Practices

### 1. Versioning Strategy

- **Use URL path versioning** for public APIs (easier for clients)
- **Use header versioning** for internal APIs (cleaner URLs)
- **Avoid query parameter versioning** for REST APIs
- **Use Accept header versioning** for content-type specific versions

### 2. Version Naming

- Use semantic versioning: `v1`, `v2`, `v3` or `1.0`, `1.1`, `2.0`
- Be consistent across your API
- Consider date-based versioning for rapidly changing APIs: `2024-01-15`

### 3. Deprecation Management

- Always provide migration guides
- Set realistic sunset dates (6-12 months minimum)
- Send deprecation warnings in response headers
- Monitor usage of deprecated versions

### 4. Error Handling

- Use version-aware error responses
- Include migration information for deprecated versions
- Provide clear error codes and messages
- Follow consistent error format across versions

### 5. Testing

- Test all supported versions
- Test version negotiation logic
- Test deprecation warnings
- Test migration paths

## Migration Example

### From v1 to v2

**v1 Response:**
```json
[
  {"id": 1, "name": "John", "email": "john@example.com"}
]
```

**v2 Response:**
```json
{
  "users": [
    {
      "id": 1,
      "name": "John",
      "email": "john@example.com",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ],
  "meta": {
    "total": 1,
    "page": 1,
    "per_page": 10
  }
}
```

### Migration Steps

1. **Update API version** in client requests
2. **Update response parsing** to handle new format
3. **Handle new error responses** with structured format
4. **Update date handling** for ISO 8601 format
5. **Test thoroughly** with new version

## Integration with OpenAPI

The versioning system automatically integrates with OpenAPI documentation:

- Each version generates separate OpenAPI specs
- Deprecation information included in documentation
- Migration guides linked from deprecated endpoints
- Interactive Swagger UI for each version

## Monitoring and Analytics

### Version Usage Tracking

```rust
// Add custom middleware to track version usage
async fn version_tracking_middleware(req: ElifRequest) -> Result<ElifRequest, HttpError> {
    if let Some(version) = req.api_version() {
        // Log version usage
        tracing::info!("API version {} used", version);
        
        // Send metrics to analytics service
        analytics::track_version_usage(version, &req);
    }
    
    Ok(req)
}
```

### Health Checks

```rust
async fn health_check(_req: ElifRequest) -> Result<ElifJson, HttpError> {
    Ok(ElifJson(json!({
        "status": "healthy",
        "api_versions": {
            "supported": ["v1", "v2", "v3"],
            "default": "v2",
            "deprecated": ["v1"]
        }
    })))
}
```

## Troubleshooting

### Common Issues

1. **Version not detected**
   - Check versioning strategy configuration
   - Verify request format (URL path, headers, etc.)
   - Check default version setting

2. **Deprecation headers not appearing**
   - Ensure `include_deprecation_headers` is true
   - Verify version is marked as deprecated
   - Check response processing middleware

3. **Migration guides not found**
   - Verify migration guide paths in configuration
   - Check file permissions
   - Ensure guide files exist

4. **OpenAPI generation fails**
   - Check route discovery configuration
   - Verify controller annotations
   - Check for circular dependencies

### Debug Mode

Enable debug logging to troubleshoot versioning issues:

```rust
tracing_subscriber::fmt()
    .with_env_filter("debug,elif_http::middleware::versioning=trace")
    .init();
```

## Contributing

When contributing to the API versioning system:

1. Add tests for new versioning strategies
2. Update documentation for new features
3. Ensure backward compatibility
4. Add migration guides for breaking changes
5. Update OpenAPI integration as needed

## Links

- [Example Implementation](../examples/api_versioning_demo.rs)
- [Integration Tests](../crates/elif-http/tests/api_versioning_tests.rs)
- [CLI Commands Reference](./CLI_REFERENCE.md#version-commands)
- [OpenAPI Documentation](./OPENAPI.md)