# Code Generators

Scaffold complete features with consistent conventions and intelligent templates. elif.rs generators understand your module system, relationships, and dependencies to create production-ready code.

## Why elif.rs Generators Rock

**🎯 Intelligent Templates**: Context-aware code generation based on your options
**🔧 Module-Aware**: Integrates seamlessly with elif.rs module system  
**🏭 Factory Integration**: Generate realistic test data automatically
**📦 Complete Features**: Generate models, controllers, services, tests in one command
**⚡ Laravel Familiar**: Commands you know, conventions you love

## Quick Start

```bash
# Generate a database seeder
elifrs make:seeder UserSeeder --table users --factory

# Generate a complete module (coming soon)
elifrs make:module UserModule --providers=UserService --controllers=UserController

# Generate API resources (coming soon)
elifrs make:api User --with-tests --with-factory
```

## Database Seeders

### `elifrs make:seeder <name>`

Generate intelligent database seeders with dependency resolution and factory integration.

**Basic Seeder:**
```bash
elifrs make:seeder AdminSeeder
```

**Table-Targeted Seeder:**
```bash
elifrs make:seeder CategorySeeder --table categories
```

**Factory-Powered Seeder:**
```bash
elifrs make:seeder ProductSeeder --table products --factory
```

### Generated Templates

**Basic Template** (Custom Logic):
```rust
use elif_orm::Database;

pub struct AdminSeeder;

impl AdminSeeder {
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        println!("🌱 Running AdminSeeder...");
        
        // Add your seeding logic here
        // Example:
        // db.query("INSERT INTO users (name, email) VALUES ($1, $2)")
        //     .bind("Admin User")
        //     .bind("admin@example.com")
        //     .execute()
        //     .await?;
        
        println!("✅ AdminSeeder completed");
        Ok(())
    }
    
    pub fn dependencies() -> Vec<&'static str> {
        vec![]  // Add dependencies here
    }
}
```

**Table-Targeted Template** (Sample Data):
```rust
use elif_orm::Database;
use serde_json::json;

pub struct CategorySeeder;

impl CategorySeeder {
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        println!("🌱 Running CategorySeeder...");
        
        // Seed categories table with sample data
        let records = vec![
            json!({
                "name": "Sample Record 1",
                "created_at": chrono::Utc::now(),
                "updated_at": chrono::Utc::now(),
            }),
            json!({
                "name": "Sample Record 2", 
                "created_at": chrono::Utc::now(),
                "updated_at": chrono::Utc::now(),
            }),
        ];
        
        for record in records {
            db.query("INSERT INTO categories (name, created_at, updated_at) VALUES ($1, $2, $3)")
                .bind(record["name"].as_str().unwrap())
                .bind(record["created_at"].as_str().unwrap())
                .bind(record["updated_at"].as_str().unwrap())
                .execute()
                .await?;
        }
        
        println!("✅ CategorySeeder completed");
        Ok(())
    }
    
    pub fn dependencies() -> Vec<&'static str> {
        vec![]
    }
}
```

**Factory Template** (Realistic Data):
```rust
use elif_orm::{Database, factory::Factory};
use serde_json::json;

pub struct ProductSeeder;

impl ProductSeeder {
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        println!("🌱 Running ProductSeeder...");
        
        // Generate test data using factory pattern
        let factory = Factory::new();
        
        // Seed products table with factory-generated data
        for i in 1..=10 {
            let data = factory
                .for_table("products")
                .with_attributes(json!({
                    "name": format!("Test products Entry {}", i),
                    "created_at": chrono::Utc::now(),
                    "updated_at": chrono::Utc::now(),
                }))
                .build();
            
            factory.insert(db, "products", &data).await?;
        }
        
        println!("✅ ProductSeeder completed - inserted 10 products records");
        Ok(())
    }
    
    pub fn dependencies() -> Vec<&'static str> {
        vec![]
    }
}
```

### Generator Features

**Smart File Placement:**
- Creates `database/seeders/` directory if needed
- Places seeders in consistent location
- Updates `mod.rs` automatically for module declarations

**Dependency-Ready:**
- All generated seeders include `dependencies()` method
- Ready for complex dependency chains
- Supports intelligent seeder ordering

**Environment Integration:**
- Works with all elif.rs environments (dev, test, staging, prod)
- Safe production controls built-in
- Environment-aware data generation patterns

