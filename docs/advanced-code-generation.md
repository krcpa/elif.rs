# Advanced Code Generation with Module Integration

**Epic 6.5** - Complete implementation of advanced code generation capabilities that leverage the elif.rs module system to create production-ready code structures including APIs, CRUD operations, and services with proper dependency injection.

## Table of Contents

1. [Overview](#overview)
2. [Core Commands](#core-commands)
3. [Advanced Features](#advanced-features)
4. [Template Engine](#template-engine)
5. [Module Integration](#module-integration)
6. [Project Analysis](#project-analysis)
7. [Examples](#examples)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)

## Overview

The Advanced Code Generation system provides Laravel/Rails-level productivity for the elif.rs framework through intelligent code generation that understands your project structure, module relationships, and dependencies. It generates production-ready code with proper error handling, logging, testing, and documentation.

### Key Benefits

- **Laravel-Level Productivity**: Generate complete CRUD systems, APIs, and services with a single command
- **Module-Aware**: Automatically integrates with elif.rs module system and suggests optimal placement
- **Dependency Injection**: Automatically resolves and injects service dependencies
- **Production-Ready**: Generated code includes error handling, logging, testing, and documentation
- **Agent-Friendly**: Uses agent-editable markers for safe AI modifications
- **Template-Driven**: Fully customizable through Tera templates with custom filters

## Core Commands

### `elifrs make api <resource>`

Generate a complete REST API with CRUD operations for a resource.

```bash
# Basic API generation
elifrs make api User

# Advanced API with authentication and documentation
elifrs make api Product --version v2 --auth --validation --docs --module inventory
```

**Options:**
- `--version <VERSION>`: API version (default: v1)
- `--module <MODULE>`: Target module name
- `--auth`: Include authentication middleware
- `--validation`: Include request validation
- `--docs`: Generate OpenAPI documentation

**Generated Files:**
- API controller with full CRUD operations
- Route definitions with proper versioning
- Request/Response DTOs with validation
- OpenAPI documentation (if `--docs` enabled)
- Authentication middleware integration (if `--auth` enabled)

### `elifrs make crud <resource>`

Generate a complete CRUD system with model, controller, and service.

```bash
# Basic CRUD system
elifrs make crud Post

# Advanced CRUD with custom fields and relationships
elifrs make crud Product \
  --fields "name:string,price:decimal,description:text" \
  --relationships "Category:belongs_to,Reviews:has_many" \
  --module inventory \
  --migration --tests --factory
```

**Options:**
- `--fields <FIELDS>`: Resource fields (format: "name:type,email:string,age:int")
- `--relationships <RELATIONSHIPS>`: Model relationships (format: "User:belongs_to,Posts:has_many")
- `--module <MODULE>`: Target module name
- `--migration`: Generate database migration
- `--tests`: Generate test files
- `--factory`: Generate factory for testing

**Generated Files:**
- Model with relationships and validation
- Controller with full CRUD operations
- Service layer for business logic
- Database migration (if `--migration`)
- Factory for testing (if `--factory`)
- Unit and integration tests (if `--tests`)
- Request/Response classes

### `elifrs make service <name>`

Generate a business logic service with dependency injection.

```bash
# Basic service
elifrs make service Email

# Advanced service with dependencies and trait implementation
elifrs make service PaymentProcessor \
  --module payments \
  --dependencies "EmailService,DatabaseService,LoggingService" \
  --trait-impl PaymentProvider \
  --async-methods
```

**Options:**
- `--module <MODULE>`: Target module name
- `--dependencies <DEPENDENCIES>`: Service dependencies (comma-separated)
- `--trait-impl <TRAIT>`: Implement specific trait
- `--async-methods`: Use async methods

**Generated Files:**
- Service class with dependency injection
- Trait definition (if `--trait-impl`)
- Comprehensive error handling
- Logging integration
- Unit tests
- Documentation

### `elifrs make factory <model>`

Generate testing and seeding factories with relationships.

```bash
# Basic factory
elifrs make factory User

# Advanced factory with traits and relationships
elifrs make factory Order \
  --count 50 \
  --relationships "User,Product" \
  --traits "WithPayment,WithShipping"
```

**Options:**
- `--count <COUNT>`: Default number of instances (default: 10)
- `--relationships <RELATIONSHIPS>`: Related factories
- `--traits <TRAITS>`: Factory traits for different states

**Generated Files:**
- Factory class with builder pattern
- Relationship handling
- Trait-based states
- Faker integration
- Seeding helpers

## Advanced Features

### Module-Aware Generation

The system automatically analyzes your project structure and makes intelligent decisions:

```bash
# Automatically suggests appropriate module
elifrs make service UserNotification
# → Suggests placing in existing UserModule if available

# Analyzes existing dependencies
elifrs make service OrderProcessor --dependencies EmailService
# → Validates EmailService exists and adds proper imports
```

### Project Structure Analysis

The `ProjectAnalyzer` provides deep insight into your codebase:

- **Module Discovery**: Finds all modules and their components
- **Dependency Mapping**: Builds dependency graphs between services
- **Relationship Detection**: Identifies related components
- **Structure Validation**: Ensures proper module organization

### Intelligent Code Generation

Generated code includes production-ready features:

- **Error Handling**: Comprehensive error types and handling
- **Logging**: Structured logging with tracing
- **Testing**: Unit tests with proper mocking
- **Documentation**: Comprehensive inline documentation
- **Agent Markers**: Safe editing zones for AI agents

## Template Engine

Built on [Tera](https://tera.netlify.app/) with custom filters for Rust development:

### Custom Filters

- `pluralize`: Convert singular to plural forms
- `snake_case`: Convert to snake_case
- `pascal_case`: Convert to PascalCase
- `camel_case`: Convert to camelCase  
- `sql_type`: Convert Rust types to SQL types

### Template Structure

Templates use agent-editable markers for safe modification:

```rust
// <<<ELIF:BEGIN agent-editable:service-methods>>>
pub async fn process_payment(&self) -> ElifResult<PaymentResult> {
    // Custom implementation here
}
// <<<ELIF:END agent-editable:service-methods>>>
```

## Module Integration

### Automatic Module Registration

Services are automatically registered in the module system:

```rust
// Generated in src/modules/payments/services/mod.rs
pub mod payment_processor_service;
pub use payment_processor_service::PaymentProcessorService;
```

### Dependency Resolution

The system resolves dependencies automatically:

```rust
impl PaymentProcessorService {
    pub fn new(
        email_service: Arc<EmailService>,
        database_service: Arc<DatabaseService>,
    ) -> Self {
        // Constructor with injected dependencies
    }
}
```

### Module Structure Organization

Generated files follow elif.rs module conventions:

```
src/modules/payments/
├── mod.rs
├── controllers/
│   └── payment_controller.rs
├── services/
│   ├── mod.rs
│   └── payment_processor_service.rs
├── models/
│   └── payment.rs
└── tests/
    └── payment_tests.rs
```

## Project Analysis

### Structure Discovery

The `ProjectAnalyzer` scans your project to understand:

```rust
// Example analysis output
ProjectStructure {
    modules: {
        "auth": ModuleInfo { controllers: ["AuthController"], services: ["AuthService"] },
        "users": ModuleInfo { controllers: ["UserController"], services: ["UserService"] }
    },
    dependencies: {
        "auth": ["UserService", "EmailService"],
        "users": ["DatabaseService"]
    }
}
```

### Smart Suggestions

Based on analysis, the system provides intelligent suggestions:

- Module placement for new components
- Dependency relationships
- Naming conventions
- File organization

## Examples

### Complete E-commerce System

Generate a complete e-commerce system with modules:

```bash
# Create core modules
elifrs make module ProductModule --services ProductService --controllers ProductController
elifrs make module OrderModule --services OrderService --controllers OrderController
elifrs make module PaymentModule --services PaymentService --controllers PaymentController

# Generate CRUD systems
elifrs make crud Product \
  --fields "name:string,price:decimal,description:text,sku:string" \
  --relationships "Category:belongs_to,Reviews:has_many" \
  --module product \
  --migration --tests --factory

elifrs make crud Order \
  --fields "total:decimal,status:string,shipped_at:timestamp" \
  --relationships "User:belongs_to,Products:belongs_to_many" \
  --module order \
  --migration --tests --factory

# Generate business logic services  
elifrs make service PaymentProcessor \
  --module payment \
  --dependencies "OrderService,EmailService" \
  --async-methods

elifrs make service InventoryManager \
  --module product \
  --dependencies "ProductService,WarehouseService" \
  --async-methods

# Generate APIs
elifrs make api Product --version v1 --auth --docs --module product
elifrs make api Order --version v1 --auth --docs --module order

# Generate factories for testing
elifrs make factory Product --count 100 --relationships "Category" --traits "Featured,OnSale"
elifrs make factory Order --count 50 --relationships "User,Product" --traits "Completed,Pending"
```

### Microservice Generation

Generate a complete microservice:

```bash
# Create service architecture
elifrs make service UserAuthenticationService \
  --dependencies "DatabaseService,TokenService,EmailService" \
  --trait-impl AuthenticationProvider \
  --async-methods

elifrs make service TokenValidationService \
  --dependencies "RedisService,ConfigService" \
  --trait-impl TokenValidator \
  --async-methods

# Generate API endpoints
elifrs make api Auth --version v1 --auth --validation --docs

# Generate factories and tests
elifrs make factory User --traits "Verified,Premium,Admin"
```

## Best Practices

### 1. Module Organization

- Use descriptive module names ending with "Module"
- Group related functionality together
- Keep modules focused and cohesive

```bash
# Good: Focused modules
elifrs make module UserAccountModule
elifrs make module PaymentProcessingModule

# Avoid: Overly broad modules
elifrs make module BusinessLogicModule
```

### 2. Service Dependencies

- Declare dependencies explicitly
- Use dependency injection consistently
- Avoid circular dependencies

```bash
# Good: Clear dependency chain
elifrs make service OrderProcessor --dependencies "PaymentService,InventoryService"

# Good: Interface-based dependencies
elifrs make service EmailSender --trait-impl NotificationProvider
```

### 3. Factory Design

- Use meaningful traits for different states
- Include realistic data generation
- Consider performance for large datasets

```bash
# Good: State-based traits
elifrs make factory Order --traits "Completed,Cancelled,Processing"
```

### 4. API Versioning

- Always version your APIs
- Use semantic versioning
- Document breaking changes

```bash
# Good: Explicit versioning
elifrs make api User --version v2 --docs
```

## Troubleshooting

### Common Issues

**1. Module Not Found Error**

```
Error: Module 'users' not found
```

**Solution**: Ensure the module exists or create it first:
```bash
elifrs make module UserModule
```

**2. Dependency Resolution Error**

```
Error: Service 'EmailService' not found in project
```

**Solution**: Generate the required service first:
```bash
elifrs make service Email --async-methods
```

**3. Template Rendering Error**

```
Error: Template rendering failed
```

**Solution**: Check template syntax and ensure all required variables are provided.

### Debugging

Enable verbose logging to see detailed generation process:

```bash
RUST_LOG=debug elifrs make service PaymentProcessor --dependencies EmailService
```

### File Conflicts

Generated files use agent-editable markers. If you modify generated code:

1. Keep modifications within agent-editable sections
2. Back up custom changes before regenerating
3. Use version control to track modifications

### Performance Considerations

For large projects:

- Use specific module targeting to avoid full project analysis
- Consider batching multiple generations
- Use `--no-tests` flag for faster generation during development

## Integration with Existing CLI Features

The advanced code generation integrates seamlessly with existing elif.rs CLI features:

### Testing Integration

```bash
# Generate code with tests
elifrs make crud User --tests

# Run generated tests
elifrs test --module user
```

### Database Integration

```bash
# Generate with migrations
elifrs make crud Product --migration

# Run migrations
elifrs migrate up
```

### Development Server

```bash
# Generate API
elifrs make api User --docs

# Start development server with hot reload
elifrs dev --port 3000
```

This advanced code generation system brings Laravel-level productivity to elif.rs while maintaining the framework's focus on simplicity, performance, and developer experience. The module-aware generation and intelligent project analysis ensure that generated code integrates seamlessly with your existing application architecture.