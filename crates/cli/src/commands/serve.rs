use std::path::PathBuf;
use std::time::Duration;
use clap::Args;
use elif_core::ElifError;

/// Serve command arguments
#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    /// Port to bind the server to
    #[arg(long, short, default_value = "3000")]
    pub port: u16,
    
    /// Host to bind the server to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    
    /// Enable hot reload for development
    #[arg(long)]
    pub hot_reload: bool,
    
    /// Watch additional directories for changes
    #[arg(long)]
    pub watch: Vec<PathBuf>,
    
    /// Exclude patterns from watching
    #[arg(long)]
    pub exclude: Vec<String>,
    
    /// Environment to run in
    #[arg(long, short, default_value = "development")]
    pub env: String,
}


/// Internal implementation for serve functionality
struct ServeCommand {
    args: ServeArgs,
}

impl ServeCommand {
    fn new(args: ServeArgs) -> Self {
        Self { args }
    }
    
    async fn handle(&self) -> Result<(), ElifError> {
        println!("🚀 Starting elif development server...");
        println!("📡 Host: {}", self.args.host);
        println!("🔌 Port: {}", self.args.port);
        println!("🌍 Environment: {}", self.args.env);
        
        if self.args.hot_reload {
            println!("🔥 Hot reload: enabled");
            self.start_hot_reload_server().await
        } else {
            println!("🔥 Hot reload: disabled");
            self.start_server().await
        }
    }
    
    async fn start_server(&self) -> Result<(), ElifError> {
        // Check if Cargo.toml exists
        if !std::path::Path::new("Cargo.toml").exists() {
            return Err(ElifError::Codegen { message: "No Cargo.toml found. Make sure you're in an elif project directory.".to_string() });
        }
        
        println!("📦 Building project...");
        
        // Build the project first
        let build_output = tokio::process::Command::new("cargo")
            .args(["build"])
            .output()
            .await?;
            
        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            return Err(ElifError::Codegen { message: format!("Build failed:\n{}", stderr) });
        }
        
        println!("✅ Build completed successfully");
        println!("🌐 Server starting at http://{}:{}", self.args.host, self.args.port);
        
        // Start the server
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.args(["run"]);
        
        // Set environment variables
        cmd.env("ELIF_HOST", &self.args.host);
        cmd.env("ELIF_PORT", self.args.port.to_string());
        cmd.env("ELIF_ENV", &self.args.env);
        
        let mut child = cmd.spawn()?;
        
        // Handle Ctrl+C gracefully
        tokio::select! {
            result = child.wait() => {
                match result {
                    Ok(status) => {
                        if status.success() {
                            println!("✅ Server stopped gracefully");
                        } else {
                            println!("❌ Server exited with error");
                        }
                    }
                    Err(e) => {
                        return Err(ElifError::Codegen { message: format!("Failed to wait for server: {}", e) });
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n🛑 Received Ctrl+C, stopping server...");
                let _ = child.kill().await;
                println!("✅ Server stopped");
            }
        }
        
        Ok(())
    }
    
    async fn start_hot_reload_server(&self) -> Result<(), ElifError> {
        println!("🔍 Setting up file watchers...");
        
        // Default watch directories
        let mut watch_dirs = vec![PathBuf::from("src")];
        watch_dirs.extend(self.args.watch.clone());
        
        // Add common directories if they exist
        for dir in ["templates", "static", "migrations", "config"] {
            let path = PathBuf::from(dir);
            if path.exists() {
                watch_dirs.push(path);
            }
        }
        
        println!("👀 Watching directories: {:?}", watch_dirs);
        
        if !self.args.exclude.is_empty() {
            println!("🚫 Excluding patterns: {:?}", self.args.exclude);
        }
        
        // For now, we'll implement a basic polling-based file watcher
        // In production, you'd use notify crate for efficient file watching
        
        let mut last_modified = std::time::SystemTime::now();
        let mut server_process: Option<tokio::process::Child> = None;
        
        loop {
            let should_reload = self.check_file_changes(&watch_dirs, last_modified).await?;
            
            if should_reload {
                println!("📝 File changes detected, reloading...");
                
                // Kill existing server
                if let Some(mut process) = server_process.take() {
                    let _ = process.kill().await;
                    println!("🛑 Stopped previous server instance");
                }
                
                // Build project
                println!("📦 Rebuilding project...");
                let build_output = tokio::process::Command::new("cargo")
                    .args(["build"])
                    .output()
                    .await?;
                    
                if !build_output.status.success() {
                    let stderr = String::from_utf8_lossy(&build_output.stderr);
                    println!("❌ Build failed:\n{}", stderr);
                    println!("⏰ Waiting for changes...");
                    last_modified = std::time::SystemTime::now();
                    continue;
                }
                
                // Start new server
                println!("🚀 Starting server...");
                let mut cmd = tokio::process::Command::new("cargo");
                cmd.args(["run"]);
                cmd.env("ELIF_HOST", &self.args.host);
                cmd.env("ELIF_PORT", self.args.port.to_string());
                cmd.env("ELIF_ENV", &self.args.env);
                
                server_process = Some(cmd.spawn()?);
                last_modified = std::time::SystemTime::now();
                println!("✅ Server reloaded at http://{}:{}", self.args.host, self.args.port);
            }
            
            // Check for Ctrl+C
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(500)) => {
                    // Continue watching
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n🛑 Received Ctrl+C, stopping server...");
                    if let Some(mut process) = server_process.take() {
                        let _ = process.kill().await;
                    }
                    println!("✅ Hot reload server stopped");
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    async fn check_file_changes(
        &self,
        watch_dirs: &[PathBuf],
        last_check: std::time::SystemTime,
    ) -> Result<bool, ElifError> {
        for dir in watch_dirs {
            if self.dir_modified_since(dir, last_check).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    
    async fn dir_modified_since(
        &self,
        dir: &PathBuf,
        since: std::time::SystemTime,
    ) -> Result<bool, ElifError> {
        if !dir.exists() {
            return Ok(false);
        }
        
        let mut entries = tokio::fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            
            // Skip excluded patterns
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if self.args.exclude.iter().any(|pattern| {
                    // Simple pattern matching - in production use glob crate
                    pattern.contains('*') && file_name.contains(&pattern.replace('*', "")) ||
                    file_name == pattern
                }) {
                    continue;
                }
            }
            
            if metadata.is_file() {
                if let Ok(modified) = metadata.modified() {
                    if modified > since {
                        return Ok(true);
                    }
                }
            } else if metadata.is_dir() {
                if Box::pin(self.dir_modified_since(&path, since)).await? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
}

/// Create and run a serve command
pub async fn run(args: ServeArgs) -> Result<(), ElifError> {
    let command = ServeCommand::new(args);
    command.handle().await
}