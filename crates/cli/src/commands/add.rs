use elif_core::ElifError;
use std::fs;
use std::path::Path;

/// Add a new module with intelligent defaults
pub async fn module(
    name: &str,
    providers: Option<&str>,
    controllers: Option<&str>,
    services: Option<&str>,
) -> Result<(), ElifError> {
    println!("🧩 Adding module '{}'", name);

    // Validate module name
    if !is_valid_module_name(name) {
        return Err(ElifError::validation(
            "Module name must be PascalCase and end with 'Module' (e.g., UserModule)",
        ));
    }

    let module_path = Path::new("src/modules").join(format!("{}.rs", to_snake_case(name)));
    let modules_mod_path = Path::new("src/modules/mod.rs");

    // Check if src/modules directory exists
    if !Path::new("src/modules").exists() {
        println!("📁 Creating modules directory...");
        fs::create_dir_all("src/modules")?;
        fs::write(modules_mod_path, "// Module declarations\n")?;
    }

    // Generate module content
    let module_content = generate_module_content(
        name,
        providers
            .map(|p| p.split(',').collect())
            .unwrap_or_default(),
        controllers
            .map(|c| c.split(',').collect())
            .unwrap_or_default(),
        services.map(|s| s.split(',').collect()).unwrap_or_default(),
    )?;

    // Write module file
    fs::write(&module_path, module_content)?;

    // Update modules/mod.rs
    update_modules_mod(modules_mod_path, name)?;

    // Generate associated controllers and services if specified
    if let Some(controllers) = controllers {
        for controller in controllers.split(',') {
            let controller = controller.trim();
            if !controller.is_empty() {
                generate_controller_file(controller, name).await?;
            }
        }
    }

    if let Some(services) = services {
        for service in services.split(',') {
            let service = service.trim();
            if !service.is_empty() {
                generate_service_file(service, name).await?;
            }
        }
    }

    println!("✅ Successfully added module '{}'", name);
    println!("\n📖 Next steps:");
    println!("   elifrs inspect modules --graph");
    if controllers.is_some() || services.is_some() {
        println!("   elifrs add controller MyController --to={}", name);
    }

    Ok(())
}

/// Add a service to an existing module
pub async fn service(
    name: &str,
    to_module: &str,
    _trait_impl: Option<&str>,
) -> Result<(), ElifError> {
    println!("⚙️ Adding service '{}' to module '{}'", name, to_module);

    // Validate service name
    if !name.ends_with("Service") {
        return Err(ElifError::validation(
            "Service name should end with 'Service' (e.g., EmailService)",
        ));
    }

    // Check if target module exists
    let module_file = Path::new("src/modules").join(format!("{}.rs", to_snake_case(to_module)));
    if !module_file.exists() {
        return Err(ElifError::validation(format!(
            "Module '{}' not found. Create it first with: elifrs add module {}",
            to_module, to_module
        )));
    }

    // Generate service file
    generate_service_file(name, to_module).await?;

    // Update module to include service
    update_module_with_service(&module_file, name)?;

    println!(
        "✅ Successfully added service '{}' to module '{}'",
        name, to_module
    );

    Ok(())
}

/// Add a controller to an existing module
pub async fn controller(name: &str, to_module: &str, crud: bool) -> Result<(), ElifError> {
    println!("🎮 Adding controller '{}' to module '{}'", name, to_module);

    // Validate controller name
    if !name.ends_with("Controller") {
        return Err(ElifError::validation(
            "Controller name should end with 'Controller' (e.g., UserController)",
        ));
    }

    // Check if target module exists
    let module_file = Path::new("src/modules").join(format!("{}.rs", to_snake_case(to_module)));
    if !module_file.exists() {
        return Err(ElifError::validation(format!(
            "Module '{}' not found. Create it first with: elifrs add module {}",
            to_module, to_module
        )));
    }

    // Generate controller file
    generate_controller_file(name, to_module).await?;

    // Update module to include controller
    update_module_with_controller(&module_file, name)?;

    println!(
        "✅ Successfully added controller '{}' to module '{}'",
        name, to_module
    );

    if crud {
        println!("📋 Generated CRUD methods: index, show, create, update, destroy");
    }

    Ok(())
}

