use std::path::PathBuf;
use std::time::{Duration, Instant};
use clap::Args;
use elif_core::ElifError;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use crossbeam_channel::{select, tick, unbounded, Receiver};

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


/// Enhanced file watcher using notify crate
struct FileWatcher {
    receiver: Receiver<Event>,
    _watcher: RecommendedWatcher,
    exclude_patterns: Vec<String>,
}

impl FileWatcher {
    fn new(watch_paths: Vec<PathBuf>, exclude_patterns: Vec<String>) -> Result<Self, ElifError> {
        let (tx, rx) = unbounded();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        )
        .map_err(|e| ElifError::system_error(format!("Failed to create file watcher: {}", e)))?;

        // Watch all specified paths
        for path in watch_paths {
            if path.exists() {
                watcher
                    .watch(&path, RecursiveMode::Recursive)
                    .map_err(|e| {
                        ElifError::system_error(format!("Failed to watch path {:?}: {}", path, e))
                    })?;
                println!("🔍 Watching: {:?}", path);
            } else {
                println!("⚠️  Path not found, skipping: {:?}", path);
            }
        }

        Ok(FileWatcher {
            receiver: rx,
            _watcher: watcher,
            exclude_patterns,
        })
    }

    /// Check if there are file changes, with debouncing
    fn check_for_changes(&self, debounce_duration: Duration) -> bool {
        let mut last_event_time = None;
        let mut relevant_changes = false;

        // Process all pending events
        while let Ok(event) = self.receiver.try_recv() {
            if self.is_relevant_change(&event) {
                relevant_changes = true;
                last_event_time = Some(Instant::now());
            }
        }

        // If we have changes, wait for the debounce period
        if let Some(last_time) = last_event_time {
            if last_time.elapsed() < debounce_duration {
                return false; // Still in debounce period
            }
        }

        relevant_changes
    }

    fn is_relevant_change(&self, event: &Event) -> bool {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Filter out temporary files and build artifacts
                event.paths.iter().any(|path| {
                    let path_str = path.to_string_lossy();
                    
                    // Skip build artifacts and temp files
                    if path_str.contains(".tmp")
                        || path_str.contains("target/")
                        || path_str.contains(".git/")
                        || path_str.ends_with("~")
                        || path_str.ends_with(".swp")
                        || path_str.contains("__pycache__") 
                    {
                        return false;
                    }
                    
                    // Skip excluded patterns
                    if self.exclude_patterns.iter().any(|pattern| {
                        if pattern.contains('*') {
                            // Simple glob pattern matching
                            path_str.contains(&pattern.replace('*', ""))
                        } else {
                            path_str.contains(pattern)
                        }
                    }) {
                        return false;
                    }
                    
                    true
                })
            }
            _ => false,
        }
    }
}

/// Development server manager
struct DevelopmentServer {
    host: String,
    port: u16,
    env: String,
    current_process: Option<tokio::process::Child>,
}

impl DevelopmentServer {
    fn new(host: String, port: u16, env: String) -> Self {
        Self {
            host,
            port,
            env,
            current_process: None,
        }
    }

    async fn start_with_reload(&mut self, file_watcher: FileWatcher) -> Result<(), ElifError> {
        let debounce_duration = Duration::from_millis(500);
        let ticker = tick(Duration::from_millis(100));

        // Start the server initially
        self.restart_server().await?;

        println!("\n🎯 Development server started successfully!");
        println!("📡 Server running at http://{}:{}", self.host, self.port);
        println!("👀 Watching for file changes... (Ctrl+C to stop)\n");

        // Main event loop
        loop {
            select! {
                recv(ticker) -> _ => {
                    // Check for file changes every 100ms
                    if file_watcher.check_for_changes(debounce_duration) {
                        println!("🔄 Files changed, restarting server...");
                        if let Err(e) = self.restart_server().await {
                            eprintln!("❌ Failed to restart server: {}", e);
                            // Continue watching even if restart fails
                        } else {
                            println!("✅ Server restarted successfully\n");
                        }
                    }
                },
            }

            // Small sleep to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn restart_server(&mut self) -> Result<(), ElifError> {
        // Kill existing process
        if let Some(mut child) = self.current_process.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        // Build the project first
        println!("🔨 Building project...");
        let build_result = tokio::process::Command::new("cargo")
            .args(["build", "--quiet"])
            .status()
            .await
            .map_err(|e| ElifError::system_error(format!("Failed to run cargo build: {}", e)))?;

        if !build_result.success() {
            println!("❌ Build failed - waiting for file changes to retry");
            return Ok(()); // Don't error out, just wait for next change
        }

        // Start new server process
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.args(["run", "--quiet"]);

        // Set environment variables
        cmd.env("ELIF_ENV", &self.env);
        cmd.env("ELIF_HOST", &self.host);
        cmd.env("ELIF_PORT", self.port.to_string());

        let child = cmd
            .spawn()
            .map_err(|e| ElifError::system_error(format!("Failed to start server: {}", e)))?;

        self.current_process = Some(child);

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(1000)).await;

        Ok(())
    }

