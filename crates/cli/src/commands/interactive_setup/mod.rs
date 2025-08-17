//! Interactive setup module for elif.rs CLI
//! 
//! This module provides interactive project setup and configuration capabilities,
//! split into logical components:
//! 
//! - `interactive_wizard`: Main wizard flow and orchestration
//! - `interactive_config`: Configuration collection and file generation

pub mod interactive_wizard;
pub mod interactive_config;

// Re-export main types and functions for external use
pub use interactive_wizard::{InteractiveSetupArgs, InteractiveSetupCommand};
pub use interactive_config::{ProjectConfig, InteractiveConfigHandler};

use elif_core::ElifError;
use crate::command_system::CommandHandler;

/// Create and run interactive setup command
pub async fn run(args: InteractiveSetupArgs) -> Result<(), ElifError> {
    let command = InteractiveSetupCommand::new(args);
    command.handle().await.map_err(|e| {
        ElifError::Codegen(format!("Interactive setup command failed: {}", e))
    })
}