## Module Generators (Epic 6 Phase 1 - Coming Soon)

### `elifrs make:module <name>`

Generate complete modules with providers, controllers, and services.

```bash
# Basic module
elifrs make:module UserModule

# Module with providers and controllers
elifrs make:module UserModule --providers=UserService --controllers=UserController

# Complex module with multiple components
elifrs make:module BlogModule \
    --providers=PostService,CommentService \
    --controllers=PostController,CommentController \
    --services=EmailService
```

## API Generators (Epic 6 Phase 5 - Coming Soon)

### `elifrs make:api <resource>`

Generate complete REST APIs with models, controllers, tests, and documentation.

```bash
# Basic API
elifrs make:api User

# API with relationships
elifrs make:api Post --belongs-to=User --has-many=Comment

# Complete API with all features
elifrs make:api Product \
    --with-tests \
    --with-factory \
    --with-seeder \
    --auth=jwt
```

## CRUD Generators (Epic 6 Phase 5 - Coming Soon)

### `elifrs make:crud <resource>`

Generate complete CRUD operations with all supporting files.

```bash
# Basic CRUD
elifrs make:crud User

# CRUD with soft deletes
elifrs make:crud Post --soft-deletes

# CRUD with validation and tests
elifrs make:crud Product --with-validation --with-tests
```

## Service Generators (Epic 6 Phase 5 - Coming Soon)

### `elifrs make:service <name>`

Generate business logic services with dependency injection.

```bash
# Basic service
elifrs make:service EmailService

# Service with trait implementation
elifrs make:service PaymentService --trait=PaymentServiceTrait

# Service with module integration
elifrs make:service UserService --module=UserModule
```

## Factory Generators (Epic 6 Phase 5 - Coming Soon)

### `elifrs make:factory <model>`

Generate model factories for testing and seeding.

```bash
# Basic factory
elifrs make:factory User

# Factory with relationships
elifrs make:factory Post --belongs-to=User

# Factory with states
elifrs make:factory User --states=admin,verified,suspended
```

## Generation Best Practices

### 1. **Use Descriptive Names**

```bash
# ✅ Good: Clear, descriptive names
elifrs make:seeder AdminUserSeeder
elifrs make:seeder BlogCategorySeeder
elifrs make:seeder EcommerceProductSeeder

# ❌ Bad: Vague names
elifrs make:seeder TestSeeder
elifrs make:seeder DataSeeder
elifrs make:seeder Seeder1
```

### 2. **Choose the Right Template**

```bash
# ✅ Basic: When you need full control
elifrs make:seeder ComplexBusinessLogicSeeder

# ✅ Table: When you need simple sample data
elifrs make:seeder QuickTestSeeder --table users

# ✅ Factory: When you need realistic data at scale  
elifrs make:seeder RealisticUserSeeder --table users --factory
```

### 3. **Plan Dependencies**

```rust
// ✅ Good: Plan dependency chains
// RoleSeeder (no dependencies)
// ↓
// UserSeeder (depends on RoleSeeder) 
// ↓
// PostSeeder (depends on UserSeeder)
// ↓ 
// CommentSeeder (depends on UserSeeder, PostSeeder)

pub fn dependencies() -> Vec<&'static str> {
    vec!["RoleSeeder"]  // Clear, explicit dependency
}
```

### 4. **Environment-Aware Generation**

When generating seeders, consider different environments:

```rust
// Generate environment-aware seeders
pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let env = std::env::var("ELIF_ENV").unwrap_or("development".to_string());
    
    match env.as_str() {
        "test" => {
            // Minimal data for fast tests
            generate_test_data(db, 5).await?;
        }
        "development" => {
            // Rich data for development
            generate_development_data(db, 50).await?;
        }
        "staging" => {
            // Production-like data
            generate_staging_data(db, 200).await?;
        }
        _ => {
            // No automatic production seeding
            println!("⚠️  No automatic seeding for production environment");
        }
    }
    
    Ok(())
}
```

## Generator Customization

### Template Locations

```
crates/cli/templates/
├── seeder.stub           # Basic seeder template
├── seeder_table.stub     # Table-targeted template  
├── seeder_factory.stub   # Factory-integrated template
├── module.stub           # Module template (coming soon)
├── controller.stub       # Controller template
└── service.stub          # Service template (coming soon)
```