    async fn start_simple(&mut self) -> Result<(), ElifError> {
        // Check if Cargo.toml exists
        if !std::path::Path::new("Cargo.toml").exists() {
            return Err(ElifError::configuration("No Cargo.toml found. Make sure you're in an elif project directory."));
        }
        
        println!("📦 Building project...");
        
        // Build the project first
        let build_output = tokio::process::Command::new("cargo")
            .args(["build"])
            .output()
            .await
            .map_err(|e| ElifError::system_error(format!("Failed to run cargo build: {}", e)))?;
            
        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            return Err(ElifError::configuration(format!("Build failed:\n{}", stderr)));
        }
        
        println!("✅ Build completed successfully");
        println!("🌐 Server starting at http://{}:{}", self.host, self.port);
        
        // Start the server
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.args(["run"]);
        
        // Set environment variables
        cmd.env("ELIF_HOST", &self.host);
        cmd.env("ELIF_PORT", self.port.to_string());
        cmd.env("ELIF_ENV", &self.env);
        
        let mut child = cmd
            .spawn()
            .map_err(|e| ElifError::system_error(format!("Failed to start server: {}", e)))?;
        
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
                        return Err(ElifError::system_error(format!("Failed to wait for server: {}", e)));
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
}

impl Drop for DevelopmentServer {
    fn drop(&mut self) {
        if let Some(mut child) = self.current_process.take() {
            let _ = child.start_kill();
        }
    }
}

/// Get default paths to watch for changes
fn get_default_watch_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("src")];

    // Add optional directories if they exist
    for optional_path in ["config", "migrations", "templates", "static", "assets"] {
        let path = PathBuf::from(optional_path);
        if path.exists() {
            paths.push(path);
        }
    }

    // Add Cargo.toml for dependency changes
    paths.push(PathBuf::from("Cargo.toml"));

    paths
}

/// Create and run a serve command
pub async fn run(args: ServeArgs) -> Result<(), ElifError> {
    println!("🚀 Starting elif development server...");
    println!("📡 Host: {}", args.host);
    println!("🔌 Port: {}", args.port);
    println!("🌍 Environment: {}", args.env);
    
    let mut server = DevelopmentServer::new(args.host.clone(), args.port, args.env.clone());
    
    if args.hot_reload {
        println!("🔥 Hot reload: enabled");
        
        // Determine watch paths
        let watch_paths = if args.watch.is_empty() {
            get_default_watch_paths()
        } else {
            args.watch
        };
        
        println!("🔍 Setting up file watchers...");
        let file_watcher = FileWatcher::new(watch_paths, args.exclude.clone())?;
        
        if !args.exclude.is_empty() {
            println!("🚫 Excluding patterns: {:?}", args.exclude);
        }
        
        server.start_with_reload(file_watcher).await
    } else {
        println!("🔥 Hot reload: disabled");
        server.start_simple().await
    }
}

/// Standalone watch command for monitoring files and executing commands
pub async fn watch(
    paths: Vec<PathBuf>,
    exclude: Vec<String>,
    command: String,
    debounce: u64,
) -> Result<(), ElifError> {
    println!("👀 Starting file watcher...");
    
    let watch_paths = if paths.is_empty() {
        get_default_watch_paths()
    } else {
        paths
    };
    
    println!("🔍 Watching paths: {:?}", watch_paths);
    if !exclude.is_empty() {
        println!("🚫 Excluding patterns: {:?}", exclude);
    }
    println!("⚡ Command to execute: {}", command);
    
    let file_watcher = FileWatcher::new(watch_paths, exclude.clone())?;
    let debounce_duration = Duration::from_millis(debounce);
    let ticker = tick(Duration::from_millis(100));
    
    println!("✅ File watcher started. Press Ctrl+C to stop.\n");
    
    // Main watch loop
    loop {
        select! {
            recv(ticker) -> _ => {
                if file_watcher.check_for_changes(debounce_duration) {
                    println!("🔄 Files changed, executing command: {}", command);
                    
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    if !parts.is_empty() {
                        let mut cmd = tokio::process::Command::new(parts[0]);
                        if parts.len() > 1 {
                            cmd.args(&parts[1..]);
                        }
                        
                        match cmd.status().await {
                            Ok(status) => {
                                if status.success() {
                                    println!("✅ Command executed successfully");
                                } else {
                                    println!("❌ Command failed with exit code: {}", 
                                        status.code().unwrap_or(-1));
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to execute command: {}", e);
                            }
                        }
                    }
                    
                    println!("👀 Continuing to watch for changes...\n");
                }
            },
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}