mod commands;
mod generators;  // Re-enabled for make commands
// mod interactive; // Disabled to fix compilation - contains unused code

use clap::{Parser, Subcommand};
use elif_core::ElifError;

#[derive(Parser)]
#[command(name = "elif")]
#[command(about = "LLM-friendly Rust web framework CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

        /// Include module system setup
        #[arg(long)]
        modules: bool,
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
        // ========== Original Elif.rs Command Structure ==========
        Commands::Create { create_command } => match create_command {
            CreateCommands::App {
                name,
                path,
                template,
                modules,
            } => {
                commands::create::app(&name, path.as_deref(), &template, modules).await?;
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
