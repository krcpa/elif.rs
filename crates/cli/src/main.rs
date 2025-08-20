mod commands;
mod generators;
mod interactive;

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
    Generate {
        #[command(subcommand)]
        generate_command: Option<GenerateCommands>,
    },
    
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
    
    /// Database seeding and factory management
    Db {
        #[command(subcommand)]
        db_command: DbCommands,
    },
    
    /// Authentication management
    Auth {
        #[command(subcommand)]
        auth_command: AuthCommands,
    },
    
    /// Start development server with hot reload
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
    
    /// Queue management commands
    Queue {
        #[command(subcommand)]
        queue_command: QueueCommands,
    },
    
    /// Interactive project setup wizard
    Setup {
        /// Skip interactive mode and use defaults
        #[arg(long)]
        non_interactive: bool,
        
        /// Show verbose output during setup
        #[arg(long, short)]
        verbose: bool,
    },
    
    /// Email system management
    Email {
        #[command(subcommand)]
        email_command: EmailCommands,
    },
    
    /// API version management
    Version {
        #[command(subcommand)]
        version_command: ApiVersionCommands,
    },
}

#[derive(Subcommand)]
enum GenerateCommands {
    /// Generate middleware with handle(request, next) pattern
    Middleware {
        /// Middleware name (e.g., Auth, RateLimit, Cors)
        name: String,
        
        /// Include debugging and instrumentation
        #[arg(long)]
        debug: bool,
        
        /// Generate with conditional execution (skip paths, only methods)
        #[arg(long)]
        conditional: bool,
        
        /// Include basic tests
        #[arg(long)]
        tests: bool,
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
    /// Generate OpenAPI specification from project
    Generate {
        /// Output file path
        #[arg(long, short)]
        output: Option<String>,
        
        /// Output format (json, yaml)
        #[arg(long, short)]
        format: Option<String>,
    },
    
    /// Export OpenAPI spec to different formats
    Export {
        /// Export format (postman, insomnia)
        #[arg(long, short)]
        format: String,
        
        /// Output file path
        #[arg(long, short)]
        output: String,
    },
    
    /// Serve interactive Swagger UI documentation
    Serve {
        /// Port to serve on
        #[arg(long, short, default_value = "8080")]
        port: u16,
    },
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
enum DbCommands {
    /// Run database seeders
    Seed {
        /// Environment to run seeders for (development, testing, staging, production)
        #[arg(long, short)]
        env: Option<String>,
        
        /// Force run seeders in production environment
        #[arg(long)]
        force: bool,
        
        /// Show verbose output during seeding
        #[arg(long, short)]
        verbose: bool,
    },
    
    /// Show factory system status
    Factory {
        #[command(subcommand)]
        factory_command: FactoryCommands,
    },
    
    /// Show seeding system status
    Status,
}

#[derive(Subcommand)]
enum FactoryCommands {
    /// Show factory system status
    Status,
    
    /// Test factory system with sample data generation
    Test {
        /// Number of sample records to generate
        #[arg(long, short, default_value = "3")]
        count: usize,
    },
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
enum QueueCommands {
    /// Process background jobs from queues
    Work {
        /// Queue names to process (comma-separated)
        #[arg(long, short, default_value = "default")]
        queue: String,
        
        /// Maximum number of jobs to process
        #[arg(long, short)]
        max_jobs: Option<u32>,
        
        /// Timeout in seconds for each job
        #[arg(long, short, default_value = "60")]
        timeout: u64,
        
        /// Sleep time between checks (in milliseconds)
        #[arg(long, default_value = "1000")]
        sleep: u64,
        
        /// Number of worker processes
        #[arg(long, short, default_value = "1")]
        workers: u8,
        
        /// Stop after this many seconds
        #[arg(long)]
        stop_when_empty: bool,
        
        /// Show verbose output
        #[arg(long, short)]
        verbose: bool,
    },
    
    /// Show queue status
    Status {
        /// Queue names to show status for
        #[arg(long, short)]
        queue: Option<String>,
        
        /// Show detailed job information
        #[arg(long, short)]
        detailed: bool,
        
        /// Refresh interval in seconds (0 for no refresh)
        #[arg(long, short, default_value = "0")]
        refresh: u64,
    },
    
    /// Execute scheduled commands and cron jobs
    Schedule {
        /// Run only jobs scheduled for specific time
        #[arg(long)]
        time: Option<String>,
        
        /// Run jobs for specific frequency (minutely, hourly, daily, weekly, monthly)
        #[arg(long)]
        frequency: Option<String>,
        
        /// Run specific scheduled job by name
        #[arg(long)]
        job: Option<String>,
        
        /// Dry run - show what would be executed
        #[arg(long)]
        dry_run: bool,
        
        /// Force run even if not scheduled
        #[arg(long)]
        force: bool,
        
        /// Show verbose output
        #[arg(long, short)]
        verbose: bool,
        
        /// Run as daemon (continuous scheduling)
        #[arg(long, short)]
        daemon: bool,
        
        /// Check interval in seconds when running as daemon
        #[arg(long, default_value = "60")]
        check_interval: u64,
    },
}

#[derive(Subcommand)]
enum EmailCommands {
    /// Send test emails
    Send {
        /// Recipient email address
        to: String,
        
        /// Email subject
        #[arg(long)]
        subject: String,
        
        /// Email template to use
        #[arg(long)]
        template: Option<String>,
        
        /// Email body (if not using template)
        #[arg(long)]
        body: Option<String>,
        
        /// Send as HTML
        #[arg(long)]
        html: bool,
        
        /// Context data as JSON
        #[arg(long)]
        context: Option<String>,
    },
    
    /// Template management
    Template {
        #[command(subcommand)]
        template_command: EmailTemplateCommands,
    },
    
    /// Provider configuration and testing
    Provider {
        #[command(subcommand)]
        provider_command: EmailProviderCommands,
    },
    
    /// Queue management for emails
    Queue {
        #[command(subcommand)]
        queue_command: EmailQueueCommands,
    },
    
    /// Email tracking and analytics
    Track {
        #[command(subcommand)]
        track_command: EmailTrackCommands,
    },
    
    /// Email configuration wizard
    Setup {
        /// Email provider (smtp, mailgun, sendgrid, postmark)
        #[arg(long)]
        provider: Option<String>,
        
        /// Skip interactive mode and use defaults
        #[arg(long)]
        non_interactive: bool,
    },
    
    /// Email testing and inspection commands
    Test {
        #[command(subcommand)]
        test_command: EmailTestCommands,
    },
}

#[derive(Subcommand)]
enum EmailTemplateCommands {
    /// List all available templates
    List,
    
    /// Validate template syntax
    Validate {
        /// Template name or file path
        template: String,
    },
    
    /// Render template with context data
    Render {
        /// Template name or file path
        template: String,
        
        /// Context data as JSON
        #[arg(long)]
        context: Option<String>,
        
        /// Output format (html, text, both)
        #[arg(long, default_value = "both")]
        format: String,
    },
}

#[derive(Subcommand)]
enum EmailProviderCommands {
    /// Test email provider connection
    Test {
        /// Provider name (smtp, mailgun, sendgrid, postmark)
        #[arg(long)]
        provider: Option<String>,
    },
    
    /// Configure email provider
    Configure {
        /// Provider name
        provider: String,
        
        /// Interactive configuration mode
        #[arg(long)]
        interactive: bool,
    },
    
    /// Switch active email provider
    Switch {
        /// Provider name
        provider: String,
    },
}

#[derive(Subcommand)]
enum EmailQueueCommands {
    /// Show email queue status
    Status {
        /// Show detailed job information
        #[arg(long, short)]
        detailed: bool,
    },
    
    /// Process queued emails
    Process {
        /// Maximum number of emails to process
        #[arg(long)]
        limit: Option<u32>,
        
        /// Timeout in seconds for each email
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    
    /// Clear failed email jobs
    Clear {
        /// Clear all failed jobs
        #[arg(long)]
        failed: bool,
        
        /// Clear all completed jobs  
        #[arg(long)]
        completed: bool,
    },
}

#[derive(Subcommand)]
enum EmailTrackCommands {
    /// Show email tracking analytics
    Analytics {
        /// Date range (today, week, month)
        #[arg(long, default_value = "today")]
        range: String,
        
        /// Email campaign or template filter
        #[arg(long)]
        filter: Option<String>,
    },
    
    /// Show delivery statistics
    Stats {
        /// Group by (day, hour, provider, template)
        #[arg(long, default_value = "day")]
        group_by: String,
    },
}

#[derive(Subcommand)]
enum EmailTestCommands {
    /// Capture emails to filesystem for testing
    Capture {
        /// Enable filesystem email capture
        #[arg(long)]
        enable: bool,
        
        /// Disable filesystem email capture
        #[arg(long)]
        disable: bool,
        
        /// Custom capture directory
        #[arg(long)]
        dir: Option<String>,
    },
    
    /// List captured emails
    List {
        /// Show detailed email information
        #[arg(long, short)]
        detailed: bool,
        
        /// Filter by recipient email
        #[arg(long)]
        to: Option<String>,
        
        /// Filter by subject
        #[arg(long)]
        subject: Option<String>,
        
        /// Number of emails to show (latest first)
        #[arg(long, short, default_value = "10")]
        limit: usize,
    },
    
    /// Show specific captured email
    Show {
        /// Email ID or index
        email_id: String,
        
        /// Show raw email content
        #[arg(long)]
        raw: bool,
        
        /// Show only specific part (headers, text, html, attachments)
        #[arg(long)]
        part: Option<String>,
    },
    
    /// Clear captured emails
    #[command(group(
        clap::ArgGroup::new("clear_mode")
            .required(true)
            .args(&["all", "older_than"]),
    ))]
    Clear {
        /// Clear all captured emails
        #[arg(long)]
        all: bool,
        
        /// Clear emails older than N days
        #[arg(long)]
        older_than: Option<u32>,
    },
    
    /// Export captured emails
    Export {
        /// Export format (json, csv, mbox)
        #[arg(long, default_value = "json")]
        format: String,
        
        /// Output file path
        #[arg(long)]
        output: Option<String>,
        
        /// Include email bodies in export
        #[arg(long)]
        include_body: bool,
    },
}

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
        Commands::Generate { generate_command } => {
            match generate_command {
                Some(GenerateCommands::Middleware { name, debug, conditional, tests }) => {
                    generate::middleware(&name, debug, conditional, tests).await?;
                }
                None => {
                    generate::run().await?;
                }
            }
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
                OpenApiCommands::Generate { output, format } => {
                    openapi::generate(output, format).await?;
                }
                OpenApiCommands::Export { format, output } => {
                    openapi::export(format, output).await?;
                }
                OpenApiCommands::Serve { port } => {
                    openapi::serve(Some(port)).await?;
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
        Commands::Db { db_command } => {
            match db_command {
                DbCommands::Seed { env, force, verbose } => {
                    database::seed(env, force, verbose).await?;
                }
                DbCommands::Factory { factory_command } => {
                    match factory_command {
                        FactoryCommands::Status => {
                            database::factory_status().await?;
                        }
                        FactoryCommands::Test { count } => {
                            database::factory_test(count).await?;
                        }
                    }
                }
                DbCommands::Status => {
                    database::seed_status().await?;
                }
            }
        }
        Commands::Auth { auth_command } => {
            match auth_command {
                AuthCommands::Setup { provider, mfa, rbac } => {
                    auth::setup(provider, mfa, rbac).await?;
                }
                AuthCommands::GenerateKey { length } => {
                    auth::generate_key(length)?;
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
        Commands::Serve { port, host, hot_reload, watch, exclude, env } => {
            let args = serve::ServeArgs {
                port,
                host,
                hot_reload,
                watch,
                exclude,
                env,
            };
            serve::run(args).await?;
        }
        Commands::Queue { queue_command } => {
            match queue_command {
                QueueCommands::Work { 
                    queue, 
                    max_jobs, 
                    timeout, 
                    sleep, 
                    workers, 
                    stop_when_empty, 
                    verbose 
                } => {
                    let args = queue::QueueWorkArgs {
                        queue,
                        max_jobs,
                        timeout,
                        sleep,
                        workers,
                        stop_when_empty,
                        verbose,
                    };
                    queue::work(args).await?;
                }
                QueueCommands::Status { queue, detailed, refresh } => {
                    let args = queue::QueueStatusArgs {
                        queue,
                        detailed,
                        refresh,
                    };
                    queue::status(args).await?;
                }
                QueueCommands::Schedule { 
                    time, 
                    frequency, 
                    job, 
                    dry_run, 
                    force, 
                    verbose, 
                    daemon, 
                    check_interval 
                } => {
                    let args = queue::ScheduleRunArgs {
                        time,
                        frequency,
                        job,
                        dry_run,
                        force,
                        verbose,
                        daemon,
                        check_interval,
                    };
                    queue::schedule_run(args).await?;
                }
            }
        }
        Commands::Setup { non_interactive, verbose } => {
            let args = interactive_setup::InteractiveSetupArgs {
                non_interactive,
                verbose,
            };
            interactive_setup::run(args).await?;
        }
        Commands::Email { email_command } => {
            match email_command {
                EmailCommands::Send { to, subject, template, body, html, context } => {
                    let args = email::EmailSendArgs {
                        to,
                        subject,
                        template,
                        body,
                        html,
                        context,
                    };
                    email::send(args).await?;
                }
                EmailCommands::Template { template_command } => {
                    match template_command {
                        EmailTemplateCommands::List => {
                            email::template_list().await?;
                        }
                        EmailTemplateCommands::Validate { template } => {
                            email::template_validate(&template).await?;
                        }
                        EmailTemplateCommands::Render { template, context, format } => {
                            let args = email::EmailTemplateRenderArgs {
                                template,
                                context,
                                format,
                            };
                            email::template_render(args).await?;
                        }
                    }
                }
                EmailCommands::Provider { provider_command } => {
                    match provider_command {
                        EmailProviderCommands::Test { provider } => {
                            email::provider_test(provider).await?;
                        }
                        EmailProviderCommands::Configure { provider, interactive } => {
                            let args = email::EmailProviderConfigureArgs {
                                provider,
                                interactive,
                            };
                            email::provider_configure(args).await?;
                        }
                        EmailProviderCommands::Switch { provider } => {
                            email::provider_switch(&provider).await?;
                        }
                    }
                }
                EmailCommands::Queue { queue_command } => {
                    match queue_command {
                        EmailQueueCommands::Status { detailed } => {
                            email::queue_status(detailed).await?;
                        }
                        EmailQueueCommands::Process { limit, timeout } => {
                            let args = email::EmailQueueProcessArgs {
                                limit,
                                timeout,
                            };
                            email::queue_process(args).await?;
                        }
                        EmailQueueCommands::Clear { failed, completed } => {
                            let args = email::EmailQueueClearArgs {
                                failed,
                                completed,
                            };
                            email::queue_clear(args).await?;
                        }
                    }
                }
                EmailCommands::Track { track_command } => {
                    match track_command {
                        EmailTrackCommands::Analytics { range, filter } => {
                            let args = email::EmailTrackAnalyticsArgs {
                                range,
                                filter,
                            };
                            email::track_analytics(args).await?;
                        }
                        EmailTrackCommands::Stats { group_by } => {
                            email::track_stats(&group_by).await?;
                        }
                    }
                }
                EmailCommands::Setup { provider, non_interactive } => {
                    let args = email::EmailSetupArgs {
                        provider,
                        non_interactive,
                    };
                    email::setup(args).await?;
                }
                EmailCommands::Test { test_command } => {
                    match test_command {
                        EmailTestCommands::Capture { enable, disable, dir } => {
                            let args = email::EmailCaptureArgs {
                                enable,
                                disable,
                                dir,
                            };
                            email::test_capture(args).await?;
                        }
                        EmailTestCommands::List { detailed, to, subject, limit } => {
                            let args = email::EmailTestListArgs {
                                detailed,
                                to,
                                subject,
                                limit,
                            };
                            email::test_list(args).await?;
                        }
                        EmailTestCommands::Show { email_id, raw, part } => {
                            let args = email::EmailTestShowArgs {
                                email_id,
                                raw,
                                part,
                            };
                            email::test_show(args).await?;
                        }
                        EmailTestCommands::Clear { all, older_than } => {
                            let args = email::EmailTestClearArgs {
                                all,
                                older_than,
                            };
                            email::test_clear(args).await?;
                        }
                        EmailTestCommands::Export { format, output, include_body } => {
                            let args = email::EmailTestExportArgs {
                                format,
                                output,
                                include_body,
                            };
                            email::test_export(args).await?;
                        }
                    }
                }
            }
        }
        Commands::Version { version_command } => {
            match version_command {
                ApiVersionCommands::Create { version, description } => {
                    api_version::create_version(&version, description.clone()).await?;
                }
                ApiVersionCommands::Deprecate { version, message, sunset_date } => {
                    api_version::deprecate_version(&version, message.clone(), sunset_date.clone()).await?;
                }
                ApiVersionCommands::List => {
                    api_version::list_versions().await?;
                }
                ApiVersionCommands::Migrate { from, to } => {
                    api_version::generate_migration_guide(&from, &to).await?;
                }
                ApiVersionCommands::Validate => {
                    api_version::validate_versions().await?;
                }
            }
        }
    }
    
    Ok(())
}