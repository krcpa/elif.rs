mod commands;
mod generators;  // Re-enabled for make commands
mod utils;
// mod interactive; // Disabled to fix compilation - contains unused code

use clap::{Parser, Subcommand};
use elif_core::ElifError;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(name = "elif")]
#[command(about = "LLM-friendly Rust web framework CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new elif application (convention over configuration)
    New {
        /// Application name
        name: String,
    },

    /// Upgrade elifrs CLI to the latest version
    Upgrade {
        /// Force upgrade even if already on latest version
        #[arg(long)]
        force: bool,
    },

    /// Create a new elif application with module system templates
    Create {
        #[command(subcommand)]
        create_command: CreateCommands,
    },

    /// Add modules, services, controllers, and other components
    Add {
        #[command(subcommand)]
        add_command: AddCommands,
    },

    /// Inspect modules, dependencies, and project structure
    Inspect {
        #[command(subcommand)]
        inspect_command: InspectCommands,
    },

    /// Development server with hot reload
    Serve {
        /// Port to bind the server to
        #[arg(long, short, default_value = "3000")]
        port: u16,

        /// Host to bind the server to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Enable hot reload for development
        #[arg(long)]
        hot_reload: bool,

        /// Watch additional directories for changes
        #[arg(long)]
        watch: Vec<std::path::PathBuf>,

        /// Exclude patterns from watching
        #[arg(long)]
        exclude: Vec<String>,

        /// Environment to run in
        #[arg(long, short, default_value = "development")]
        env: String,
    },

    /// File watcher for auto-restart
    Watch {
        /// Directories to watch for changes
        #[arg(long)]
        paths: Vec<std::path::PathBuf>,

        /// Patterns to exclude from watching
        #[arg(long)]
        exclude: Vec<String>,

        /// Command to execute on file changes
        #[arg(long, default_value = "cargo build")]
        command: String,

        /// Debounce delay in milliseconds
        #[arg(long, default_value = "500")]
        debounce: u64,
    },

    /// Unified development mode with hot reload and intelligent features
    Dev {
        /// Watch specific directories for changes
        #[arg(long)]
        watch: Vec<std::path::PathBuf>,

        /// Enable performance profiling
        #[arg(long)]
        profile: bool,

        /// Port to bind the server to
        #[arg(long, short, default_value = "3000")]
        port: u16,

        /// Host to bind the server to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Environment to run in
        #[arg(long, short, default_value = "development")]
        env: String,
    },

    /// Comprehensive project validation and health check
    Check {
        /// Enable comprehensive checking (modules, dependencies, config)
        #[arg(long)]
        comprehensive: bool,

        /// Focus check on specific module
        #[arg(long)]
        module: Option<String>,
    },

    /// Intelligent issue diagnosis and automatic fixes
    Doctor {
        /// Automatically fix issues where possible
        #[arg(long)]
        fix_issues: bool,

        /// Show detailed diagnostic information
        #[arg(long)]
        verbose: bool,
    },

    /// Smart testing integration with module awareness
    Test {
        /// Run unit tests only
        #[arg(long)]
        unit: bool,

        /// Run integration tests only
        #[arg(long)]
        integration: bool,

        /// Enable continuous testing (watch mode)
        #[arg(long)]
        watch: bool,

        /// Enable coverage reporting
        #[arg(long)]
        coverage: bool,

        /// Focus on specific module
        #[arg(long)]
        module: Option<String>,
    },

    /// Complete database lifecycle management
    Db {
        #[command(subcommand)]
        db_command: DbCommands,
    },

    /// Migration conversion and transformation tools
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },

    /// Production build optimization and deployment preparation
    Build {
        /// Build for release (production optimization)
        #[arg(long)]
        release: bool,

        /// Build target (docker, native, wasm)
        #[arg(long, default_value = "native")]
        target: String,

        /// Enable specific optimizations
        #[arg(long)]
        optimizations: Vec<String>,
    },

    /// Framework optimization tools
    Optimize {
        /// Optimize routes (caching, precompilation)
        #[arg(long)]
        routes: bool,

        /// Optimize assets (bundling, minification)
        #[arg(long)]
        assets: bool,

        /// Optimize configuration (caching, validation)
        #[arg(long)]
        config: bool,

        /// Force overwrite existing files without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Deployment preparation and tools
    Deploy {
        #[command(subcommand)]
        deploy_command: DeployCommands,
    },

    /// Framework information and status
    Info {
        /// Show detailed framework information
        #[arg(long)]
        detailed: bool,

        /// Show module system information
        #[arg(long)]
        modules: bool,
    },

    /// Runtime status and health monitoring
    Status {
        /// Perform comprehensive health check
        #[arg(long)]
        health: bool,

        /// Check specific component status
        #[arg(long)]
        component: Option<String>,
    },

    /// Framework dependency management and updates
    Update {
        /// Check for framework updates
        #[arg(long)]
        check: bool,

        /// Perform dependency vulnerability scanning
        #[arg(long)]
        security: bool,

        /// Update dependencies automatically
        #[arg(long)]
        dependencies: bool,

        /// Show verbose update information
        #[arg(long)]
        verbose: bool,
    },

    /// API version management
    Version {
        #[command(subcommand)]
        version_command: ApiVersionCommands,
    },

    /// Advanced module system management
    Module {
        #[command(subcommand)]
        module_command: ModuleCommands,
    },

    /// Generate code components and scaffolding
    Make {
        #[command(subcommand)]
        make_command: MakeCommands,
    },
}