/// Add middleware to the project
pub async fn middleware(name: &str, to_module: Option<&str>, debug: bool) -> Result<(), ElifError> {
    println!("🔧 Adding middleware '{}'", name);

    if !name.ends_with("Middleware") {
        return Err(ElifError::validation(
            "Middleware name should end with 'Middleware' (e.g., AuthMiddleware)",
        ));
    }

    // Generate middleware file
    generate_middleware_file(name, debug).await?;

    // If module specified, add to module
    if let Some(module) = to_module {
        let module_file = Path::new("src/modules").join(format!("{}.rs", to_snake_case(module)));
        if module_file.exists() {
            update_module_with_middleware(&module_file, name)?;
        }
    }

    println!("✅ Successfully added middleware '{}'", name);
    println!("\n📖 Usage:");
    println!(
        "   Add to controller: #[middleware(\"{}\")]",
        to_snake_case(name)
    );

    Ok(())
}

/// Add a migration file
pub async fn migration(name: &str) -> Result<(), ElifError> {
    // Delegate to existing migrate functionality for now
    crate::commands::migrate::create(name).await
}

/// Add a seeder file  
pub async fn seeder(name: &str) -> Result<(), ElifError> {
    seeder_with_options(name, None, false).await
}

/// Add a seeder file with enhanced options (for make:seeder command)
pub async fn seeder_with_options(name: &str, table: Option<&str>, factory: bool) -> Result<(), ElifError> {
    println!("🌱 Creating seeder '{}'", name);

    let seeder_dir = Path::new("database/seeders");
    if !seeder_dir.exists() {
        fs::create_dir_all(seeder_dir)?;
        fs::write(seeder_dir.join("mod.rs"), "// Database seeder declarations\n")?;
    }

    // Enhanced seeder template with factory integration and table targeting
    let seeder_content = if factory && table.is_some() {
        let table_name = table.unwrap();
        format!(
            r#"use elif_orm::{{Database, factory::Factory}};
use serde_json::json;

pub struct {}Seeder;

impl {}Seeder {{
    /// Run the database seeder
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {{
        println!("🌱 Running {} seeder...");
        
        // Generate test data using factory pattern
        let factory = Factory::new();
        
        // Seed {} table with factory-generated data
        for i in 1..=10 {{
            let data = factory
                .for_table("{}")
                .with_attributes(json!({{
                    "name": format!("Test {} Entry {{}}", i),
                    "created_at": chrono::Utc::now(),
                    "updated_at": chrono::Utc::now(),
                }}))
                .build();
            
            factory.insert(db, "{}", &data).await?;
        }}
        
        println!("✅ {} seeder completed - inserted 10 {} records");
        Ok(())
    }}
    
    /// Get seeding dependencies (run these seeders first)  
    pub fn dependencies() -> Vec<&'static str> {{
        vec![]
    }}
}}
"#,
            name, name, name, table_name, table_name, table_name, table_name, name, table_name
        )
    } else if table.is_some() {
        let table_name = table.unwrap();
        format!(
            r#"use elif_orm::Database;
use serde_json::json;

pub struct {}Seeder;

impl {}Seeder {{
    /// Run the database seeder
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {{
        println!("🌱 Running {} seeder...");
        
        // Seed {} table with sample data
        let records = vec![
            json!({{
                "name": "Sample Record 1",
                "created_at": chrono::Utc::now(),
                "updated_at": chrono::Utc::now(),
            }}),
            json!({{
                "name": "Sample Record 2", 
                "created_at": chrono::Utc::now(),
                "updated_at": chrono::Utc::now(),
            }}),
        ];
        
        for record in [("Sample Record 1"), ("Sample Record 2")] {
            db.query("INSERT INTO {} (name, created_at, updated_at) VALUES ($1, $2, $3)")
                .bind(record)
                .bind(chrono::Utc::now())
                .bind(chrono::Utc::now())
                .execute()
                .await?;
        }
        
        println!("✅ {} seeder completed");
        Ok(())
    }}
    
    /// Get seeding dependencies (run these seeders first)
    pub fn dependencies() -> Vec<&'static str> {{
        vec![]
    }}
}}
"#,
            name, name, name, table_name, table_name, name
        )
    } else {
        format!(
            r#"use elif_orm::Database;
use serde_json::json;

pub struct {}Seeder;

impl {}Seeder {{
    /// Run the database seeder
    pub async fn run(db: &Database) -> Result<(), Box<dyn std::error::Error>> {{
        println!("🌱 Running {} seeder...");
        
        // Add your seeding logic here
        // Example:
        // db.query("INSERT INTO users (name, email) VALUES ($1, $2)")
        //     .bind("John Doe")
        //     .bind("john@example.com")
        //     .execute()
        //     .await?;
        
        println!("✅ {} seeder completed");
        Ok(())
    }}
    
    /// Get seeding dependencies (run these seeders first)
    pub fn dependencies() -> Vec<&'static str> {{
        vec![]
    }}
}}
"#,
            name, name, name, name
        )
    };

    let seeder_file = seeder_dir.join(format!("{}_seeder.rs", to_snake_case(name)));
    fs::write(&seeder_file, seeder_content)?;

    // Update mod.rs
    let mut mod_content = fs::read_to_string(seeder_dir.join("mod.rs"))?;
    let module_declaration = format!("pub mod {}_seeder;\n", to_snake_case(name));
    if !mod_content.contains(&module_declaration) {
        mod_content.push_str(&module_declaration);
        fs::write(seeder_dir.join("mod.rs"), mod_content)?;
    }

    println!("✅ Successfully created seeder '{}'", name);
    println!("   Location: {}", seeder_file.display());
    
    if factory {
        println!("   🏭 Factory integration enabled");
    }
    if let Some(table_name) = table {
        println!("   📊 Targeting table: {}", table_name);
    }
    
    println!("\n📖 Usage:");
    println!("   elifrs db seed              - Run all seeders");
    println!("   elifrs db seed --env test   - Run seeders for test environment");
    println!("   elifrs db fresh --seed      - Fresh database with seeds");

    Ok(())
}

