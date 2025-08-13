use elif_core::ElifError;
use std::fs;
use std::path::Path;

pub async fn add_route(method: &str, path: &str, controller: &str) -> Result<(), ElifError> {
    // Validate method
    let method = method.to_uppercase();
    if !["GET", "POST", "PUT", "DELETE", "PATCH"].contains(&method.as_str()) {
        return Err(ElifError::Validation(
            format!("Invalid HTTP method: {}. Use GET, POST, PUT, DELETE, or PATCH", method)
        ));
    }
    
    println!("🛣️  Adding route: {} {} -> {}", method, path, controller);
    
    // Create controller if it doesn't exist
    create_controller_if_missing(controller).await?;
    
    // Add route to routes/mod.rs
    add_route_to_router(&method, path, controller).await?;
    
    println!("✅ Route added successfully!");
    println!("📝 Controller: src/controllers/{}.rs", controller);
    println!("🔗 Route: {} {} -> {}", method, path, controller);
    
    Ok(())
}

pub async fn list_routes() -> Result<(), ElifError> {
    let routes_file = "src/routes/mod.rs";
    
    if !Path::new(routes_file).exists() {
        println!("No routes found. Create an elif app first with: elif new <app_name>");
        return Ok(());
    }
    
    let content = fs::read_to_string(routes_file)?;
    
    println!("📋 Current Routes:");
    
    // Parse routes from the file (simple parsing for now)
    for line in content.lines() {
        if line.trim().starts_with(".route(") {
            println!("  {}", line.trim());
        }
    }
    
    Ok(())
}

async fn create_controller_if_missing(controller_name: &str) -> Result<(), ElifError> {
    let controller_path = format!("src/controllers/{}.rs", controller_name);
    
    if Path::new(&controller_path).exists() {
        return Ok(()); // Controller already exists
    }
    
    let controller_content = format!(r#"use axum::{{
    response::Json,
    extract::{{Path, Query}},
    http::StatusCode,
}};
use serde_json::Value;
use uuid::Uuid;

// <<<ELIF:BEGIN agent-editable:{}>>>
pub async fn {}() -> Result<Json<Value>, StatusCode> {{
    // TODO: Implement your logic here
    Ok(Json(serde_json::json!({{
        "message": "Hello from {}!",
        "status": "success"
    }})))
}}
// <<<ELIF:END agent-editable:{}>>>
"#, controller_name, controller_name, controller_name, controller_name);
    
    fs::write(&controller_path, controller_content)?;
    
    // Update controllers/mod.rs
    update_controllers_mod(controller_name)?;
    
    println!("📝 Created controller: {}", controller_path);
    
    Ok(())
}

fn update_controllers_mod(controller_name: &str) -> Result<(), ElifError> {
    let mod_path = "src/controllers/mod.rs";
    let mut content = fs::read_to_string(mod_path)?;
    
    // Add module declaration if not present
    let mod_declaration = format!("pub mod {};", controller_name);
    if !content.contains(&mod_declaration) {
        // Insert after the existing comments but before any existing modules
        if content.starts_with("//") {
            // Find first non-comment line
            let lines: Vec<&str> = content.lines().collect();
            let mut insert_index = 0;
            for (i, line) in lines.iter().enumerate() {
                if !line.starts_with("//") && !line.trim().is_empty() {
                    insert_index = i;
                    break;
                }
                if i == lines.len() - 1 {
                    insert_index = lines.len();
                }
            }
            
            let mut new_lines = lines[0..insert_index].to_vec();
            new_lines.push(&mod_declaration);
            new_lines.extend_from_slice(&lines[insert_index..]);
            content = new_lines.join("\n");
        } else {
            content = format!("{}\n{}", mod_declaration, content);
        }
        
        fs::write(mod_path, content)?;
    }
    
    Ok(())
}

async fn add_route_to_router(method: &str, path: &str, controller: &str) -> Result<(), ElifError> {
    let routes_path = "src/routes/mod.rs";
    let mut content = fs::read_to_string(routes_path)?;
    
    let axum_method = match method {
        "GET" => "get",
        "POST" => "post", 
        "PUT" => "put",
        "DELETE" => "delete",
        "PATCH" => "patch",
        _ => return Err(ElifError::Validation(format!("Unsupported method: {}", method))),
    };
    
    let route_line = format!(r#"        .route("{}", {}(crate::controllers::{}::{}))"#, 
                           path, axum_method, controller, controller);
    
    // Find the Router::new() section and add the route
    if content.contains("Router::new()") {
        // Add after Router::new()
        let new_content = content.replace(
            "Router::new()",
            &format!("Router::new()\n{}", route_line)
        );
        
        fs::write(routes_path, new_content)?;
    } else {
        return Err(ElifError::Validation(
            "Could not find Router::new() in src/routes/mod.rs".to_string()
        ));
    }
    
    Ok(())
}