// ========== Original Elif.rs Command Structure ==========

#[derive(Subcommand)]
enum CreateCommands {
    /// Create a new elif application
    App {
        /// Application name
        name: String,

        /// Target directory (optional)
        #[arg(long)]
        path: Option<String>,

        /// Template type (api, web, minimal)
        #[arg(long, default_value = "web")]
        template: String,

        // Module system is always enabled
    },
}

#[derive(Subcommand)]
enum AddCommands {
    /// Add a new module with intelligent defaults (DEPRECATED: use 'make module' instead)
    Module {
        /// Module name (e.g., UserModule, BlogModule)
        name: String,

        /// Include providers (comma-separated)
        #[arg(long)]
        providers: Option<String>,

        /// Include controllers (comma-separated)
        #[arg(long)]
        controllers: Option<String>,

        /// Include services (comma-separated)
        #[arg(long)]
        services: Option<String>,
    },

    /// Add a service to an existing module
    Service {
        /// Service name (e.g., EmailService, UserService)
        name: String,

        /// Target module name
        #[arg(long)]
        to: String,

        /// Implement specific trait
        #[arg(long)]
        trait_impl: Option<String>,
    },

    /// Add a controller to an existing module
    Controller {
        /// Controller name (e.g., UserController, BlogController)
        name: String,

        /// Target module name
        #[arg(long)]
        to: String,

        /// Generate CRUD methods
        #[arg(long)]
        crud: bool,
    },

    /// Add middleware to the project
    Middleware {
        /// Middleware name (e.g., Auth, RateLimit, Cors)
        name: String,

        /// Target module name
        #[arg(long)]
        to: Option<String>,

        /// Include debugging and instrumentation
        #[arg(long)]
        debug: bool,
    },

    /// Add a migration file
    Migration {
        /// Migration name
        name: String,
    },

    /// Add a seeder file
    Seeder {
        /// Seeder name
        name: String,
    },
}