// Helper functions

fn is_valid_module_name(name: &str) -> bool {
    name.chars().next().unwrap_or(' ').is_uppercase() && name.ends_with("Module")
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}

fn generate_module_content(
    name: &str,
    providers: Vec<&str>,
    controllers: Vec<&str>,
    services: Vec<&str>,
) -> Result<String, ElifError> {
    let controllers_list = controllers
        .iter()
        .map(|c| format!("{}Controller", c.trim_end_matches("Controller")))
        .collect::<Vec<_>>()
        .join(", ");

    let providers_list = providers
        .iter()
        .chain(services.iter())
        .map(|p| format!("{}Service", p.trim_end_matches("Service")))
        .collect::<Vec<_>>()
        .join(", ");

    let content = format!(
        r#"use elif_http_derive::module;

#[module(
    controllers = [{}],
    providers = [{}],
    imports = [],
    exports = [{}]
)]
pub struct {};

// Module implementation
impl {} {{
    pub fn new() -> Self {{
        Self
    }}
}}

impl Default for {} {{
    fn default() -> Self {{
        Self::new()
    }}
}}
"#,
        controllers_list, providers_list, providers_list, name, name, name
    );

    Ok(content)
}

async fn generate_controller_file(name: &str, _module_name: &str) -> Result<(), ElifError> {
    let controller_dir = Path::new("src/controllers");
    if !controller_dir.exists() {
        fs::create_dir_all(controller_dir)?;
        fs::write(
            controller_dir.join("mod.rs"),
            "// Controller declarations\n",
        )?;
    }

    let controller_content = format!(
        r#"use elif_http::{{controller, get, post, put, delete, ElifRequest, ElifResponse, HttpResult}};

#[controller("/api/{}")]
pub struct {name};

impl {name} {{
    #[get("")]
    pub async fn index(&self, _req: ElifRequest) -> HttpResult<ElifResponse> {{
        // List all resources
        Ok(ElifResponse::json(&serde_json::json!({{
            "data": [],
            "message": "List endpoint for {name}"
        }}))?)
    }}
    
    #[get("/{{id}}")]
    pub async fn show(&self, req: ElifRequest) -> HttpResult<ElifResponse> {{
        let id = req.path_param("id")?;
        // Show specific resource
        Ok(ElifResponse::json(&serde_json::json!({{
            "data": {{"id": id}},
            "message": "Show endpoint for {name}"
        }}))?)
    }}
    
    #[post("")]
    pub async fn create(&self, req: ElifRequest) -> HttpResult<ElifResponse> {{
        // Create new resource
        Ok(ElifResponse::created().json(&serde_json::json!({{
            "data": {{"id": 1}},
            "message": "Created successfully"
        }}))?)
    }}
    
    #[put("/{{id}}")]
    pub async fn update(&self, req: ElifRequest) -> HttpResult<ElifResponse> {{
        let id = req.path_param("id")?;
        // Update resource
        Ok(ElifResponse::json(&serde_json::json!({{
            "data": {{"id": id}},
            "message": "Updated successfully"
        }}))?)
    }}
    
    #[delete("/{{id}}")]
    pub async fn destroy(&self, req: ElifRequest) -> HttpResult<ElifResponse> {{
        let id = req.path_param("id")?;
        // Delete resource
        Ok(ElifResponse::ok().json(&serde_json::json!({{
            "message": "Deleted successfully"
        }}))?)
    }}
}}
"#,
        to_snake_case(name.trim_end_matches("Controller")),
        name = name
    );

    let controller_file = controller_dir.join(format!("{}.rs", to_snake_case(name)));
    fs::write(controller_file, controller_content)?;

    // Update controllers/mod.rs
    let mut mod_content = fs::read_to_string(controller_dir.join("mod.rs"))?;
    let module_declaration = format!("pub mod {};\n", to_snake_case(name));
    if !mod_content.contains(&module_declaration) {
        mod_content.push_str(&module_declaration);
        fs::write(controller_dir.join("mod.rs"), mod_content)?;
    }

    Ok(())
}

