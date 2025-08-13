mod commands;

use clap::{Parser, Subcommand};
use elif_core::ElifError;
use commands::*;

#[derive(Parser)]
#[command(name = "elif")]
#[command(about = "LLM-friendly Rust web framework CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new elif application
    New {
        /// Application name
        name: String,
        
        /// Target directory (optional)
        #[arg(long)]
        path: Option<String>,
    },
    
    /// Generate code from resource specifications
    Generate,
    
    /// Route management
    Route {
        #[command(subcommand)]
        route_command: RouteCommands,
    },
    
    /// Model management  
    Model {
        #[command(subcommand)]
        model_command: ModelCommands,
    },
    
    /// Create a new resource specification
    Resource {
        #[command(subcommand)]
        resource_command: ResourceCommands,
    },
    
    /// Check project for errors and lint
    Check,
    
    /// Run tests
    Test {
        /// Focus on specific resource
        #[arg(long)]
        focus: Option<String>,
    },
    
    /// Generate project map
    Map {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    
    /// Export OpenAPI specification
    Openapi {
        #[command(subcommand)]
        openapi_command: OpenApiCommands,
    },
}

#[derive(Subcommand)]
enum RouteCommands {
    /// Add a new route
    Add {
        /// HTTP method (GET, POST, PUT, DELETE)
        method: String,
        
        /// Route path (e.g., /users/:id)
        path: String,
        
        /// Controller function name
        controller: String,
    },
    
    /// List all routes
    List,
}

#[derive(Subcommand)]
enum ModelCommands {
    /// Add a new model
    Add {
        /// Model name
        name: String,
        
        /// Fields in format name:type,name:type
        fields: String,
    },
}

#[derive(Subcommand)]
enum ResourceCommands {
    /// Create new resource
    New {
        /// Resource name
        name: String,
        
        /// Route path
        #[arg(long)]
        route: String,
        
        /// Fields in format name:type,name:type
        #[arg(long)]
        fields: String,
    },
}

#[derive(Subcommand)]
enum OpenApiCommands {
    /// Export OpenAPI spec
    Export,
}

#[tokio::main]
async fn main() -> Result<(), ElifError> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::New { name, path } => {
            new::create_app(&name, path.as_deref()).await?;
        }
        Commands::Generate => {
            generate::run().await?;
        }
        Commands::Route { route_command } => {
            match route_command {
                RouteCommands::Add { method, path, controller } => {
                    route::add_route(&method, &path, &controller).await?;
                }
                RouteCommands::List => {
                    route::list_routes().await?;
                }
            }
        }
        Commands::Model { model_command } => {
            match model_command {
                ModelCommands::Add { name, fields } => {
                    model::add_model(&name, &fields).await?;
                }
            }
        }
        Commands::Resource { resource_command } => {
            match resource_command {
                ResourceCommands::New { name, route, fields } => {
                    resource::new_resource(&name, &route, &fields)?;
                }
            }
        }
        Commands::Check => {
            check::run().await?;
        }
        Commands::Test { focus } => {
            test::run(focus).await?;
        }
        Commands::Map { json } => {
            map::run(json).await?;
        }
        Commands::Openapi { openapi_command } => {
            match openapi_command {
                OpenApiCommands::Export => {
                    openapi::export().await?;
                }
            }
        }
    }
    
    Ok(())
}