#[derive(Subcommand)]
enum InspectCommands {
    /// List and visualize all modules
    Modules {
        /// Show dependency graph
        #[arg(long)]
        graph: bool,

        /// Output format (text, json, dot)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Show dependencies for a specific module
    Dependencies {
        /// Module name to inspect
        #[arg(long)]
        module: String,

        /// Include transitive dependencies
        #[arg(long)]
        transitive: bool,
    },

    /// Show project structure and configuration
    Config {
        /// Show detailed configuration
        #[arg(long)]
        detailed: bool,

        /// Validate configuration
        #[arg(long)]
        validate: bool,
    },
}

#[derive(Subcommand)]
enum DeployCommands {
    /// Prepare for deployment (validation, optimization, packaging)
    Prepare {
        /// Deployment target (docker, kubernetes, serverless)
        #[arg(long, default_value = "docker")]
        target: String,

        /// Environment to prepare for
        #[arg(long, default_value = "production")]
        env: String,
    },
}

// ========== Updated Database Commands ==========

#[derive(Subcommand)]
enum DbCommands {
    /// Database setup and health check
    Setup {
        /// Environment
        #[arg(long, short)]
        env: Option<String>,

        /// Show verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Database status and health reporting
    Status {
        /// Environment
        #[arg(long, short)]
        env: Option<String>,

        /// Show verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Create database
    Create {
        /// Database name
        name: String,

        /// Environment (development, test, staging, production)
        #[arg(long, short, default_value = "development")]
        env: String,
    },

    /// Drop database
    Drop {
        /// Database name
        name: Option<String>,

        /// Environment
        #[arg(long, short, default_value = "development")]
        env: String,

        /// Force drop without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Reset database with fresh migrations and seeds
    Reset {
        /// Include seeds after reset
        #[arg(long)]
        with_seeds: bool,

        /// Environment
        #[arg(long, short)]
        env: Option<String>,

        /// Force reset without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Run database seeders
    Seed {
        /// Environment to run seeders for
        #[arg(long, short)]
        env: Option<String>,

        /// Force run seeders in production
        #[arg(long)]
        force: bool,

        /// Show verbose output
        #[arg(long, short)]
        verbose: bool,
    },

    /// Fresh database with seeds
    Fresh {
        /// Environment
        #[arg(long, short)]
        env: Option<String>,

        /// Skip seeds
        #[arg(long)]
        no_seed: bool,
    },

    /// Create database backup
    Backup {
        /// Backup file path (optional)
        #[arg(long)]
        path: Option<String>,

        /// Enable compression
        #[arg(long)]
        compress: bool,
    },

    /// Restore database from backup
    Restore {
        /// Backup file path
        backup_file: String,
    },

    /// Database performance analysis
    Analyze,
}

// ========== Updated Migration Commands ==========

#[derive(Subcommand)]
enum MigrateCommands {
    /// Convert manual IoC to module system
    IocToModules {
        /// Backup existing code before migration
        #[arg(long, default_value = "true")]
        backup: bool,

        /// Dry run (show what would be migrated)
        #[arg(long)]
        dry_run: bool,
    },

    /// Run pending database migrations
    Up {
        /// Number of migrations to run
        #[arg(long)]
        steps: Option<u32>,
    },

    /// Rollback database migrations
    Down {
        /// Number of migrations to rollback
        #[arg(long, default_value = "1")]
        steps: u32,
    },

    /// Show migration status
    Status,

    /// Create a new migration
    Create {
        /// Migration name
        name: String,
    },
}

// ========== Legacy Command Support ==========

#[derive(Subcommand)]
enum ApiVersionCommands {
    /// Create a new API version
    Create {
        /// Version identifier (e.g., v2, 1.0)
        version: String,

        /// Optional description for the version
        #[arg(long)]
        description: Option<String>,
    },

    /// Deprecate an API version
    Deprecate {
        /// Version to deprecate
        version: String,

        /// Deprecation message
        #[arg(long)]
        message: Option<String>,

        /// Sunset date (ISO 8601 format)
        #[arg(long)]
        sunset_date: Option<String>,
    },

    /// List all API versions
    List,

    /// Generate migration guide between versions
    Migrate {
        /// Source version
        from: String,

        /// Target version
        to: String,
    },

    /// Validate API version configuration
    Validate,
}

// ========== Module System Commands ==========

#[derive(Subcommand)]
enum ModuleCommands {
    /// List all modules and their dependencies
    List {
        /// Show dependency relationships
        #[arg(long)]
        dependencies: bool,
    },

    /// Generate and visualize module dependency graph
    Graph {
        /// Output format (text, dot, svg, json)
        #[arg(long, default_value = "text")]
        format: String,

        /// Output file path (optional)
        #[arg(long)]
        output: Option<String>,
    },

    /// Convert manual IoC setup to module system
    Migrate {
        /// Analyze existing setup before migration
        #[arg(long)]
        analyze_first: bool,
    },

    /// Validate module composition and detect issues
    Validate {
        /// Automatically fix issues where possible
        #[arg(long)]
        fix_issues: bool,
    },
}

// ========== Make Commands ==========

#[derive(Subcommand)]
enum MakeCommands {
    /// Generate a new module with providers and controllers
    Module {
        /// Module name (e.g., UserModule, BlogModule)
        name: String,

        /// Include providers (comma-separated)
        #[arg(long)]
        providers: Option<String>,

        /// Include controllers (comma-separated)
        #[arg(long)]
        controllers: Option<String>,

        /// Include services (comma-separated)
        #[arg(long)]
        services: Option<String>,
    },

    /// Generate a new database seeder
    Seeder {
        /// Seeder name (e.g., UserSeeder, BlogSeeder)
        name: String,

        /// Target table or model name
        #[arg(long)]
        table: Option<String>,

        /// Generate with factory integration
        #[arg(long)]
        factory: bool,
    },

    /// Generate a complete REST API with CRUD operations
    Api {
        /// Resource name (e.g., User, Blog, Product)
        resource: String,

        /// API version (e.g., v1, v2)
        #[arg(long, default_value = "v1")]
        version: String,

        /// Target module name
        #[arg(long)]
        module: Option<String>,

        /// Include authentication middleware
        #[arg(long)]
        auth: bool,

        /// Include validation
        #[arg(long)]
        validation: bool,

        /// Generate OpenAPI documentation
        #[arg(long)]
        docs: bool,
    },

    /// Generate a complete CRUD system with model, controller, and service
    Crud {
        /// Resource name (e.g., User, Blog, Product)
        resource: String,

        /// Resource fields (e.g., "name:string,email:string,age:int")
        #[arg(long)]
        fields: Option<String>,

        /// Relationships (e.g., "User:belongs_to,Posts:has_many")
        #[arg(long)]
        relationships: Option<String>,

        /// Target module name
        #[arg(long)]
        module: Option<String>,

        /// Include migration
        #[arg(long)]
        migration: bool,

        /// Include tests
        #[arg(long)]
        tests: bool,

        /// Include factory
        #[arg(long)]
        factory: bool,
    },

    /// Generate a business logic service with dependency injection
    Service {
        /// Service name (e.g., EmailService, PaymentService)
        name: String,

        /// Target module name
        #[arg(long)]
        module: Option<String>,

        /// Implement specific trait
        #[arg(long)]
        trait_impl: Option<String>,

        /// Service dependencies (comma-separated)
        #[arg(long)]
        dependencies: Option<String>,

        /// Include async methods
        #[arg(long)]
        async_methods: bool,
    },

    /// Generate testing and seeding factories with relationships
    Factory {
        /// Model name (e.g., User, Blog, Product)
        model: String,

        /// Number of default instances to create
        #[arg(long, default_value = "10")]
        count: u32,

        /// Related factories (comma-separated)
        #[arg(long)]
        relationships: Option<String>,

        /// Include traits (e.g., Faker, Randomized)
        #[arg(long)]
        traits: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), ElifError> {
    let cli = Cli::parse();

    match cli.command {
        // ========== New Command: Convention Over Configuration ==========
        Commands::New { name } => {
            // Zero configuration - just create a great API project with zero-boilerplate bootstrap
            create_new_app(&name).await?;
        }

        // ========== Upgrade Command: Self-Update CLI ==========
        Commands::Upgrade { force } => {
            update_cli(force).await?;
        }

        // ========== Original Elif.rs Command Structure ==========
        Commands::Create { create_command } => match create_command {
            CreateCommands::App {
                name,
                path,
                template,
                // modules always enabled
            } => {
                commands::create::app(&name, path.as_deref(), &template).await?;
            }
        },

        Commands::Add { add_command } => match add_command {
            AddCommands::Module {
                name,
                providers,
                controllers,
                services,
            } => {
                println!("⚠️  DEPRECATED: 'add module' is deprecated. Use 'make module' instead.");
                println!("   This command will be removed in a future version.\n");
                commands::add::module(
                    &name,
                    providers.as_deref(),
                    controllers.as_deref(),
                    services.as_deref(),
                )
                .await?;
            }
            AddCommands::Service {
                name,
                to,
                trait_impl,
            } => {
                commands::add::service(&name, &to, trait_impl.as_deref()).await?;
            }
            AddCommands::Controller { name, to, crud } => {
                commands::add::controller(&name, &to, crud).await?;
            }
            AddCommands::Middleware { name, to, debug } => {
                commands::add::middleware(&name, to.as_deref(), debug).await?;
            }
            AddCommands::Migration { name } => {
                commands::add::migration(&name).await?;
            }
            AddCommands::Seeder { name } => {
                commands::add::seeder(&name).await?;
            }
        },

        Commands::Inspect { inspect_command } => match inspect_command {
            InspectCommands::Modules { graph, format } => {
                commands::inspect::modules(graph, &format).await?;
            }
            InspectCommands::Dependencies { module, transitive } => {
                commands::inspect::dependencies(&module, transitive).await?;
            }
            InspectCommands::Config { detailed, validate } => {
                commands::inspect::config(detailed, validate).await?;
            }
        },

        Commands::Serve {
            port,
            host,
            hot_reload,
            watch,
            exclude,
            env,
        } => {
            let args = commands::serve::ServeArgs {
                port,
                host,
                hot_reload,
                watch,
                exclude,
                env,
            };
            commands::serve::run(args).await?;
        }

        Commands::Watch {
            paths,
            exclude,
            command,
            debounce,
        } => {
            commands::serve::watch(paths, exclude, command, debounce).await?;
        }

        Commands::Dev {
            watch,
            profile,
            port,
            host,
            env,
        } => {
            commands::dev::run(watch, profile, port, &host, &env).await?;
        }

        Commands::Check {
            comprehensive,
            module,
        } => {
            commands::check::run(comprehensive, module.as_deref()).await?;
        }

        Commands::Doctor {
            fix_issues,
            verbose,
        } => {
            commands::doctor::run(fix_issues, verbose).await?;
        }

        Commands::Test {
            unit,
            integration,
            watch,
            coverage,
            module,
        } => {
            commands::test::run(unit, integration, watch, coverage, module.as_deref()).await?;
        }

        Commands::Db { db_command } => match db_command {
            DbCommands::Setup { env, verbose } => {
                commands::db::setup(env.as_deref(), verbose).await?;
            }
            DbCommands::Status { env, verbose } => {
                commands::db::status(env.as_deref(), verbose).await?;
            }
            DbCommands::Create { name, env } => {
                commands::db::create(&name, &env).await?;
            }
            DbCommands::Drop { name, env, force } => {
                commands::db::drop(name.as_deref(), &env, force).await?;
            }
            DbCommands::Reset { with_seeds, env, force } => {
                commands::db::reset(with_seeds, env.as_deref(), force).await?;
            }
            DbCommands::Seed {
                env,
                force,
                verbose,
            } => {
                commands::db::seed(env.as_deref(), force, verbose).await?;
            }
            DbCommands::Fresh { env, no_seed } => {
                commands::db::fresh(env.as_deref(), !no_seed).await?;
            }
            DbCommands::Backup { path, compress } => {
                commands::db::backup(path.as_deref(), compress).await?;
            }
            DbCommands::Restore { backup_file } => {
                commands::db::restore(&backup_file).await?;
            }
            DbCommands::Analyze => {
                commands::db::analyze().await?;
            }
        },

        Commands::Migrate { migrate_command } => match migrate_command {
            MigrateCommands::IocToModules { backup, dry_run } => {
                commands::migrate::ioc_to_modules(backup, dry_run).await?;
            }
            MigrateCommands::Up { steps } => {
                commands::migrate::up(steps).await?;
            }
            MigrateCommands::Down { steps } => {
                commands::migrate::down(steps).await?;
            }
            MigrateCommands::Status => {
                commands::migrate::status().await?;
            }
            MigrateCommands::Create { name } => {
                commands::migrate::create(&name).await?;
            }
        },

        Commands::Build {
            release,
            target,
            optimizations,
        } => {
            commands::build::run(release, &target, optimizations).await?;
        }

        Commands::Optimize {
            routes,
            assets,
            config,
            force,
        } => {
            commands::optimize::run(routes, assets, config, force).await?;
        }

        Commands::Deploy { deploy_command } => match deploy_command {
            DeployCommands::Prepare { target, env } => {
                commands::deploy::prepare(&target, &env).await?;
            }
        },

        Commands::Info { detailed, modules } => {
            commands::info::run(detailed, modules).await?;
        }

        Commands::Status { health, component } => {
            commands::status::run(health, component.as_deref()).await?;
        }

        Commands::Update {
            check,
            security,
            dependencies,
            verbose,
        } => {
            commands::update::run(check, security, dependencies, verbose).await?;
        }

        Commands::Version { version_command } => match version_command {
            ApiVersionCommands::Create {
                version,
                description,
            } => {
                commands::version::create(&version, description.as_deref()).await?;
            }
            ApiVersionCommands::Deprecate {
                version,
                message,
                sunset_date,
            } => {
                commands::version::deprecate(&version, message.as_deref(), sunset_date.as_deref())
                    .await?;
            }
            ApiVersionCommands::List => {
                commands::version::list().await?;
            }
            ApiVersionCommands::Migrate { from, to } => {
                commands::version::migrate(&from, &to).await?;
            }
            ApiVersionCommands::Validate => {
                commands::version::validate().await?;
            }
        },

        Commands::Module { module_command } => match module_command {
            ModuleCommands::List { dependencies } => {
                commands::module::list(dependencies).await?;
            }
            ModuleCommands::Graph { format, output } => {
                commands::module::graph(&format, output.as_deref()).await?;
            }
            ModuleCommands::Migrate { analyze_first } => {
                commands::module::migrate(analyze_first).await?;
            }
            ModuleCommands::Validate { fix_issues } => {
                commands::module::validate(fix_issues).await?;
            }
        },

        Commands::Make { make_command } => match make_command {
            MakeCommands::Module {
                name,
                providers,
                controllers,
                services,
            } => {
                commands::add::module(
                    &name,
                    providers.as_deref(),
                    controllers.as_deref(),
                    services.as_deref(),
                )
                .await?;
            }
            MakeCommands::Seeder {
                name,
                table,
                factory,
            } => {
                commands::add::seeder_with_options(
                    &name,
                    table.as_deref(),
                    factory,
                )
                .await?;
            }
            MakeCommands::Api {
                resource,
                version,
                module,
                auth,
                validation,
                docs,
            } => {
                commands::make::api(
                    &resource,
                    &version,
                    module.as_deref(),
                    auth,
                    validation,
                    docs,
                )
                .await?;
            }
            MakeCommands::Crud {
                resource,
                fields,
                relationships,
                module,
                migration,
                tests,
                factory,
            } => {
                commands::make::crud(
                    &resource,
                    fields.as_deref(),
                    relationships.as_deref(),
                    module.as_deref(),
                    migration,
                    tests,
                    factory,
                )
                .await?;
            }
            MakeCommands::Service {
                name,
                module,
                trait_impl,
                dependencies,
                async_methods,
            } => {
                commands::make::service(
                    &name,
                    module.as_deref(),
                    trait_impl.as_deref(),
                    dependencies.as_deref(),
                    async_methods,
                )
                .await?;
            }
            MakeCommands::Factory {
                model,
                count,
                relationships,
                traits,
            } => {
                commands::make::factory(
                    &model,
                    count,
                    relationships.as_deref(),
                    traits.as_deref(),
                )
                .await?;
            }
        },
    }

    Ok(())
}

/// Create a new elif application with convention over configuration
/// Zero choices, great defaults - just works
async fn create_new_app(name: &str) -> Result<(), ElifError> {
    println!("🚀 Creating new elif.rs application '{}'...", name);
    
    // Create project directory
    let project_path = Path::new(name);
    if project_path.exists() {
        return Err(ElifError::Validation {
            message: format!("Directory '{}' already exists", name),
        });
    }
    
    // Create production-ready directory structure
    println!("📁 Creating directory structure...");
    
    // Core directories
    fs::create_dir_all(project_path.join("src"))?;
    fs::create_dir_all(project_path.join("tests"))?;
    fs::create_dir_all(project_path.join("docs"))?;
    fs::create_dir_all(project_path.join("migrations"))?;
    
    // NestJS-style modular structure
    fs::create_dir_all(project_path.join("src/modules"))?;
    fs::create_dir_all(project_path.join("src/modules/app"))?;
    fs::create_dir_all(project_path.join("src/modules/users"))?;
    fs::create_dir_all(project_path.join("src/modules/users/dto"))?;
    
    // Traditional structure for services, middleware, etc.
    fs::create_dir_all(project_path.join("src/services"))?;
    fs::create_dir_all(project_path.join("src/middleware"))?;
    fs::create_dir_all(project_path.join("src/models"))?;
    fs::create_dir_all(project_path.join("src/controllers"))?;
    
    // Create Cargo.toml with sensible defaults
    let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
# elif.rs framework components
elif-web = "0.8.6"
elif-core = "0.7.1"
elif-http = "0.8.8"
elif-http-derive = "0.2.10"
elif-macros = "0.2.0"
elif-orm = "0.7.1"

# Common dependencies
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["full"] }}
async-trait = "0.1"
env_logger = "0.10"
ctor = "0.2"

[dev-dependencies]
"#, name);
    
    fs::write(project_path.join("Cargo.toml"), cargo_toml)?;
    
    // Create main.rs with modular structure
    let main_rs = format!(r#"use elif_http::{{HttpError, AppBootstrap}};
use elif_macros::bootstrap;
use elif_web::prelude::*;

// Import modules
mod modules;
mod controllers;

use modules::users::UsersModule;

// Define the root application module
#[module(
    imports: [UsersModule],
    is_app
)]
pub struct AppModule;

#[bootstrap(AppModule)]
async fn main() -> Result<(), HttpError> {{
    println!("🚀 Starting {} server...", "{}");
    println!("📊 Health check: http://127.0.0.1:3000/api/health");
    println!("👥 Users API: http://127.0.0.1:3000/api/users");
    
    // Modular architecture! 🏗️
    // - NestJS-style module organization
    // - Clean separation of concerns
    // - Scalable project structure
    // - Production-ready setup
    Ok(())
}}
"#, name, name);
    
    fs::write(project_path.join("src/main.rs"), main_rs)?;
    
    // Create modules/mod.rs
    let modules_mod = "pub mod app;\npub mod users;";
    fs::write(project_path.join("src/modules/mod.rs"), modules_mod)?;
    
    // Create app module files
    let app_module_rs = format!(r#"use elif_web::prelude::*;
use crate::controllers::HealthController;

#[module(
    controllers: [HealthController]
)]
pub struct AppModule;
"#);
    
    let app_mod_rs = "pub mod app_module;\npub use app_module::AppModule;";
    fs::write(project_path.join("src/modules/app/mod.rs"), app_mod_rs)?;
    fs::write(project_path.join("src/modules/app/app_module.rs"), app_module_rs)?;
    
    // Create users module files
    let users_module_rs = format!(r#"use elif_web::prelude::*;
use super::users_controller::UsersController;
use super::users_service::UsersService;

#[module(
    controllers: [UsersController],
    providers: [UsersService]
)]
pub struct UsersModule;
"#);
    