async fn generate_service_file(name: &str, _module_name: &str) -> Result<(), ElifError> {
    let services_dir = Path::new("src/services");
    if !services_dir.exists() {
        fs::create_dir_all(services_dir)?;
        fs::write(services_dir.join("mod.rs"), "// Service declarations\n")?;
    }

    let service_content = format!(
        r#"use std::sync::Arc;
use async_trait::async_trait;

#[async_trait]
pub trait {name}Trait: Send + Sync {{
    async fn example_method(&self) -> Result<String, Box<dyn std::error::Error>>;
}}

pub struct {name} {{
    // Add service dependencies here
}}

impl {name} {{
    pub fn new() -> Self {{
        Self {{
            // Initialize dependencies
        }}
    }}
}}

#[async_trait]
impl {name}Trait for {name} {{
    async fn example_method(&self) -> Result<String, Box<dyn std::error::Error>> {{
        // Implement service logic
        Ok("Hello from {}".to_string())
    }}
}}

impl Default for {name} {{
    fn default() -> Self {{
        Self::new()
    }}
}}
"#,
        name.trim_end_matches("Service")
    );

    let service_file = services_dir.join(format!("{}.rs", to_snake_case(name)));
    fs::write(service_file, service_content)?;

    // Update services/mod.rs
    let mut mod_content = fs::read_to_string(services_dir.join("mod.rs"))?;
    let module_declaration = format!("pub mod {};\n", to_snake_case(name));
    if !mod_content.contains(&module_declaration) {
        mod_content.push_str(&module_declaration);
        fs::write(services_dir.join("mod.rs"), mod_content)?;
    }

    Ok(())
}

