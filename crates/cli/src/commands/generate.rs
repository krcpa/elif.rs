use elif_core::ElifError;
use elif_codegen::CodeGenerator;
use std::fs;
use std::path::Path;

pub async fn run() -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;
    
    let generator = CodeGenerator::new(project_root);
    generator.generate_all()?;
    
    println!("✓ Code generation completed");
    Ok(())
}

pub async fn middleware(name: &str, debug: bool, conditional: bool, tests: bool) -> Result<(), ElifError> {
    let project_root = std::env::current_dir()
        .map_err(|e| ElifError::Io(e))?;

    // Convert name to proper formats
    let struct_name = to_pascal_case(name);
    let snake_name = to_snake_case(name);
    let file_name = format!("{}_middleware.rs", snake_name);
    
    // Create src/middleware directory if it doesn't exist
    let middleware_dir = project_root.join("src").join("middleware");
    fs::create_dir_all(&middleware_dir)
        .map_err(|e| ElifError::Io(e))?;
    
    // Generate middleware file
    let middleware_path = middleware_dir.join(&file_name);
    
    // Check if file already exists
    if middleware_path.exists() {
        return Err(ElifError::Validation {
            message: format!("Middleware file {} already exists", file_name)
        });
    }
    
    // Read template
    let template_path = get_template_path("middleware.stub")?;
    let template_content = fs::read_to_string(&template_path)
        .map_err(|e| ElifError::Io(e))?;
    
    // Generate content
    let content = generate_middleware_content(
        &template_content,
        &struct_name,
        &snake_name,
        debug,
        conditional,
        tests
    );
    
    // Write middleware file
    fs::write(&middleware_path, content)
        .map_err(|e| ElifError::Io(e))?;
    
    // Update mod.rs if it exists
    update_middleware_mod(&middleware_dir, &snake_name)?;
    
    println!("✓ Generated middleware: {}", middleware_path.display());
    println!("  Add to your middleware pipeline with:");
    println!("  app.use_middleware({}::new());", struct_name);
    
    if conditional {
        println!("  Use conditional features:");
        println!("  app.use_middleware(ConditionalMiddleware::new({}::new())", struct_name);
        println!("      .skip_paths(vec![\"/public/*\"])");
        println!("      .only_methods(vec![ElifMethod::POST]));");
    }
    
    if debug {
        println!("  Use with debugging:");
        println!("  app.use_middleware(InstrumentedMiddleware::new({}::new(), \"{}\".to_string()));", struct_name, struct_name);
    }
    
    Ok(())
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }
    result
}

fn get_template_path(template_name: &str) -> Result<std::path::PathBuf, ElifError> {
    // Try to find template in CLI crate first
    let current_exe = std::env::current_exe()
        .map_err(|e| ElifError::Io(e))?;
    
    // Look for template relative to the CLI installation
    if let Some(exe_dir) = current_exe.parent() {
        let template_path = exe_dir.join("templates").join(template_name);
        if template_path.exists() {
            return Ok(template_path);
        }
    }
    
    // Look in the crates/cli/templates directory (for development)
    let dev_template_path = Path::new("crates/cli/templates").join(template_name);
    if dev_template_path.exists() {
        return Ok(dev_template_path);
    }
    
    // Try current directory templates folder
    let local_template_path = Path::new("templates").join(template_name);
    if local_template_path.exists() {
        return Ok(local_template_path);
    }
    
    Err(ElifError::Validation {
        message: format!("Could not find template: {}", template_name)
    })
}

fn generate_middleware_content(
    template: &str,
    struct_name: &str,
    snake_name: &str,
    debug: bool,
    conditional: bool,
    _tests: bool, // tests are always included in template
) -> String {
    let mut content = template.to_string();
    
    // Replace placeholders
    content = content.replace("{{STRUCT_NAME}}", struct_name);
    content = content.replace("{{SNAKE_NAME}}", snake_name);
    content = content.replace("{{DESCRIPTION}}", &format!("{} middleware", struct_name));
    
    // Add conditional implementation if requested
    let conditional_impl = if conditional {
        format!(r#"
/// Conditional wrapper for {}
pub type Conditional{} = elif_http::middleware::v2::ConditionalMiddleware<{}>;

impl {} {{
    /// Create a conditional version of this middleware
    pub fn conditional(self) -> Conditional{} {{
        elif_http::middleware::v2::ConditionalMiddleware::new(self)
    }}
}}
"#, struct_name, struct_name, struct_name, struct_name, struct_name)
    } else {
        String::new()
    };
    content = content.replace("{{CONDITIONAL_IMPL}}", &conditional_impl);
    
    // Add debug implementation if requested
    let debug_impl = if debug {
        format!(r#"
/// Debug instrumentation for {}
pub type Instrumented{} = elif_http::middleware::v2::introspection::InstrumentedMiddleware<{}>;

impl {} {{
    /// Create an instrumented version of this middleware for debugging
    pub fn instrumented(self, name: String) -> Instrumented{} {{
        elif_http::middleware::v2::introspection::instrument(self, name)
    }}
}}
"#, struct_name, struct_name, struct_name, struct_name, struct_name)
    } else {
        String::new()
    };
    content = content.replace("{{DEBUG_IMPL}}", &debug_impl);
    
    content
}

fn update_middleware_mod(middleware_dir: &Path, module_name: &str) -> Result<(), ElifError> {
    let mod_path = middleware_dir.join("mod.rs");
    
    if mod_path.exists() {
        let content = fs::read_to_string(&mod_path)
            .map_err(|e| ElifError::Io(e))?;
        
        // Check if module is already declared
        let module_line = format!("pub mod {};", module_name);
        if !content.contains(&module_line) {
            let new_content = if content.trim().is_empty() {
                format!("{}\n", module_line)
            } else {
                format!("{}\n{}\n", content.trim(), module_line)
            };
            
            fs::write(&mod_path, new_content)
                .map_err(|e| ElifError::Io(e))?;
            
            println!("  Updated src/middleware/mod.rs");
        }
    } else {
        // Create new mod.rs file
        let mod_content = format!("pub mod {};\n", module_name);
        fs::write(&mod_path, mod_content)
            .map_err(|e| ElifError::Io(e))?;
        
        println!("  Created src/middleware/mod.rs");
    }
    
    Ok(())
}