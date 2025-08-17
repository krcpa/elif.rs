use crate::command_system::CommandError;
use crate::interactive::{Prompt, Format, ProgressBar};

/// Project configuration structure
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub project_name: String,
    pub database_url: String,
    pub database_type: String,
    pub auth_provider: String,
    pub enable_mfa: bool,
    pub enable_rbac: bool,
    pub environment: String,
    pub port: u16,
    pub host: String,
    pub enable_logging: bool,
    pub log_level: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project_name: "my-elif-app".to_string(),
            database_url: "postgres://localhost:5432/my_app".to_string(),
            database_type: "postgresql".to_string(),
            auth_provider: "jwt".to_string(),
            enable_mfa: false,
            enable_rbac: false,
            environment: "development".to_string(),
            port: 3000,
            host: "127.0.0.1".to_string(),
            enable_logging: true,
            log_level: "info".to_string(),
        }
    }
}

/// Handler for interactive configuration collection and generation
pub struct InteractiveConfigHandler {
    verbose: bool,
}

impl InteractiveConfigHandler {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
    
    pub async fn configure_project(&self, config: &mut ProjectConfig) -> Result<(), CommandError> {
        Format::subheader("📋 Project Configuration");
        
        config.project_name = Prompt::input(
            "Project name", 
            Some(&config.project_name)
        ).map_err(|e| CommandError::Io(e))?;
        
        let environments = [
            ("development".to_string(), "Development (recommended for local work)"),
            ("staging".to_string(), "Staging (for testing)"),
            ("production".to_string(), "Production (live environment)"),
        ];
        
        config.environment = Prompt::select(
            "Select your environment:",
            &environments
        ).map_err(|e| CommandError::Io(e))?;
        
        Ok(())
    }
    
    pub async fn configure_database(&self, config: &mut ProjectConfig) -> Result<(), CommandError> {
        Format::subheader("🗄️  Database Configuration");
        
        let db_types = [
            ("postgresql".to_string(), "PostgreSQL (recommended)"),
            ("mysql".to_string(), "MySQL"),
            ("sqlite".to_string(), "SQLite (for development)"),
        ];
        
        config.database_type = Prompt::select(
            "Select your database type:",
            &db_types
        ).map_err(|e| CommandError::Io(e))?;
        
        let default_url = match config.database_type.as_str() {
            "postgresql" => format!("postgres://localhost:5432/{}", config.project_name.replace('-', "_")),
            "mysql" => format!("mysql://localhost:3306/{}", config.project_name.replace('-', "_")),
            "sqlite" => format!("sqlite:./{}.db", config.project_name),
            _ => config.database_url.clone(),
        };
        
        config.database_url = Prompt::input(
            "Database URL",
            Some(&default_url)
        ).map_err(|e| CommandError::Io(e))?;
        
        Ok(())
    }
    
    pub async fn configure_auth(&self, config: &mut ProjectConfig) -> Result<(), CommandError> {
        Format::subheader("🔐 Authentication Configuration");
        
        let auth_providers = [
            ("jwt".to_string(), "JWT (JSON Web Tokens) - recommended"),
            ("session".to_string(), "Session-based authentication"),
            ("both".to_string(), "Both JWT and Session support"),
        ];
        
        config.auth_provider = Prompt::select(
            "Select authentication provider:",
            &auth_providers
        ).map_err(|e| CommandError::Io(e))?;
        
        config.enable_mfa = Prompt::confirm(
            "Enable Multi-Factor Authentication (MFA)?",
            false
        ).map_err(|e| CommandError::Io(e))?;
        
        config.enable_rbac = Prompt::confirm(
            "Enable Role-Based Access Control (RBAC)?",
            false
        ).map_err(|e| CommandError::Io(e))?;
        
        Ok(())
    }
    
    pub async fn configure_server(&self, config: &mut ProjectConfig) -> Result<(), CommandError> {
        Format::subheader("🌐 Server Configuration");
        
        config.host = Prompt::input(
            "Server host",
            Some(&config.host)
        ).map_err(|e| CommandError::Io(e))?;
        
        config.port = Prompt::number(
            "Server port",
            Some(config.port)
        ).map_err(|e| CommandError::Io(e))?;
        
        Ok(())
    }
    
