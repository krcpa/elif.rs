/*!
# elif-openapi

OpenAPI 3.0 specification generation for elif.rs framework.

This crate provides automatic API documentation generation from annotated Rust code,
with support for interactive Swagger UI and multiple export formats.

## Features

- Automatic OpenAPI 3.0 specification generation
- Route and endpoint discovery from framework types
- Request/response schema extraction from Rust structs
- Authentication scheme documentation
- Interactive Swagger UI integration
- Multiple export formats (Postman, Insomnia)

## Usage

```rust,no_run
use elif_openapi::{OpenApiGenerator, OpenApiConfig};

let mut generator = OpenApiGenerator::new(OpenApiConfig::default());
let routes = vec![]; // Your route metadata here
let spec = generator.generate(&routes).unwrap();
```
*/

// Re-export main types
pub use crate::{
    config::OpenApiConfig,
    error::{OpenApiError, OpenApiResult},
    generator::OpenApiGenerator,
    macros::OpenApiSchema,
    schema::{SchemaGenerator, TypeSchema},
    specification::OpenApiSpec,
    swagger::SwaggerUi,
};

// Re-export the derive macro from the proc-macro crate
pub use elif_openapi_derive::OpenApiSchema;

// Core modules
pub mod config;
pub mod error;
pub mod generator;
pub mod specification;

// Schema generation
pub mod macros;
pub mod schema;

// Route and endpoint discovery
pub mod discovery;
pub mod endpoints;

// Export functionality
pub mod export;

// Interactive documentation
pub mod swagger;

// Utilities
pub mod utils;

// Test utilities
#[cfg(test)]
mod test_utils;
