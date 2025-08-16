use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use clap::Args;
use elif_core::ElifError;

/// Trait for command handlers
#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn handle(&self) -> Result<(), CommandError>;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn help(&self) -> Option<&'static str> {
        None
    }
}

/// Command execution errors
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Command failed: {0}")]
    ExecutionError(String),
    
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    
    #[error("Command not found: {0}")]
    NotFound(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Core error: {0}")]
    Core(#[from] ElifError),
}

/// Command registry for managing custom and built-in commands
pub struct CommandRegistry {
    commands: Arc<Mutex<HashMap<String, Arc<dyn CommandHandler>>>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Register a new command handler
    pub fn register<T>(&self, handler: T) -> Result<(), CommandError>
    where
        T: CommandHandler + 'static,
    {
        let mut commands = self.commands.lock().unwrap();
        let name = handler.name().to_string();
        commands.insert(name, Arc::new(handler));
        Ok(())
    }
    
    /// Execute a command by name
    pub async fn execute(&self, name: &str) -> Result<(), CommandError> {
        let commands = self.commands.lock().unwrap();
        if let Some(handler) = commands.get(name) {
            let handler = Arc::clone(handler);
            drop(commands); // Release lock before async execution
            handler.handle().await
        } else {
            Err(CommandError::NotFound(name.to_string()))
        }
    }
    
    /// List all registered commands
    pub fn list_commands(&self) -> Vec<(String, String)> {
        let commands = self.commands.lock().unwrap();
        commands
            .iter()
            .map(|(name, handler)| (name.clone(), handler.description().to_string()))
            .collect()
    }
    
    /// Get help for a specific command
    pub fn get_help(&self, name: &str) -> Option<String> {
        let commands = self.commands.lock().unwrap();
        commands.get(name).and_then(|handler| {
            handler.help().map(|h| h.to_string())
        })
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global command registry instance
static COMMAND_REGISTRY: std::sync::OnceLock<CommandRegistry> = std::sync::OnceLock::new();

/// Get or initialize the global command registry
pub fn get_command_registry() -> &'static CommandRegistry {
    COMMAND_REGISTRY.get_or_init(CommandRegistry::new)
}

/// Trait for defining command-line argument structures
pub trait CommandDefinition: Args + Send + Sync + Clone {
    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    const HELP: Option<&'static str> = None;
}

/// Macro to derive CommandDefinition automatically
#[macro_export]
macro_rules! impl_command {
    ($struct_name:ident, $name:expr, $description:expr) => {
        impl CommandDefinition for $struct_name {
            const NAME: &'static str = $name;
            const DESCRIPTION: &'static str = $description;
        }
    };
    ($struct_name:ident, $name:expr, $description:expr, $help:expr) => {
        impl CommandDefinition for $struct_name {
            const NAME: &'static str = $name;
            const DESCRIPTION: &'static str = $description;
            const HELP: Option<&'static str> = Some($help);
        }
    };
}

pub use impl_command;

/// Wrapper to convert CommandDefinition + async handler into CommandHandler
pub struct DefinitionCommand<T> 
where 
    T: CommandDefinition + 'static,
{
    pub args: T,
    pub handler: Box<dyn Fn(T) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), CommandError>> + Send>> + Send + Sync>,
}

#[async_trait]
impl<T> CommandHandler for DefinitionCommand<T>
where
    T: CommandDefinition + 'static,
{
    async fn handle(&self) -> Result<(), CommandError> {
        // This is a placeholder - in practice, we'd need to get the args from command line
        // For now, we'll use the stored args
        let future = (self.handler)(self.args.clone());
        future.await
    }
    
    fn name(&self) -> &'static str {
        T::NAME
    }
    
    fn description(&self) -> &'static str {
        T::DESCRIPTION
    }
    
    fn help(&self) -> Option<&'static str> {
        T::HELP
    }
}