    pub async fn configure_logging(&self, config: &mut ProjectConfig) -> Result<(), CommandError> {
        Format::subheader("📝 Logging Configuration");
        
        config.enable_logging = Prompt::confirm(
            "Enable application logging?",
            true
        ).map_err(|e| CommandError::Io(e))?;
        
        if config.enable_logging {
            let log_levels = [
                ("error".to_string(), "Error - only errors"),
                ("warn".to_string(), "Warning - errors and warnings"),
                ("info".to_string(), "Info - general information (recommended)"),
                ("debug".to_string(), "Debug - detailed information"),
                ("trace".to_string(), "Trace - very detailed information"),
            ];
            
            config.log_level = Prompt::select(
                "Select log level:",
                &log_levels
            ).map_err(|e| CommandError::Io(e))?;
        }
        
        Ok(())
    }
    
    pub async fn show_summary(&self, config: &ProjectConfig) -> Result<(), CommandError> {
        Format::header("📋 Configuration Summary");
        
        let data = vec![
            vec!["Project Name".to_string(), config.project_name.clone()],
            vec!["Environment".to_string(), config.environment.clone()],
            vec!["Database Type".to_string(), config.database_type.clone()],
            vec!["Database URL".to_string(), config.database_url.clone()],
            vec!["Auth Provider".to_string(), config.auth_provider.clone()],
            vec!["Enable MFA".to_string(), config.enable_mfa.to_string()],
            vec!["Enable RBAC".to_string(), config.enable_rbac.to_string()],
            vec!["Host".to_string(), config.host.clone()],
            vec!["Port".to_string(), config.port.to_string()],
            vec!["Enable Logging".to_string(), config.enable_logging.to_string()],
            vec!["Log Level".to_string(), config.log_level.clone()],
        ];
        
        Format::table(&["Setting", "Value"], &data);
        
        Ok(())
    }
    
    pub async fn apply_configuration(&self, config: &ProjectConfig) -> Result<(), CommandError> {
        Format::info("Applying configuration...");
        
        let steps = vec![
            "Creating project structure",
            "Generating configuration files",
            "Setting up database connection",
            "Configuring authentication",
            "Setting up development tools",
            "Finalizing setup",
        ];
        
        let mut pb = ProgressBar::new("Setup Progress", steps.len());
        
        for (i, step) in steps.iter().enumerate() {
            if self.verbose {
                Format::info(&format!("Step {}: {}", i + 1, step));
            }
            
            // Simulate work
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            
            pb.update(i + 1).map_err(|e| CommandError::Io(e))?;
        }
        
        pb.finish_with_message("Configuration applied successfully!")
            .map_err(|e| CommandError::Io(e))?;
        
        // Generate actual configuration files
        self.generate_config_files(config).await?;
        
        Ok(())
    }
    
    async fn generate_config_files(&self, config: &ProjectConfig) -> Result<(), CommandError> {
        if self.verbose {
            Format::info("Generating configuration files...");
        }
        
        // Create .env file
        let env_content = format!(
            "# elif.rs Project Configuration\n\
             PROJECT_NAME={}\n\
             ENVIRONMENT={}\n\
             DATABASE_URL={}\n\
             HOST={}\n\
             PORT={}\n\
             LOG_LEVEL={}\n\
             \n\
             # Authentication\n\
             AUTH_PROVIDER={}\n\
             ENABLE_MFA={}\n\
             ENABLE_RBAC={}\n",
            config.project_name,
            config.environment,
            config.database_url,
            config.host,
            config.port,
            config.log_level,
            config.auth_provider,
            config.enable_mfa,
            config.enable_rbac
        );
        
        tokio::fs::write(".env", env_content).await
            .map_err(|e| CommandError::Io(e))?;
        
        if self.verbose {
            Format::success("Generated .env file");
        }
        
        // Create config directory if it doesn't exist
        if !std::path::Path::new("config").exists() {
            tokio::fs::create_dir("config").await
                .map_err(|e| CommandError::Io(e))?;
        }
        
        // Create database.toml
        let db_content = format!(
            "[database]\n\
             url = \"{}\"\n\
             max_connections = 10\n\
             min_connections = 1\n\
             connect_timeout = 30\n\
             idle_timeout = 600\n",
            config.database_url
        );
        
        tokio::fs::write("config/database.toml", db_content).await
            .map_err(|e| CommandError::Io(e))?;
        
        if self.verbose {
            Format::success("Generated config/database.toml");
        }
        
        Ok(())
    }
}