use crossbeam_channel::{select, tick, unbounded, Receiver};
use elif_core::ElifError;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::process::{Child, Command};
use tokio::time::sleep;

/// Rust compiler flags to ignore warnings during development
const CARGO_IGNORE_WARNINGS_FLAG: &str = "-A warnings";

/// Enhanced development server with hot-reload functionality
pub async fn run(
    watch: Vec<PathBuf>,
    profile: bool,
    port: u16,
    host: &str,
    env: &str,
) -> Result<(), ElifError> {
    println!("🚀 Starting elif.rs unified development mode...");
    println!("   Host: {}", host);
    println!("   Port: {}", port);
    println!("   Environment: {}", env);
    println!("   Profiling: {}", if profile { "enabled" } else { "disabled" });
    println!("   Features: Hot reload + Module validation + Performance monitoring");

    // Run comprehensive pre-flight checks
    if let Err(e) = run_preflight_checks().await {
        eprintln!("❌ Pre-flight checks failed: {}", e);
        return Err(e);
    }

    // Run module system validation
    if let Err(e) = validate_module_system().await {
        eprintln!("⚠️  Module system validation warnings: {}", e);
        // Continue anyway - this is just a warning
    }

    println!("✅ Pre-flight checks passed");

    // Determine watch paths with intelligent defaults
    let watch_paths = if watch.is_empty() {
        get_comprehensive_watch_paths()
    } else {
        watch
    };

    println!("👀 Watching paths: {:?}", watch_paths);
    println!("🔍 Excluding: target/, .git/, *.tmp, *~, *.swp");

    // Start enhanced file watcher with built-in exclusions
    let file_watcher = FileWatcher::new(watch_paths)?;

    // Start comprehensive development server
    let dev_server = DevelopmentServer::new(host.to_string(), port, env.to_string(), profile);

    dev_server.start_with_reload(file_watcher).await
}

/// Run pre-flight validation checks
async fn run_preflight_checks() -> Result<(), ElifError> {
    // Check if we're in an elif project
    if !Path::new("Cargo.toml").exists() {
        return Err(ElifError::configuration(
            "Not in a Rust project directory (no Cargo.toml found)",
        ));
    }

    // Check if basic project structure exists
    if !Path::new("src").exists() {
        return Err(ElifError::configuration("No src/ directory found"));
    }

    // Try to compile the project (allow warnings)
    println!("🔍 Validating project compilation...");
    let output = Command::new("cargo")
        .args(["check", "--quiet"])
        .env("RUSTFLAGS", CARGO_IGNORE_WARNINGS_FLAG)
        .output()
        .await
        .map_err(|e| ElifError::system_error(format!("Failed to run cargo check: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ElifError::configuration(format!(
            "Project compilation failed:\n{}",
            stderr
        )));
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

/// Get comprehensive paths to watch including module-related files
fn get_comprehensive_watch_paths() -> Vec<PathBuf> {
    let mut paths = get_default_watch_paths();

    // Add module-specific paths if they exist
    for module_path in ["modules", "services", "controllers", "middleware"] {
        let path = PathBuf::from(module_path);
        if path.exists() {
            paths.push(path);
        }
    }

    // Add configuration files
    for config_file in ["elifrs.toml", ".env", ".env.local", "config.toml"] {
        let path = PathBuf::from(config_file);
        if path.exists() {
            paths.push(path);
        }
    }

    paths
}

/// Validate module system for development mode
async fn validate_module_system() -> Result<(), ElifError> {
    // Check if modules directory exists
    if !Path::new("modules").exists() && !Path::new("src/modules").exists() {
        return Err(ElifError::configuration("No modules directory found. Consider running 'elifrs make module' to create your first module."));
    }

    // Run basic module validation
    println!("🔍 Validating module system...");
    
    // Check for common module patterns
    let has_mod_rs = Path::new("src/modules/mod.rs").exists();
    let has_lib_rs = Path::new("src/lib.rs").exists();
    
    if !has_mod_rs && !has_lib_rs {
        println!("💡 Tip: Consider organizing your code with modules using 'elifrs module' commands");
    } else {
        println!("✅ Module system structure detected");
    }
    
    // Check for Phase 1 integration
    if has_mod_rs {
        println!("🔗 Phase 1 module system integration: Available");
        println!("   Use 'elifrs module list' to see your modules");
        println!("   Use 'elifrs module validate' to check module dependencies");
    }
    
    Ok(())
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
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        // Build the project first (allow warnings)
        println!("🔨 Building project...");
        let build_result = Command::new("cargo")
            .args(["build", "--quiet"])
            .env("RUSTFLAGS", CARGO_IGNORE_WARNINGS_FLAG)
            .status()
            .await
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
        cmd.env("RUSTFLAGS", CARGO_IGNORE_WARNINGS_FLAG);

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
            // Note: In Drop we have to use the synchronous kill() method
            // since Drop cannot be async. This is acceptable for cleanup.
            let _ = child.start_kill();
        }
    }
}