### Extending Generators

Create custom generators by extending the template system:

```rust
// Custom generator example (pseudo-code)
pub struct CustomSeederGenerator {
    template_engine: TemplateEngine,
    project_analyzer: ProjectAnalyzer,
}

impl CustomSeederGenerator {
    pub fn generate(&self, spec: &SeederSpec) -> Result<String, ElifError> {
        let template = match (spec.table, spec.factory) {
            (Some(_), true) => "custom_factory_seeder.stub",
            (Some(_), false) => "custom_table_seeder.stub", 
            (None, _) => "custom_basic_seeder.stub",
        };
        
        self.template_engine.render(template, &spec.context())
    }
}
```

## Development Workflow with Generators

### Feature Development Workflow

```bash
# 1. Plan your feature
# Need: User management with roles and permissions

# 2. Generate seeders in dependency order
elifrs make:seeder RoleSeeder --table roles
elifrs make:seeder PermissionSeeder --table permissions  
elifrs make:seeder UserSeeder --table users --factory

# 3. Edit dependencies in generated seeders
# UserSeeder dependencies: vec!["RoleSeeder", "PermissionSeeder"]

# 4. Test seeding
elifrs db:fresh --seed

# 5. Generate additional components (coming soon)
# elifrs make:module UserModule --providers=UserService --controllers=UserController
# elifrs make:api User --with-tests
```

### Team Collaboration

```bash
# Team member pulls new generators
git pull origin main

# Regenerate if templates changed  
elifrs make:seeder UserSeeder --table users --factory --force

# Update database with new structure
elifrs db:fresh --seed
```

## Troubleshooting Generators

### Common Issues

**File Already Exists:**
```bash
❌ Error: Seeder 'UserSeeder' already exists

💡 Fix: Use --force to overwrite or choose different name
elifrs make:seeder UserSeeder --force
```

**Invalid Names:**
```bash
❌ Error: Seeder name must end with 'Seeder'

💡 Fix: Use proper naming convention
elifrs make:seeder User  # Bad
elifrs make:seeder UserSeeder  # Good
```

**Directory Permissions:**
```bash
❌ Error: Cannot create database/seeders directory

💡 Fix: Check directory permissions or run with proper permissions
sudo elifrs make:seeder UserSeeder  # If needed
```

### Best Practices for Generated Code

1. **Review Generated Code**: Always review and customize generated code
2. **Add Proper Dependencies**: Define seeder dependencies explicitly
3. **Environment Awareness**: Make seeders environment-aware
4. **Error Handling**: Add proper error handling to generated code
5. **Documentation**: Document complex seeding logic

## Comparison: elif.rs vs Other Frameworks

| Feature | elif.rs | Laravel | Rails | NestJS |
|---------|---------|---------|-------|---------|
| **Seeder Generation** | ✅ 3 templates | ✅ Basic | ✅ Basic | ❌ Manual |
| **Dependency Resolution** | ✅ Automatic | ❌ Manual | ❌ Manual | ❌ Manual |
| **Factory Integration** | ✅ Built-in | ✅ Separate | ✅ Separate | ❌ Manual |
| **Module Awareness** | ✅ Coming | ❌ No | ❌ No | ✅ Yes |
| **Type Safety** | ✅ Compile-time | ❌ Runtime | ❌ Runtime | ✅ Compile-time |
| **Template System** | ✅ Smart | ✅ Basic | ✅ Basic | ✅ Advanced |

## What's Next?

**Current (Epic 6 Phase 3):**
- ✅ `make:seeder` with intelligent templates
- ✅ Dependency resolution system
- ✅ Factory integration

**Coming Soon:**
- **Phase 4**: Testing generators with module awareness
- **Phase 5**: Complete API and CRUD generators  
- **Phase 6**: Advanced service and factory generators

**Try It Now:**
```bash
# Install elif.rs and try the generator
elifrs new my-project --template web
cd my-project

# Generate your first intelligent seeder
elifrs make:seeder UserSeeder --table users --factory

# See the magic
cat database/seeders/user_seeder.rs
elifrs db:fresh --seed
```

elif.rs generators: **Intelligent**, **Consistent**, **Powerful**. Code generation the way it should be! 🚀✨

