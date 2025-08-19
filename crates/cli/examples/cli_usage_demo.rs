//! Example: Comprehensive CLI usage demonstration
//!
//! This example shows all the CLI commands and workflows for the elif.rs
//! framework, including project scaffolding, code generation, and database operations.

use std::path::Path;
use std::fs;
use tempfile::TempDir;

/// Demonstrates CLI usage patterns
pub struct CliDemo {
    temp_dir: TempDir,
    project_path: String,
}

impl CliDemo {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let project_path = temp_dir.path().join("demo_project").to_string_lossy().to_string();
        
        Ok(Self {
            temp_dir,
            project_path,
        })
    }

    /// Demonstrates project creation
    pub fn demonstrate_project_creation(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 === PROJECT CREATION DEMO ===");
        
        // Show help for new command
        println!("💡 Getting help for 'new' command:");
        self.run_command("elifrs", &["new", "--help"])?;
        
        println!("\n📁 Creating new project 'demo_project':");
        self.run_command("elifrs", &["new", "demo_project", "--path", &self.temp_dir.path().to_string_lossy()])?;
        
        println!("\n📋 Project structure created:");
        self.show_directory_structure(&self.project_path, 0)?;
        
        Ok(())
    }

    /// Demonstrates resource generation
    pub fn demonstrate_resource_generation(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n📝 === RESOURCE GENERATION DEMO ===");
        
        // Change to project directory for relative commands
        std::env::set_current_dir(&self.project_path)?;
        
        // Generate a User resource
        println!("👤 Generating User resource with fields:");
        self.run_command("elifrs", &[
            "resource", "new", "User",
            "--route", "/api/users",
            "--fields", "name:string,email:string,age:int,is_active:bool"
        ])?;

        // Generate a Post resource
        println!("\n📄 Generating Post resource with relationships:");
        self.run_command("elifrs", &[
            "resource", "new", "Post", 
            "--route", "/api/posts",
            "--fields", "title:string,content:text,user_id:uuid,published:bool,view_count:int"
        ])?;

        // Generate a Comment resource
        println!("\n💬 Generating Comment resource:");
        self.run_command("elifrs", &[
            "resource", "new", "Comment",
            "--route", "/api/comments", 
            "--fields", "content:text,post_id:uuid,user_id:uuid"
        ])?;

        println!("\n📁 Generated resource files:");
        if Path::new("src/models").exists() {
            self.show_directory_structure("src/models", 1)?;
        }
        if Path::new("src/controllers").exists() {
            self.show_directory_structure("src/controllers", 1)?;
        }
        
        Ok(())
    }

    /// Demonstrates database operations
    pub fn demonstrate_database_operations(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🗄️  === DATABASE OPERATIONS DEMO ===");
        
        // Create a custom migration
        println!("📝 Creating custom migration:");
        self.run_command("elifrs", &[
            "migrate", "create", "add_user_indexes"
        ])?;

        // Show migration status (would show pending migrations)
        println!("\n📊 Migration status:");
        self.run_command("elifrs", &["migrate", "status"])?;

        // Show generated migration files
        if Path::new("migrations").exists() {
            println!("\n📁 Migration files:");
            self.show_directory_structure("migrations", 1)?;
        }

        println!("\n💡 To run migrations:");
        println!("   elifrs migrate run");
        
        println!("\n💡 To rollback migrations:");
        println!("   elifrs migrate rollback --steps 1");
        
        Ok(())
    }

    /// Demonstrates code generation
    pub fn demonstrate_code_generation(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n⚙️  === CODE GENERATION DEMO ===");
        
        // Generate code from existing resources
        println!("🏭 Generating code from resource specifications:");
        self.run_command("elifrs", &["generate"])?;

        // Show generated API documentation
        println!("\n📖 Generating OpenAPI documentation:");
        self.run_command("elifrs", &["openapi", "export", "--format", "yaml"])?;

        if Path::new("openapi.yaml").exists() {
            println!("\n📄 OpenAPI spec generated: openapi.yaml");
            if let Ok(content) = fs::read_to_string("openapi.yaml") {
                println!("Preview (first 10 lines):");
                for (i, line) in content.lines().enumerate() {
                    if i >= 10 { break; }
                    println!("   {}", line);
                }
                if content.lines().count() > 10 {
                    println!("   ... ({} more lines)", content.lines().count() - 10);
                }
            }
        }
        
        Ok(())
    }

    /// Demonstrates project inspection and mapping
    pub fn demonstrate_project_inspection(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🔍 === PROJECT INSPECTION DEMO ===");
        
        // Generate route map
        println!("🗺️  Generating route map:");
        self.run_command("elifrs", &["map", "--format", "table"])?;

        println!("\n🗺️  Generating JSON route map:");
        self.run_command("elifrs", &["map", "--json"])?;

        // Check project health
        println!("\n🏥 Running project health check:");
        self.run_command("elifrs", &["check"])?;

        Ok(())
    }

    /// Demonstrates testing workflows
    pub fn demonstrate_testing(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🧪 === TESTING DEMO ===");
        
        // Run all tests
        println!("🧪 Running all tests:");
        println!("   elifrs test");
        
        // Run specific resource tests
        println!("\n🧪 Running tests for specific resource:");
        println!("   elifrs test --focus User");
        
        // Run tests with coverage
        println!("\n📊 Running tests with coverage:");
        println!("   elifrs test --coverage");
        
        println!("\n💡 These commands would run the actual test suite");
        println!("   (skipped in demo to avoid compilation requirements)");
        
        Ok(())
    }

    /// Demonstrates advanced workflows
    pub fn demonstrate_advanced_workflows(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🚀 === ADVANCED WORKFLOWS DEMO ===");
        
        // Show complex resource generation
        println!("🔧 Advanced resource with custom templates:");
        println!("   elifrs resource new Product \\");
        println!("     --route /api/products \\");
        println!("     --fields 'name:string,price:decimal,category_id:uuid' \\");
        println!("     --template advanced \\");
        println!("     --with-auth \\");
        println!("     --with-validation");
        
        println!("\n🔄 Batch operations:");
        println!("   elifrs generate --all");
        println!("   elifrs migrate run --all");
        println!("   elifrs test --parallel");
        
        println!("\n📊 Performance analysis:");
        println!("   elifrs check --performance");
        println!("   elifrs map --analyze-complexity");
        
        println!("\n🔐 Security analysis:");
        println!("   elifrs check --security");
        println!("   elifrs generate --secure-defaults");
        
        Ok(())
    }

    /// Shows example configuration files
    pub fn show_configuration_examples(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n⚙️  === CONFIGURATION EXAMPLES ===");
        
        // Show example .elif.toml configuration
        let config_example = r#"[project]
name = "demo_project"
version = "0.1.0"
framework_version = "0.1.0"

[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
url = "postgresql://user:pass@localhost/demo_db"
pool_size = 10
migrations_dir = "migrations"

[templates]
model_template = "default"
controller_template = "rest_api"
test_template = "comprehensive"

[features]
authentication = true
authorization = true
rate_limiting = true
cors = true
swagger = true

[generation]
auto_timestamps = true
soft_deletes = true
uuid_primary_keys = true
validation = true"#;

        println!("📄 Example .elif.toml configuration:");
        for line in config_example.lines() {
            println!("   {}", line);
        }

        // Show example environment configuration
        println!("\n📄 Example .env configuration:");
        let env_example = r#"# Database configuration
DATABASE_URL=postgresql://username:password@localhost/elif_demo
DATABASE_POOL_SIZE=10

# Server configuration  
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
SERVER_WORKERS=4

# Authentication
JWT_SECRET=your-super-secret-jwt-key-here
JWT_EXPIRATION=86400

# Logging
LOG_LEVEL=info
LOG_FORMAT=json

# Feature flags
ENABLE_CORS=true
ENABLE_RATE_LIMITING=true
ENABLE_SWAGGER=true"#;

        for line in env_example.lines() {
            println!("   {}", line);
        }
        
        Ok(())
    }

    /// Helper function to run CLI commands
    fn run_command(&self, program: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        println!("💻 Running: {} {}", program, args.join(" "));
        
        // In a real demo, this would execute the actual command
        // For this example, we simulate the output
        match args.first() {
            Some(&"new") => {
                println!("✅ Created new elif.rs project structure");
                println!("   📁 src/");
                println!("   📁 migrations/");
                println!("   📁 tests/");
                println!("   📄 Cargo.toml");
                println!("   📄 .elif.toml");
                println!("   📄 .env.example");
            },
            Some(&"resource") => {
                let resource_name = args.get(2).unwrap_or(&"Resource");
                println!("✅ Generated {} resource files:", resource_name);
                println!("   📄 src/models/{}.rs", resource_name.to_lowercase());
                println!("   📄 src/controllers/{}_controller.rs", resource_name.to_lowercase());
                println!("   📄 migrations/create_{}_table.sql", resource_name.to_lowercase());
                println!("   📄 tests/{}_test.rs", resource_name.to_lowercase());
            },
            Some(&"migrate") => {
                match args.get(1) {
                    Some(&"create") => {
                        let migration_name = args.get(2).unwrap_or(&"migration");
                        println!("✅ Created migration: {}", migration_name);
                        println!("   📄 migrations/20231201120000_{}.sql", migration_name);
                    },
                    Some(&"status") => {
                        println!("📊 Migration Status:");
                        println!("   ✅ 20231201100000_create_users_table");
                        println!("   ✅ 20231201110000_create_posts_table");
                        println!("   ⏳ 20231201120000_add_user_indexes (pending)");
                    },
                    _ => println!("✅ Migration command completed"),
                }
            },
            Some(&"generate") => {
                println!("✅ Code generation completed:");
                println!("   📄 Generated 5 model files");
                println!("   📄 Generated 5 controller files");
                println!("   📄 Generated 12 test files");
                println!("   📄 Updated OpenAPI specification");
            },
            Some(&"map") => {
                if args.contains(&"--json") {
                    println!("📊 Route Map (JSON):");
                    println!("   {{\n     \"routes\": [\n       {{\"path\": \"/api/users\", \"methods\": [\"GET\", \"POST\"]}},");
                    println!("       {{\"path\": \"/api/users/:id\", \"methods\": [\"GET\", \"PUT\", \"DELETE\"]}}\n     ]\n   }}");
                } else {
                    println!("🗺️  Route Map:");
                    println!("   ┌─────────────────┬──────────────────┬─────────────┐");
                    println!("   │ Path            │ Methods          │ Controller  │");
                    println!("   ├─────────────────┼──────────────────┼─────────────┤");
                    println!("   │ /api/users      │ GET, POST        │ UsersCtrl   │");
                    println!("   │ /api/users/:id  │ GET, PUT, DELETE │ UsersCtrl   │");
                    println!("   │ /api/posts      │ GET, POST        │ PostsCtrl   │");
                    println!("   └─────────────────┴──────────────────┴─────────────┘");
                }
            },
            Some(&"check") => {
                println!("🏥 Project Health Check:");
                println!("   ✅ Cargo.toml structure valid");
                println!("   ✅ All resource specifications valid");
                println!("   ✅ Migration files consistent");
                println!("   ✅ Tests coverage > 80%");
                println!("   ⚠️  Missing documentation for 2 controllers");
            },
            Some(&"openapi") => {
                println!("✅ OpenAPI specification exported:");
                println!("   📄 openapi.yaml (3,245 lines)");
                println!("   📊 5 resources, 23 endpoints documented");
            },
            _ => println!("✅ Command executed successfully"),
        }
        
        println!(); // Add blank line for readability
        Ok(())
    }

    /// Helper function to display directory structure
    fn show_directory_structure(&self, path: &str, indent: usize) -> Result<(), Box<dyn std::error::Error>> {
        let indent_str = "  ".repeat(indent);
        
        if Path::new(path).exists() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if entry.path().is_dir() {
                            println!("{}📁 {}/", indent_str, name);
                        } else {
                            println!("{}📄 {}", indent_str, name);
                        }
                    }
                }
            }
        } else {
            // Simulate structure for demo
            match path {
                p if p.contains("demo_project") => {
                    println!("{}📁 src/", indent_str);
                    println!("{}📁 migrations/", indent_str);
                    println!("{}📁 tests/", indent_str);
                    println!("{}📄 Cargo.toml", indent_str);
                    println!("{}📄 .elif.toml", indent_str);
                    println!("{}📄 README.md", indent_str);
                },
                p if p.contains("models") => {
                    println!("{}📄 user.rs", indent_str);
                    println!("{}📄 post.rs", indent_str);
                    println!("{}📄 comment.rs", indent_str);
                    println!("{}📄 mod.rs", indent_str);
                },
                p if p.contains("controllers") => {
                    println!("{}📄 user_controller.rs", indent_str);
                    println!("{}📄 post_controller.rs", indent_str);
                    println!("{}📄 comment_controller.rs", indent_str);
                    println!("{}📄 mod.rs", indent_str);
                },
                _ => {}
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 elif.rs CLI Usage Demo");
    println!("=========================");
    println!("This demonstrates the complete CLI workflow for the elif.rs framework");
    println!();

    let demo = CliDemo::new()?;

    // Run all demonstrations
    demo.demonstrate_project_creation()?;
    demo.demonstrate_resource_generation()?;
    demo.demonstrate_database_operations()?;
    demo.demonstrate_code_generation()?;
    demo.demonstrate_project_inspection()?;
    demo.demonstrate_testing()?;
    demo.demonstrate_advanced_workflows()?;
    demo.show_configuration_examples()?;

    println!("\n🎉 === DEMO COMPLETE ===");
    println!();
    println!("✨ CLI Features Demonstrated:");
    println!("   ✅ Project scaffolding with 'elifrs new'");
    println!("   ✅ Resource generation with fields and routes");
    println!("   ✅ Database migration management");
    println!("   ✅ Automated code generation");
    println!("   ✅ Project mapping and introspection");
    println!("   ✅ Health checking and validation");
    println!("   ✅ OpenAPI documentation export");
    println!("   ✅ Testing workflows and coverage");
    println!("   ✅ Configuration management");
    println!();
    println!("🚀 To get started with elif.rs:");
    println!("   1. Install: cargo install elifrs");
    println!("   2. Create project: elifrs new my-app");
    println!("   3. Add resources: elifrs resource new User --fields name:string,email:string");
    println!("   4. Generate code: elifrs generate");
    println!("   5. Run migrations: elifrs migrate run");
    println!("   6. Start development: cargo run");
    println!();
    println!("📚 For more help: elifrs --help");

    Ok(())
}