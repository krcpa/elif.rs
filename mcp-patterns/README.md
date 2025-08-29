# elif.rs MCP Server Patterns

This directory contains JSON pattern definitions for the MCP (Model Context Protocol) server, documenting 8+ core elif.rs patterns based on **actual codebase implementation**.

## 🎯 Pattern Categories

### **Production-Ready Patterns** (High Priority)

#### 1. [Declarative Controllers](./declarative_controllers.json)
- **Category**: HTTP
- **Stability**: Production
- **Description**: 70% boilerplate reduction through derive macros for REST API routing
- **Key Features**: `#[controller("/path")]`, `#[get]`, `#[post]`, `#[middleware]`, `#[param]` attributes
- **Source**: `/examples/declarative_controller_example.rs`, `crates/elif-http-derive/`

#### 2. [Middleware V2 System](./middleware_v2_system.json)
- **Category**: HTTP  
- **Stability**: Production
- **Description**: Laravel-inspired middleware system with handle(request, next) pattern
- **Key Features**: `NextFuture`, pre/post processing, pipeline composition, conditional middleware
- **Source**: `crates/elif-http/src/middleware/v2.rs`, `/docs/middleware/examples/`

#### 3. [ORM Models](./orm_models.json)
- **Category**: Database
- **Stability**: Production
- **Description**: Django/Laravel-inspired ORM with Model trait, query builder, relationships
- **Key Features**: `Model` trait, relationships, migrations, query builder, factories
- **Source**: `crates/orm/examples/advanced_queries.rs`, `/examples/controllers/user_model.rs`

#### 4. [HTTP Server Core](./http_server_core.json)  
- **Category**: HTTP
- **Stability**: Production
- **Description**: One-line server setup and Laravel-style response builders
- **Key Features**: `Server::new().listen()`, `Response::json()`, `ElifRequest`/`ElifResponse`
- **Source**: `crates/elif-http/src/lib.rs`, `/examples/user_controller_example.rs`

#### 5. [Dependency Injection](./dependency_injection.json)
- **Category**: Architecture
- **Stability**: Production  
- **Description**: NestJS-inspired DI with IoC container, service lifetimes, module system
- **Key Features**: `#[module]`, providers, controllers, imports, service lifetimes
- **Source**: `crates/core/src/container/examples.rs`, `/examples/demo-dsl-showcase.rs`

### **Stable Patterns** (Medium Priority)

#### 6. [Service-Builder Pattern](./service_builder_pattern.json)
- **Category**: Architecture
- **Stability**: Stable
- **Description**: Builder pattern for configuration objects (not hot paths)
- **Key Features**: `#[builder]` macro, `build_with_defaults()`, optional/default fields
- **Source**: Framework guidelines, service-builder 0.3.0

#### 7. [Error Handling](./error_handling.json)
- **Category**: HTTP
- **Stability**: Production
- **Description**: Structured error responses with HttpResult and custom error types
- **Key Features**: `HttpResult<ElifResponse>`, error JSON format, validation errors
- **Source**: Framework convention, `crates/elif-http/src/errors/`

#### 8. [Testing Framework](./testing_framework.json)
- **Category**: Testing
- **Stability**: Stable
- **Description**: Framework-native testing with TestClient, integration tests, UI tests
- **Key Features**: `TestClient`, integration test patterns, trybuild UI tests
- **Source**: `crates/elif-testing/`, `crates/elif-http/tests/proper_integration_tests.rs`

## 📁 Pattern Sources from Codebase

Each pattern JSON includes:

- **Real code examples** from the actual codebase
- **File source references** with line numbers for easy navigation  
- **Working implementations** from `examples/` directory
- **Version info** and stability markers (production/stable/development)
- **Prerequisites** with exact dependencies and imports
- **Advanced examples** showing real-world usage patterns
- **Common mistakes** and solutions from development experience

## 📋 JSON Pattern Format

Each pattern follows this enhanced structure:

```json
{
  "name": "pattern_name",
  "title": "Human Readable Title", 
  "category": "http|database|architecture|testing",
  "version": "0.8.0",
  "stability": "production|stable|development",
  "description": "Brief description with key benefits",
  "tags": ["relevant", "tags"],
  
  "basic_example": {
    "title": "Example Title",
    "code": "// Working code from codebase",
    "explanation": "What this code demonstrates",
    "file_source": "/path/to/source/file.rs:line-range"
  },
  
  "advanced_example": {
    "title": "Advanced Usage",
    "code": "// Complex real-world example"
  },
  
  "when_to_use": ["Use case 1", "Use case 2"],
  "benefits": ["Benefit 1", "Benefit 2"],
  
  "source_files": [
    {
      "path": "crates/path/to/implementation.rs",
      "description": "What this file contains"
    }
  ],
  
  "related_patterns": ["other_patterns"],
  "common_mistakes": [
    {
      "mistake": "What goes wrong",
      "solution": "How to fix it"
    }
  ],
  
  "prerequisites": {
    "dependencies": ["elif-http = \"0.8.0\""],
    "imports": ["use elif_http::*;"]
  }
}
```

## 🎯 Usage for MCP Server

These patterns are designed for AI agents to:

1. **Understand elif.rs patterns** through real codebase examples
2. **Generate idiomatic code** following established conventions  
3. **Avoid common mistakes** with proven solutions
4. **Navigate the codebase** using file source references
5. **Use correct versions** with stability and version information

## 🔧 Pattern Validation

All patterns have been validated against:
- ✅ **Actual codebase implementation** (analyzed from 22 crates + examples)
- ✅ **Working code examples** that compile and run
- ✅ **Current version compatibility** (elif.rs 0.8.0+)
- ✅ **Real file sources** with accurate line references
- ✅ **Production usage** in framework examples

---

**Total Patterns**: 8+ core patterns  
**Framework Version**: elif.rs 0.8.0+  
**Last Updated**: 2025-08-29  
**Quality**: Based on comprehensive codebase analysis (22 crates + examples)