    let users_controller_rs = format!(r#"use elif_web::prelude::*;
use serde_json::json;
use super::users_service::UsersService;
use super::dto::{{CreateUserDto, UpdateUserDto}};

#[derive(Default)]
pub struct UsersController {{
    users_service: Option<UsersService>,
}}

#[controller("/api/users")]
impl UsersController {{
    #[get("/")]
    pub async fn index(&self) -> HttpResult<ElifResponse> {{
        // TODO: Implement with users_service.find_all()
        Ok(ElifResponse::ok().json(&json!({{
            "users": [],
            "total": 0,
            "message": "Users list endpoint - implement with your database"
        }}))?)
    }}

    #[post("/")]
    #[body(dto: CreateUserDto)]
    pub async fn create(&self, dto: CreateUserDto) -> HttpResult<ElifResponse> {{
        // TODO: Implement with users_service.create(dto)
        Ok(ElifResponse::created().json(&json!({{
            "message": "User creation endpoint - implement with your database",
            "user": {{ "id": 1, "name": "New User" }}
        }}))?)
    }}

    #[get("/{{id}}")]
    #[param(id: u32)]
    pub async fn show(&self, id: u32) -> HttpResult<ElifResponse> {{
        // TODO: Implement with users_service.find_by_id(id)
        Ok(ElifResponse::ok().json(&json!({{
            "user": {{ "id": id, "name": "Sample User" }},
            "message": "User detail endpoint - implement with your database"
        }}))?)
    }}

    #[put("/{{id}}")]
    #[param(id: u32)]
    #[body(dto: UpdateUserDto)]
    pub async fn update(&self, id: u32, dto: UpdateUserDto) -> HttpResult<ElifResponse> {{
        // TODO: Implement with users_service.update(id, dto)
        Ok(ElifResponse::ok().json(&json!({{
            "user": {{ "id": id, "name": "Updated User" }},
            "message": "User update endpoint - implement with your database"
        }}))?)
    }}

    #[delete("/{{id}}")]
    #[param(id: u32)]
    pub async fn destroy(&self, id: u32) -> HttpResult<ElifResponse> {{
        // TODO: Implement with users_service.delete(id)
        Ok(ElifResponse::ok().json(&json!({{
            "message": "User deleted successfully",
            "deleted_id": id
        }}))?)
    }}
}}
"#);
    
    let users_service_rs = format!(r#"use elif_core::container::Injectable;
use super::dto::{{CreateUserDto, UpdateUserDto}};

#[derive(Default)]
pub struct UsersService;

impl UsersService {{
    pub async fn find_all(&self) -> Result<Vec<User>, String> {{
        // TODO: Implement database query
        Ok(vec![])
    }}

    pub async fn find_by_id(&self, id: u32) -> Result<Option<User>, String> {{
        // TODO: Implement database query
        Ok(Some(User {{ id, name: "Sample User".to_string() }}))
    }}

    pub async fn create(&self, _dto: CreateUserDto) -> Result<User, String> {{
        // TODO: Implement database insertion
        Ok(User {{ id: 1, name: "New User".to_string() }})
    }}

    pub async fn update(&self, id: u32, _dto: UpdateUserDto) -> Result<User, String> {{
        // TODO: Implement database update
        Ok(User {{ id, name: "Updated User".to_string() }})
    }}

    pub async fn delete(&self, id: u32) -> Result<(), String> {{
        // TODO: Implement database deletion
        println!("Deleted user with id: {{}}", id);
        Ok(())
    }}
}}

// TODO: Replace with your actual User model
#[derive(Debug, Clone)]
pub struct User {{
    pub id: u32,
    pub name: String,
}}
"#);
    
    // Create DTOs
    let create_user_dto = r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserDto {
    pub name: String,
    pub email: String,
}
"#;
    
    let update_user_dto = r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUserDto {
    pub name: Option<String>,
    pub email: Option<String>,
}
"#;
    
    let dto_mod = "pub mod create_user_dto;\npub mod update_user_dto;\n\npub use create_user_dto::CreateUserDto;\npub use update_user_dto::UpdateUserDto;";
    
    // Write users module files
    let users_mod_rs = "pub mod users_module;\npub mod users_controller;\npub mod users_service;\npub mod dto;\n\npub use users_module::UsersModule;";
    fs::write(project_path.join("src/modules/users/mod.rs"), users_mod_rs)?;
    fs::write(project_path.join("src/modules/users/users_module.rs"), users_module_rs)?;
    fs::write(project_path.join("src/modules/users/users_controller.rs"), users_controller_rs)?;
    fs::write(project_path.join("src/modules/users/users_service.rs"), users_service_rs)?;
    fs::write(project_path.join("src/modules/users/dto/mod.rs"), dto_mod)?;
    fs::write(project_path.join("src/modules/users/dto/create_user_dto.rs"), create_user_dto)?;
    fs::write(project_path.join("src/modules/users/dto/update_user_dto.rs"), update_user_dto)?;
    
    // Create health controller in controllers directory
    let health_controller_rs = format!(r#"use elif_web::prelude::*;
use serde_json::json;

#[derive(Default)]
pub struct HealthController;

#[controller("/api")]
impl HealthController {{
    #[get("/health")]
    pub async fn health(&self) -> HttpResult<ElifResponse> {{
        Ok(ElifResponse::ok().json(&json!({{
            "status": "ok",
            "service": "{}",
            "version": "1.0",
            "framework": "elif.rs"
        }}))?)
    }}
}}
"#, name);
    
    fs::write(project_path.join("src/controllers/mod.rs"), "pub mod health_controller;\npub use health_controller::HealthController;")?;
    fs::write(project_path.join("src/controllers/health_controller.rs"), health_controller_rs)?;
    
    // Create production-ready .env file
    let env_file = format!(r#"# Application Environment
APP_NAME={}
APP_ENV=development
APP_KEY=generate_with_elifrs_auth_generate_key

# Server Configuration
HOST=127.0.0.1
PORT=3000

# Database Configuration (uncomment and configure as needed)
# DATABASE_URL=postgresql://user:password@localhost/{}
# DATABASE_URL=mysql://user:password@localhost/{}
# DATABASE_URL=sqlite:./{}.db

# Logging
RUST_LOG=info
RUST_BACKTRACE=1

# Authentication (if using auth features)
# JWT_SECRET=your-secret-key-here
# SESSION_SECRET=your-session-secret-here
"#, name, name, name, name);
    fs::write(project_path.join(".env"), env_file)?;
    
    // Create comprehensive .gitignore
    let gitignore = r#"# Rust
/target/
Cargo.lock

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# Environment
.env
.env.local
.env.production
.env.*.local

# Logs
*.log
logs/

# Database
*.db
*.sqlite
*.sqlite3

# OS
.DS_Store
Thumbs.db

# Development
/tmp/
/temp/
.cache/

# Documentation
/docs/build/

# Testing
coverage/
.nyc_output/

# Node.js (if using frontend tools)
node_modules/
npm-debug.log*
yarn-debug.log*
yarn-error.log*
"#;
    fs::write(project_path.join(".gitignore"), gitignore)?;

    // Create README.md
    let readme = format!(r#"# {}

A modern web application built with [elif.rs](https://github.com/krcpa/elif.rs) - The Laravel of Rust.

## 🚀 Quick Start

```bash
# Development server with hot reload
elifrs dev

# Build for production
cargo build --release

# Run tests
cargo test
```

## 📁 Project Structure

```
{}/ 
├── src/
│   ├── modules/           # Feature modules (NestJS-style)
│   │   ├── app/          # Application core module
│   │   └── users/        # Users feature module
│   │       ├── dto/      # Data Transfer Objects
│   │       ├── users_controller.rs
│   │       ├── users_service.rs
│   │       └── users_module.rs
│   ├── controllers/      # Shared controllers
│   ├── services/         # Shared services
│   ├── middleware/       # Custom middleware
│   ├── models/          # Data models
│   └── main.rs          # Application entry point
├── tests/               # Integration tests
├── migrations/          # Database migrations
├── docs/               # Documentation
└── .env                # Environment configuration
```

## 🏗️ Architecture

This project follows a **modular architecture** inspired by NestJS:

- **Modules**: Self-contained feature units with controllers, services, and DTOs
- **Controllers**: Handle HTTP requests and responses
- **Services**: Business logic and data access
- **DTOs**: Data validation and serialization
- **Dependency Injection**: Automatic service resolution

## 📚 API Endpoints

### Health Check
- `GET /api/health` - Application health status

### Users API
- `GET /api/users` - List all users
- `POST /api/users` - Create a new user
- `GET /api/users/{{id}}` - Get user by ID
- `PUT /api/users/{{id}}` - Update user
- `DELETE /api/users/{{id}}` - Delete user

## 🔧 Configuration

Environment variables in `.env`:

```env
APP_NAME={}
APP_ENV=development
HOST=127.0.0.1
PORT=3000
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with coverage
cargo test --verbose
```

## 📖 Documentation

- [elif.rs Documentation](https://github.com/krcpa/elif.rs)
- [API Documentation](./docs/) (generate with `elifrs docs`)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License.
"#, name, name, name);
    
    fs::write(project_path.join("README.md"), readme)?;
    
    // Create basic test file
    let test_file = format!(r#"use {}::*;

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_basic() {{
        // Add your tests here
        assert_eq!(2 + 2, 4);
    }}
}}
"#, name.replace("-", "_"));
    
    fs::write(project_path.join("tests/integration_test.rs"), test_file)?;
    
    // Create docs/README.md
    let docs_readme = format!(r#"# {} Documentation

## Overview

This directory contains documentation for the {} application.

## Structure

- `api/` - API documentation
- `architecture/` - System architecture docs
- `deployment/` - Deployment guides

## Generating Documentation

```bash
# Generate API documentation
elifrs docs

# Generate code documentation
cargo doc --open
```
"#, name, name);
    
    fs::create_dir_all(project_path.join("docs"))?;
    fs::write(project_path.join("docs/README.md"), docs_readme)?;

    // Initialize git repository
    println!("🔧 Initializing git repository...");
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&project_path)
        .output()
        .map_err(|e| ElifError::Validation {
            message: format!("Failed to initialize git repository: {}", e),
        })?;

    // Add initial commit
    std::process::Command::new("git")
        .args(&["add", "."])
        .current_dir(&project_path)
        .output()
        .map_err(|e| ElifError::Validation {
            message: format!("Failed to add files to git: {}", e),
        })?;

    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit from elifrs CLI"])
        .current_dir(&project_path)
        .output()
        .map_err(|e| ElifError::Validation {
            message: format!("Failed to create initial commit: {}", e),
        })?;

    println!("✅ Created {} successfully!", name);
    println!();
    println!("Next steps:");
    println!("   cd {}", name);
    println!("   elifrs dev");
    println!();
    println!("🌐 Your API will be available at:");
    println!("   📊 Health: http://127.0.0.1:3000/api/health");
    println!("   👥 Users:  http://127.0.0.1:3000/api/users");
    println!();
    println!("📖 Documentation:");
    println!("   📋 README: {}/README.md", name);
    println!("   📚 Docs:   {}/docs/", name);
    
    Ok(())
}

/// Update the elifrs CLI to the latest version
async fn update_cli(force: bool) -> Result<(), ElifError> {
    const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
    
    println!("🔍 Checking for elifrs updates...");
    println!("   Current version: v{}", CURRENT_VERSION);
    
    // Check latest version on crates.io
    let output = Command::new("cargo")
        .args(&["search", "elifrs", "--limit", "1"])
        .output()
        .map_err(|e| ElifError::validation(&format!("Failed to check for updates: {}", e)))?;
    
    if !output.status.success() {
        return Err(ElifError::validation("Failed to query crates.io for latest version"));
    }
    
    let search_output = String::from_utf8_lossy(&output.stdout);
    let latest_version = extract_latest_version(&search_output)?;
    
    println!("   Latest version: v{}", latest_version);
    
    if !force && latest_version == CURRENT_VERSION {
        println!("✅ You're already on the latest version!");
        return Ok(());
    }
    
    if !force && is_newer_version(&latest_version, CURRENT_VERSION)? {
        println!("⬆️  Update available: v{} → v{}", CURRENT_VERSION, latest_version);
    } else if force {
        println!("🔄 Force updating to v{}", latest_version);
    } else {
        println!("✅ You're already on the latest version!");
        return Ok(());
    }
    
    // Perform the update
    println!("📦 Installing elifrs v{}...", latest_version);
    
    let install_output = Command::new("cargo")
        .args(&["install", "elifrs", "--force"])
        .output()
        .map_err(|e| ElifError::validation(&format!("Failed to install update: {}", e)))?;
    
    if !install_output.status.success() {
        let error_msg = String::from_utf8_lossy(&install_output.stderr);
        return Err(ElifError::validation(&format!("Installation failed: {}", error_msg)));
    }
    
    println!("✅ Successfully updated to elifrs v{}!", latest_version);
    println!("🎉 Run 'elifrs --version' to verify the update");
    
    Ok(())
}

/// Extract the latest version from cargo search output
fn extract_latest_version(search_output: &str) -> Result<String, ElifError> {
    // Parse output like: elifrs = "0.11.0"    # description
    for line in search_output.lines() {
        if line.starts_with("elifrs = ") {
            if let Some(version_part) = line.split('"').nth(1) {
                return Ok(version_part.to_string());
            }
        }
    }
    
    Err(ElifError::validation("Could not parse version from crates.io response"))
}

/// Check if new_version is newer than current_version
fn is_newer_version(new_version: &str, current_version: &str) -> Result<bool, ElifError> {
    // Simple version comparison (assumes semantic versioning)
    let new_parts: Vec<u32> = new_version.split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    let current_parts: Vec<u32> = current_version.split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    
    // Compare major.minor.patch
    for i in 0..3 {
        let new_part = new_parts.get(i).unwrap_or(&0);
        let current_part = current_parts.get(i).unwrap_or(&0);
        
        if new_part > current_part {
            return Ok(true);
        } else if new_part < current_part {
            return Ok(false);
        }
    }
    
    Ok(false) // Versions are equal
}
