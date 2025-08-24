use elif_core::ElifError;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};
use crossbeam_channel::{select, tick, unbounded, Receiver};
use tokio::time::sleep;

/// Development server with hot-reload functionality
pub async fn run(watch: Vec<PathBuf>, profile: bool, port: u16, host: &str, env: &str) -> Result<(), ElifError> {
    println!("🚀 Starting elif.rs development server...");
    println!("   Host: {}", host);
    println!("   Port: {}", port);
    println!("   Environment: {}", env);
    println!("   Profiling: {}", if profile { "enabled" } else { "disabled" });
    
    // Run pre-flight checks
    if let Err(e) = run_preflight_checks().await {
        eprintln!("❌ Pre-flight checks failed: {}", e);
        return Err(e);
    }
    
    println!("✅ Pre-flight checks passed");
    
    // Determine watch paths
    let watch_paths = if watch.is_empty() {
        get_default_watch_paths()
    } else {
        watch
    };
    
    println!("👀 Watching paths: {:?}", watch_paths);
    
    // Start file watcher
    let file_watcher = FileWatcher::new(watch_paths)?;
    
    // Start development server
    let dev_server = DevelopmentServer::new(
        host.to_string(),
        port,
        env.to_string(),
        profile,
    );
    
    dev_server.start_with_reload(file_watcher).await
}

/// Run pre-flight validation checks
async fn run_preflight_checks() -> Result<(), ElifError> {
    // Check if we're in an elif project
    if !Path::new("Cargo.toml").exists() {
        return Err(ElifError::configuration("Not in a Rust project directory (no Cargo.toml found)"));
    }
    
    // Check if basic project structure exists
    if !Path::new("src").exists() {
        return Err(ElifError::configuration("No src/ directory found"));
    }
    
    // Try to compile the project
    println!("🔍 Validating project compilation...");
    let output = Command::new("cargo")
        .args(["check", "--quiet"])
        .output()
        .map_err(|e| ElifError::system_error(format!("Failed to run cargo check: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ElifError::configuration(format!("Project compilation failed:\n{}", stderr)));
    }
    
    Ok(())
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

/// File watching system
struct FileWatcher {
    receiver: Receiver<Event>,
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    fn new(watch_paths: Vec<PathBuf>) -> Result<Self, ElifError> {
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
                    .map_err(|e| ElifError::system_error(format!("Failed to watch path {:?}: {}", path, e)))?;
                println!("🔍 Watching: {:?}", path);
            } else {
                println!("⚠️  Path not found, skipping: {:?}", path);
            }
        }
        
        Ok(FileWatcher {
            receiver: rx,
            _watcher: watcher,
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
                    !path_str.contains(".tmp") 
                        && !path_str.contains("target/") 
                        && !path_str.contains(".git/")
                        && !path_str.ends_with("~")
                        && !path_str.ends_with(".swp")
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
    profile: bool,
    current_process: Option<Child>,
}

impl DevelopmentServer {
    fn new(host: String, port: u16, env: String, profile: bool) -> Self {
        Self {
            host,
            port,
            env,
            profile,
            current_process: None,
        }
    }
    
    async fn start_with_reload(mut self, file_watcher: FileWatcher) -> Result<(), ElifError> {
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
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    async fn restart_server(&mut self) -> Result<(), ElifError> {
        // Kill existing process
        if let Some(mut child) = self.current_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        
        // Build the project first
        println!("🔨 Building project...");
        let build_result = Command::new("cargo")
            .args(["build", "--quiet"])
            .status()
            .map_err(|e| ElifError::system_error(format!("Failed to run cargo build: {}", e)))?;
        
        if !build_result.success() {
            return Err(ElifError::configuration("Build failed"));
        }
        
        // Start new server process
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "--quiet"]);
        
        // Set environment variables
        cmd.env("ELIF_ENV", &self.env);
        cmd.env("ELIF_HOST", &self.host);
        cmd.env("ELIF_PORT", self.port.to_string());
        
        if self.profile {
            cmd.env("ELIF_PROFILE", "true");
        }
        
        let child = cmd
            .spawn()
            .map_err(|e| ElifError::system_error(format!("Failed to start server: {}", e)))?;
        
        self.current_process = Some(child);
        
        // Give the server a moment to start
        sleep(Duration::from_millis(1000)).await;
        
        Ok(())
    }
}

impl Drop for DevelopmentServer {
    fn drop(&mut self) {
        if let Some(mut child) = self.current_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}