async fn generate_middleware_file(name: &str, debug: bool) -> Result<(), ElifError> {
    let middleware_dir = Path::new("src/middleware");
    if !middleware_dir.exists() {
        fs::create_dir_all(middleware_dir)?;
        fs::write(
            middleware_dir.join("mod.rs"),
            "// Middleware declarations\n",
        )?;
    }

    let debug_imports = if debug {
        "use tracing::{info, debug, warn};\nuse std::time::Instant;\n"
    } else {
        ""
    };

    let debug_logic = if debug {
        r#"        let start = Instant::now();
        debug!("Middleware {} started", stringify!({}));
        
        let result = next(req).await;
        
        let duration = start.elapsed();
        info!("Middleware {} completed in {:?}", stringify!({}), duration);
        
        result"#
    } else {
        "next(req).await"
    };

    let middleware_content = format!(
        r#"use elif_http::{{ElifRequest, ElifResponse, HttpResult}};
use std::future::Future;
use std::pin::Pin;
{}

pub struct {};

impl {} {{
    pub fn new() -> Self {{
        Self
    }}
    
    pub async fn handle<F, Fut>(
        &self,
        req: ElifRequest,
        next: F,
    ) -> HttpResult<ElifResponse>
    where
        F: FnOnce(ElifRequest) -> Fut,
        Fut: Future<Output = HttpResult<ElifResponse>>,
    {{
        // Middleware logic here
        {}
    }}
}}

impl Default for {} {{
    fn default() -> Self {{
        Self::new()
    }}
}}
"#,
        debug_imports, name, name, debug_logic, name
    );

    let middleware_file = middleware_dir.join(format!("{}.rs", to_snake_case(name)));
    fs::write(middleware_file, middleware_content)?;

    // Update middleware/mod.rs
    let mut mod_content = fs::read_to_string(middleware_dir.join("mod.rs"))?;
    let module_declaration = format!("pub mod {};\n", to_snake_case(name));
    if !mod_content.contains(&module_declaration) {
        mod_content.push_str(&module_declaration);
        fs::write(middleware_dir.join("mod.rs"), mod_content)?;
    }

    Ok(())
}

fn update_modules_mod(mod_path: &Path, module_name: &str) -> Result<(), ElifError> {
    let mut content = fs::read_to_string(mod_path)?;
    let module_declaration = format!("pub mod {};\n", to_snake_case(module_name));

    if !content.contains(&module_declaration) {
        content.push_str(&module_declaration);
        fs::write(mod_path, content)?;
    }

    Ok(())
}

fn update_module_with_service(module_file: &Path, service_name: &str) -> Result<(), ElifError> {
    // This is a simplified implementation - in a real implementation,
    // you'd parse the Rust AST and properly update the module macro
    let _content = fs::read_to_string(module_file)?;

    // For now, just print instructions
    println!("📝 Manual step required:");
    println!(
        "   Add '{}' to the providers list in {}",
        service_name,
        module_file.display()
    );

    Ok(())
}

fn update_module_with_controller(
    module_file: &Path,
    controller_name: &str,
) -> Result<(), ElifError> {
    // This is a simplified implementation - in a real implementation,
    // you'd parse the Rust AST and properly update the module macro
    let _content = fs::read_to_string(module_file)?;

    // For now, just print instructions
    println!("📝 Manual step required:");
    println!(
        "   Add '{}' to the controllers list in {}",
        controller_name,
        module_file.display()
    );

    Ok(())
}

fn update_module_with_middleware(
    _module_file: &Path,
    middleware_name: &str,
) -> Result<(), ElifError> {
    // This is a simplified implementation
    println!("📝 Manual step required:");
    println!(
        "   Add middleware to your controller methods: #[middleware(\"{}\")]",
        to_snake_case(middleware_name)
    );

    Ok(())
}
