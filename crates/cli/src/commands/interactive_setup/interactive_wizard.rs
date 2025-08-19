use clap::Args;
use elif_core::ElifError;
use crate::interactive::Prompt;
use super::interactive_config::{InteractiveConfig, ProjectSettings};

/// Interactive setup wizard arguments
#[derive(Args, Debug, Clone)]
pub struct InteractiveSetupArgs {
    /// Skip interactive mode and use defaults
    #[arg(long)]
    pub non_interactive: bool,
    
    /// Show verbose output during setup
    #[arg(long, short)]
    pub verbose: bool,
}

/// Run the interactive project setup wizard
pub async fn run_wizard(args: InteractiveSetupArgs) -> Result<(), ElifError> {
    if args.verbose {
        println!("🔧 Interactive Setup Configuration:");
        println!("  Non-interactive: {}", args.non_interactive);
    }
    
    println!("🧙 Welcome to the elif.rs Interactive Setup Wizard!");
    println!("====================================================");
    
    // Validate project structure
    let is_valid_project = InteractiveConfig::validate_project()?;
    
    if !is_valid_project {
        println!("⚠️  This doesn't appear to be an elif project directory.");
        if !args.non_interactive {
            let should_continue = Prompt::confirm("Continue anyway?", false)
                .map_err(|e| ElifError::Codegen { message: format!("Input error: {}", e) })?;
            
            if !should_continue {
                println!("👋 Setup cancelled. Run 'elifrs new <project_name>' to create a new project.");
                return Ok(());
            }
        }
    }
    
    let settings = if args.non_interactive {
        println!("🤖 Using default settings (non-interactive mode)");
        InteractiveConfig::get_recommended_settings()
    } else {
        gather_user_preferences()?
    };
    
    apply_settings(settings, args.verbose).await?;
    
    println!("\n✅ Interactive setup completed!");
    println!("🚀 Your elif project is ready to go!");
    
    Ok(())
}

/// Gather user preferences through interactive prompts
fn gather_user_preferences() -> Result<ProjectSettings, ElifError> {
    println!("\n📋 Let's configure your project:");
    
    let use_hot_reload = Prompt::confirm("Enable hot reload for development?", true)
        .map_err(|e| ElifError::Codegen { message: format!("Input error: {}", e) })?;
    
    let default_port = Prompt::number("Default server port", Some(3000u16))
        .map_err(|e| ElifError::Codegen { message: format!("Input error: {}", e) })?;
    
    let include_auth = Prompt::confirm("Include authentication scaffolding?", false)
        .map_err(|e| ElifError::Codegen { message: format!("Input error: {}", e) })?;
    
    let include_database = Prompt::confirm("Include database configuration?", true)
        .map_err(|e| ElifError::Codegen { message: format!("Input error: {}", e) })?;
    
    Ok(ProjectSettings {
        use_hot_reload,
        default_port,
        include_auth,
        include_database,
    })
}

/// Apply the configured settings to the project
async fn apply_settings(settings: ProjectSettings, verbose: bool) -> Result<(), ElifError> {
    if verbose {
        println!("\n📝 Applying settings:");
        println!("  Hot reload: {}", settings.use_hot_reload);
        println!("  Default port: {}", settings.default_port);
        println!("  Include auth: {}", settings.include_auth);
        println!("  Include database: {}", settings.include_database);
    }
    
    // Placeholder implementation
    println!("\n⚠️  Setting application is not yet fully implemented");
    println!("📋 TODO: Generate configuration files and project structure");
    
    Ok(())
}