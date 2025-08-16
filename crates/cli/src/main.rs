mod commands;
mod generators;

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
    
    /// Advanced code generation (make: commands)
    Make {
        #[command(subcommand)]
        make_command: MakeCommands,
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
    
    /// Database migration management
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },
    
    /// Authentication management
    Auth {
        #[command(subcommand)]
        auth_command: AuthCommands,
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

#[derive(Subcommand)]
enum MigrateCommands {
    /// Create a new migration
    Create {
        /// Migration name
        name: String,
    },
    
    /// Run pending migrations
    Run,
    
    /// Rollback the last migration
    Rollback,
    
    /// Show migration status
    Status,
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Set up authentication configuration
    Setup {
        /// Authentication provider (jwt, session)
        #[arg(long, value_enum, default_value = "jwt")]
        provider: AuthProvider,
        
        /// Include MFA support
        #[arg(long)]
        mfa: bool,
        
        /// Include RBAC support
        #[arg(long)]
        rbac: bool,
    },
    
    /// Generate JWT secret key
    GenerateKey {
        /// Key length in bytes
        #[arg(long, default_value = "64")]
        length: usize,
    },
    
    /// Generate authentication scaffold
    Scaffold {
        /// Include registration endpoints
        #[arg(long)]
        registration: bool,
        
        /// Include password reset
        #[arg(long)]
        reset_password: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum AuthProvider {
    Jwt,
    Session,
    Both,
}

#[derive(Subcommand)]
enum MakeCommands {
    /// Generate a complete resource with model, controller, migration, tests, and policies
    Resource {
        /// Resource name (e.g., Post, User, Product)
        name: String,
        
        /// Fields in format name:type,name:type (e.g., title:string,content:text,user_id:integer)
        #[arg(long)]
        fields: String,
        
        /// Relationships in format name:type (e.g., user:belongs_to,comments:has_many)
        #[arg(long)]
        relationships: Option<String>,
        
        /// Generate API endpoints
        #[arg(long)]
        api: bool,
        
        /// Generate comprehensive tests
        #[arg(long)]
        tests: bool,
        
        /// Generate authorization policy
        #[arg(long)]
        policy: bool,
        
        /// Generate request validation classes
        #[arg(long)]
        requests: bool,
        
        /// Generate API resource classes  
        #[arg(long)]
        resources: bool,
        
        /// Include authentication middleware
        #[arg(long)]
        auth: bool,
        
        /// Enable soft deletes
        #[arg(long)]
        soft_delete: bool,
    },
    
    /// Generate authentication system
    Auth {
        /// Use JWT authentication
        #[arg(long)]
        jwt: bool,
        
        /// Use session authentication
        #[arg(long)]
        session: bool,
        
        /// Include MFA support
        #[arg(long)]
        mfa: bool,
        
        /// Include password reset functionality
        #[arg(long)]
        password_reset: bool,
        
        /// Include user registration
        #[arg(long)]
        registration: bool,
        
        /// Include RBAC support
        #[arg(long)]
        rbac: bool,
    },
    
    /// Generate API with OpenAPI documentation
    Api {
        /// API version (e.g., v1, v2)
        version: String,
        
        /// Resources to include (comma-separated)
        #[arg(long)]
        resources: String,
        
        /// Generate OpenAPI specification
        #[arg(long)]
        openapi: bool,
        
        /// Enable API versioning
        #[arg(long)]
        versioning: bool,
    },
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
        Commands::Migrate { migrate_command } => {
            match migrate_command {
                MigrateCommands::Create { name } => {
                    migrate::create(&name).await?;
                }
                MigrateCommands::Run => {
                    migrate::run().await?;
                }
                MigrateCommands::Rollback => {
                    migrate::rollback().await?;
                }
                MigrateCommands::Status => {
                    migrate::status().await?;
                }
            }
        }
        Commands::Auth { auth_command } => {
            match auth_command {
                AuthCommands::Setup { provider, mfa, rbac } => {
                    auth::setup(provider, mfa, rbac).await?;
                }
                AuthCommands::GenerateKey { length } => {
                    auth::generate_key(length).await?;
                }
                AuthCommands::Scaffold { registration, reset_password } => {
                    auth::scaffold(registration, reset_password).await?;
                }
            }
        }
        Commands::Make { make_command } => {
            match make_command {
                MakeCommands::Resource { 
                    name, 
                    fields, 
                    relationships,
                    api,
                    tests,
                    policy,
                    requests,
                    resources,
                    auth,
                    soft_delete,
                } => {
                    make::resource(
                        &name,
                        &fields,
                        relationships.as_deref(),
                        api,
                        tests,
                        policy,
                        requests,
                        resources,
                        auth,
                        soft_delete,
                    ).await?;
                }
                MakeCommands::Auth {
                    jwt,
                    session,
                    mfa,
                    password_reset,
                    registration,
                    rbac,
                } => {
                    make::auth(jwt, session, mfa, password_reset, registration, rbac).await?;
                }
                MakeCommands::Api {
                    version,
                    resources,
                    openapi,
                    versioning,
                } => {
                    make::api(&version, &resources, openapi, versioning).await?;
                }
            }
        }
    }
    
    Ok(())
}