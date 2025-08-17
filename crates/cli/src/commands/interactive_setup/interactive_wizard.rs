use clap::Args;
use crate::command_system::{CommandHandler, CommandError, CommandDefinition, impl_command};
use crate::interactive::{Prompt, Format};
use async_trait::async_trait;
use super::interactive_config::{ProjectConfig, InteractiveConfigHandler};

/// Interactive setup command arguments
#[derive(Args, Debug, Clone)]
pub struct InteractiveSetupArgs {
    /// Skip interactive mode and use defaults
    #[arg(long)]
    pub non_interactive: bool,
    
    /// Show verbose output during setup
    #[arg(long, short)]
    pub verbose: bool,
}

impl_command!(
    InteractiveSetupArgs,
    "interactive-setup",
    "Interactive project setup and configuration wizard",
    "Run an interactive setup wizard to configure your elif project.\n\n\
     Features:\n\
     - Database configuration\n\
     - Authentication setup\n\
     - Environment configuration\n\
     - Development tools setup\n\n\
     Examples:\n\
       elifrs interactive-setup\n\
       elifrs interactive-setup --verbose\n\
       elifrs interactive-setup --non-interactive"
);

/// Interactive setup command handler
pub struct InteractiveSetupCommand {
    pub args: InteractiveSetupArgs,
    config_handler: InteractiveConfigHandler,
}

#[async_trait]
impl CommandHandler for InteractiveSetupCommand {
    async fn handle(&self) -> Result<(), CommandError> {
        if self.args.non_interactive {
            self.non_interactive_setup().await
        } else {
            self.interactive_setup().await
        }
    }
    
    fn name(&self) -> &'static str {
        InteractiveSetupArgs::NAME
    }
    
    fn description(&self) -> &'static str {
        InteractiveSetupArgs::DESCRIPTION
    }
    
    fn help(&self) -> Option<&'static str> {
        InteractiveSetupArgs::HELP
    }
}

impl InteractiveSetupCommand {
    pub fn new(args: InteractiveSetupArgs) -> Self {
        Self { 
            config_handler: InteractiveConfigHandler::new(args.verbose),
            args,
        }
    }
    
    async fn interactive_setup(&self) -> Result<(), CommandError> {
        Format::header("🚀 elif.rs Interactive Setup");
        
        println!("Welcome to the elif.rs project setup wizard!");
        println!("This will help you configure your project with the best settings.");
        println!();
        
        if !Prompt::confirm("Would you like to continue with the setup?", true)
            .map_err(|e| CommandError::Io(e))? 
        {
            Format::info("Setup cancelled by user");
            return Ok(());
        }
        
        let mut config = ProjectConfig::default();
        
        // Project configuration
        self.config_handler.configure_project(&mut config).await?;
        
        // Database configuration
        self.config_handler.configure_database(&mut config).await?;
        
        // Authentication configuration
        self.config_handler.configure_auth(&mut config).await?;
        
        // Server configuration
        self.config_handler.configure_server(&mut config).await?;
        
        // Logging configuration
        self.config_handler.configure_logging(&mut config).await?;
        
        // Summary
        self.config_handler.show_summary(&config).await?;
        
        // Apply configuration
        if Prompt::confirm("Apply this configuration?", true)
            .map_err(|e| CommandError::Io(e))? 
        {
            self.config_handler.apply_configuration(&config).await?;
            Format::success("Setup completed successfully!");
        } else {
            Format::info("Setup cancelled - no changes were made");
        }
        
        Ok(())
    }
    
    async fn non_interactive_setup(&self) -> Result<(), CommandError> {
        Format::info("Running non-interactive setup with default values");
        
        let config = ProjectConfig::default();
        
        if self.args.verbose {
            self.config_handler.show_summary(&config).await?;
        }
        
        self.config_handler.apply_configuration(&config).await?;
        Format::success("Non-interactive setup completed!");
        
        Ok(())
    }
}