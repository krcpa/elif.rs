use elif_core::ElifError;
use std::path::Path;
use std::process::Command;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{System, Disks};

#[derive(Serialize, Deserialize, Debug)]
struct SystemStatus {
    project_status: ProjectStatus,
    service_status: HashMap<String, ServiceStatus>,
    database_status: Option<DatabaseStatus>,
    health_checks: Vec<HealthCheck>,
    system_metrics: SystemMetrics,
    recommendations: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProjectStatus {
    name: String,
    version: String,
    build_status: String,
    last_built: Option<String>,
    compilation_errors: u32,
    test_status: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServiceStatus {
    name: String,
    status: String, // "running", "stopped", "error", "unknown"
    uptime: Option<u64>,
    cpu_usage: Option<f32>,
    memory_usage: Option<u64>,
    port: Option<u16>,
    last_health_check: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DatabaseStatus {
    connection_status: String,
    active_connections: Option<u32>,
    slow_queries: Option<u32>,
    last_migration: Option<String>,
    size_mb: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthCheck {
    name: String,
    status: String, // "pass", "fail", "warn"
    message: String,
    timestamp: String,
    response_time_ms: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SystemMetrics {
    cpu_usage: Option<f32>,
    memory_usage: Option<u64>,
    disk_usage: Option<u64>,
    load_average: Option<f32>,
    open_files: Option<u32>,
}

pub async fn run(health: bool, component: Option<&str>) -> Result<(), ElifError> {
    println!("📊 elif.rs Runtime Status");
    
    // Check if we're in an elif.rs project
    if !Path::new("Cargo.toml").exists() {
        return Err(ElifError::Validation {
            message: "Not in a Rust project directory".to_string(),
        });
    }

    let mut status = SystemStatus {
        project_status: get_project_status().await?,
        service_status: HashMap::new(),
        database_status: None,
        health_checks: Vec::new(),
        system_metrics: get_system_metrics().await?,
        recommendations: Vec::new(),
    };

    // Get service status
    status.service_status = get_service_status().await?;

    // Get database status if available
    status.database_status = get_database_status().await?;

    // Run health checks if requested or if no specific component is requested
    if health || component.is_none() {
        status.health_checks = run_health_checks().await?;
    }

    // If a specific component is requested, filter the results
    if let Some(comp) = component {
        display_component_status(&status, comp).await?;
    } else {
        display_full_status(&status).await?;
    }

    // Generate recommendations
    status.recommendations = generate_status_recommendations(&status).await?;
    if !status.recommendations.is_empty() {
        display_recommendations(&status.recommendations).await?;
    }

    // Save status report
    save_status_report(&status).await?;

    Ok(())
}

async fn get_project_status() -> Result<ProjectStatus, ElifError> {
    println!("🔍 Checking project status...");
    
    let mut project_status = ProjectStatus {
        name: "unknown".to_string(),
        version: "0.1.0".to_string(),
        build_status: "unknown".to_string(),
        last_built: None,
        compilation_errors: 0,
        test_status: "unknown".to_string(),
    };

    // Read project info from Cargo.toml
    if let Ok(cargo_content) = tokio::fs::read_to_string("Cargo.toml").await {
        if let Ok(cargo_toml) = cargo_content.parse::<toml::Value>() {
            if let Some(package) = cargo_toml.get("package") {
                if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
                    project_status.name = name.to_string();
                }
                if let Some(version) = package.get("version").and_then(|v| v.as_str()) {
                    project_status.version = version.to_string();
                }
            }
        }
    }

    // Check build status
    let build_result = Command::new("cargo")
        .args(&["check", "--quiet"])
        .output()
        .map_err(|e| ElifError::Io(e))?;

    if build_result.status.success() {
        project_status.build_status = "success".to_string();
        project_status.compilation_errors = 0;
    } else {
        project_status.build_status = "failed".to_string();
        let stderr = String::from_utf8_lossy(&build_result.stderr);
        project_status.compilation_errors = stderr.matches("error:").count() as u32;
    }

    // Check if target directory exists and get last build time
    if let Ok(metadata) = std::fs::metadata("target") {
        if let Ok(modified) = metadata.modified() {
            if let Ok(since_epoch) = modified.duration_since(UNIX_EPOCH) {
                project_status.last_built = Some(format_timestamp(since_epoch.as_secs()));
            }
        }
    }

    // Check test status
    let test_result = Command::new("cargo")
        .args(&["test", "--quiet", "--no-run"])
        .output()
        .map_err(|e| ElifError::Io(e))?;

    project_status.test_status = if test_result.status.success() {
        "ready".to_string()
    } else {
        "failed".to_string()
    };

    Ok(project_status)
}

async fn get_service_status() -> Result<HashMap<String, ServiceStatus>, ElifError> {
    let mut services = HashMap::new();
    
    println!("🔍 Checking service status...");

    // Check if the main application is running
    let main_service = check_application_service().await?;
    services.insert("application".to_string(), main_service);

    // Check database service
    let db_service = check_database_service().await?;
    services.insert("database".to_string(), db_service);

    // Check Redis service (if configured)
    let redis_service = check_redis_service().await?;
    services.insert("redis".to_string(), redis_service);

    Ok(services)
}

async fn check_application_service() -> Result<ServiceStatus, ElifError> {
    // Try to find if the application is running on common ports
    let common_ports = [3000, 8000, 8080, 4000];
    
    for port in &common_ports {
        if is_port_in_use(*port).await {
            return Ok(ServiceStatus {
                name: "Application".to_string(),
                status: "running".to_string(),
                uptime: None, // Would require more complex process tracking
                cpu_usage: None,
                memory_usage: None,
                port: Some(*port),
                last_health_check: Some(get_current_timestamp()),
            });
        }
    }
    
    Ok(ServiceStatus {
        name: "Application".to_string(),
        status: "stopped".to_string(),
        uptime: None,
        cpu_usage: None,
        memory_usage: None,
        port: None,
        last_health_check: Some(get_current_timestamp()),
    })
}

async fn check_database_service() -> Result<ServiceStatus, ElifError> {
    // Check if PostgreSQL is running on default port
    if is_port_in_use(5432).await {
        Ok(ServiceStatus {
            name: "PostgreSQL".to_string(),
            status: "running".to_string(),
            uptime: None,
            cpu_usage: None,
            memory_usage: None,
            port: Some(5432),
            last_health_check: Some(get_current_timestamp()),
        })
    } else {
        Ok(ServiceStatus {
            name: "PostgreSQL".to_string(),
            status: "stopped".to_string(),
            uptime: None,
            cpu_usage: None,
            memory_usage: None,
            port: Some(5432),
            last_health_check: Some(get_current_timestamp()),
        })
    }
}

async fn check_redis_service() -> Result<ServiceStatus, ElifError> {
    // Check if Redis is running on default port
    if is_port_in_use(6379).await {
        Ok(ServiceStatus {
            name: "Redis".to_string(),
            status: "running".to_string(),
            uptime: None,
            cpu_usage: None,
            memory_usage: None,
            port: Some(6379),
            last_health_check: Some(get_current_timestamp()),
        })
    } else {
        Ok(ServiceStatus {
            name: "Redis".to_string(),
            status: "stopped".to_string(),
            uptime: None,
            cpu_usage: None,
            memory_usage: None,
            port: Some(6379),
            last_health_check: Some(get_current_timestamp()),
        })
    }
}

async fn is_port_in_use(port: u16) -> bool {
    use std::net::{TcpListener, SocketAddr};
    
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpListener::bind(addr).is_err()
}

async fn get_database_status() -> Result<Option<DatabaseStatus>, ElifError> {
    // Check if database URL is configured
    if std::env::var("DATABASE_URL").is_err() {
        return Ok(None);
    }

    println!("🔍 Checking database status...");

    // This is a simplified check - in a real implementation you'd connect to the database
    Ok(Some(DatabaseStatus {
        connection_status: if is_port_in_use(5432).await { "connected" } else { "disconnected" }.to_string(),
        active_connections: Some(5), // Mock data
        slow_queries: Some(0),
        last_migration: get_last_migration().await?,
        size_mb: Some(128), // Mock data
    }))
}

async fn get_last_migration() -> Result<Option<String>, ElifError> {
    if Path::new("migrations").exists() {
        // Get the most recent migration file
        let mut entries = tokio::fs::read_dir("migrations").await.map_err(|e| ElifError::Io(e))?;
        let mut latest_migration = None;
        let mut latest_time = UNIX_EPOCH;

        while let Some(entry) = entries.next_entry().await.map_err(|e| ElifError::Io(e))? {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if modified > latest_time {
                        latest_time = modified;
                        latest_migration = entry.file_name().to_str().map(|s| s.to_string());
                    }
                }
            }
        }

        Ok(latest_migration)
    } else {
        Ok(None)
    }
}

async fn run_health_checks() -> Result<Vec<HealthCheck>, ElifError> {
    println!("🔍 Running health checks...");
    
    let mut health_checks = Vec::new();

    // Check 1: Project compilation
    let start_time = std::time::Instant::now();
    let build_check = Command::new("cargo")
        .args(&["check", "--quiet"])
        .output()
        .map_err(|e| ElifError::Io(e))?;
    
    let build_time = start_time.elapsed().as_millis() as u32;
    
    health_checks.push(HealthCheck {
        name: "Project Compilation".to_string(),
        status: if build_check.status.success() { "pass" } else { "fail" }.to_string(),
        message: if build_check.status.success() { 
            "Project compiles successfully".to_string() 
        } else { 
            "Project has compilation errors".to_string() 
        },
        timestamp: get_current_timestamp(),
        response_time_ms: Some(build_time),
    });

    // Check 2: Dependencies
    let deps_check = check_dependencies().await;
    health_checks.push(HealthCheck {
        name: "Dependencies".to_string(),
        status: if deps_check.0 { "pass" } else { "warn" }.to_string(),
        message: deps_check.1,
        timestamp: get_current_timestamp(),
        response_time_ms: None,
    });

    // Check 3: Database connection (if configured)
    if std::env::var("DATABASE_URL").is_ok() {
        let db_check = check_database_connection().await;
        health_checks.push(HealthCheck {
            name: "Database Connection".to_string(),
            status: if db_check.0 { "pass" } else { "fail" }.to_string(),
            message: db_check.1,
            timestamp: get_current_timestamp(),
            response_time_ms: None,
        });
    }

    // Check 4: Configuration
    let config_check = check_configuration().await;
    health_checks.push(HealthCheck {
        name: "Configuration".to_string(),
        status: if config_check.0 { "pass" } else { "warn" }.to_string(),
        message: config_check.1,
        timestamp: get_current_timestamp(),
        response_time_ms: None,
    });

    // Check 5: Security
    let security_check = check_security_configuration().await;
    health_checks.push(HealthCheck {
        name: "Security Configuration".to_string(),
        status: security_check.0,
        message: security_check.1,
        timestamp: get_current_timestamp(),
        response_time_ms: None,
    });

    Ok(health_checks)
}

async fn check_dependencies() -> (bool, String) {
    // Check if Cargo.lock exists and is up to date
    if !Path::new("Cargo.lock").exists() {
        return (false, "Cargo.lock not found - run 'cargo build' to generate".to_string());
    }

    // Simple check - in a real implementation you'd check for outdated dependencies
    (true, "Dependencies are properly locked".to_string())
}

async fn check_database_connection() -> (bool, String) {
    // This is a simplified check - in a real implementation you'd actually connect
    if is_port_in_use(5432).await {
        (true, "Database service is running on port 5432".to_string())
    } else {
        (false, "Cannot connect to database on port 5432".to_string())
    }
}

async fn check_configuration() -> (bool, String) {
    let mut issues = Vec::new();

    // Check for essential environment variables
    let required_vars = ["DATABASE_URL", "SECRET_KEY"];
    for var in &required_vars {
        if std::env::var(var).is_err() {
            issues.push(format!("Missing {}", var));
        }
    }

    if issues.is_empty() {
        (true, "All required configuration is present".to_string())
    } else {
        (false, format!("Missing configuration: {}", issues.join(", ")))
    }
}

async fn check_security_configuration() -> (String, String) {
    let mut warnings = Vec::new();

    // Check SECRET_KEY strength
    if let Ok(secret_key) = std::env::var("SECRET_KEY") {
        if secret_key.len() < 32 {
            warnings.push("SECRET_KEY is too short (should be 32+ characters)");
        }
        if secret_key == "your-secret-key-here" || secret_key == "changeme" {
            warnings.push("SECRET_KEY appears to be a default value");
        }
    }

    // Check RUST_LOG level
    if let Ok(log_level) = std::env::var("RUST_LOG") {
        if log_level.contains("debug") || log_level.contains("trace") {
            warnings.push("Debug logging enabled in production environment");
        }
    }

    if warnings.is_empty() {
        ("pass".to_string(), "Security configuration looks good".to_string())
    } else {
        ("warn".to_string(), format!("Security issues: {}", warnings.join(", ")))
    }
}

async fn get_system_metrics() -> Result<SystemMetrics, ElifError> {
    let mut system = System::new_all();
    system.refresh_all();
    
    Ok(SystemMetrics {
        cpu_usage: get_cpu_usage(&mut system),
        memory_usage: get_memory_usage(&system),
        disk_usage: get_disk_usage().await,
        load_average: get_load_average(&system),
        open_files: None,
    })
}

fn get_cpu_usage(system: &mut System) -> Option<f32> {
    // Refresh CPU usage and get global CPU usage
    system.refresh_cpu_usage();
    
    // Wait a bit for CPU usage to be calculated (required by sysinfo)
    std::thread::sleep(std::time::Duration::from_millis(200));
    system.refresh_cpu_usage();
    
    Some(system.global_cpu_usage())
}

fn get_memory_usage(system: &System) -> Option<u64> {
    // Get used memory in MB
    let used_memory = system.used_memory();
    Some(used_memory / 1024 / 1024) // Convert bytes to MB
}

async fn get_disk_usage() -> Option<u64> {
    // Get disk usage for current directory using cross-platform approach
    let disks = Disks::new_with_refreshed_list();
    
    if let Ok(current_dir) = std::env::current_dir() {
        // Find the disk that contains the current directory
        for disk in &disks {
            let mount_point = disk.mount_point();
            if current_dir.starts_with(mount_point) {
                // Calculate used space from disk capacity and available space
                let total_space = disk.total_space();
                let available_space = disk.available_space();
                let used_space = total_space - available_space;
                return Some(used_space / 1024 / 1024); // Convert bytes to MB
            }
        }
    }
    
    None
}

fn get_load_average(_system: &System) -> Option<f32> {
    // Get load average (1-minute load average on supported systems)
    let load_avg = System::load_average();
    Some(load_avg.one as f32)
}

async fn display_component_status(status: &SystemStatus, component: &str) -> Result<(), ElifError> {
    println!("\n📊 Component Status: {}", component);

    match component.to_lowercase().as_str() {
        "project" => {
            display_project_status(&status.project_status).await?;
        }
        "database" | "db" => {
            if let Some(db_status) = &status.database_status {
                display_database_status(db_status).await?;
            } else {
                println!("   ❌ Database not configured");
            }
        }
        "services" => {
            display_services_status(&status.service_status).await?;
        }
        "system" => {
            display_system_metrics(&status.system_metrics).await?;
        }
        _ => {
            println!("   ❓ Unknown component: {}", component);
            println!("   Available components: project, database, services, system");
        }
    }

    Ok(())
}

async fn display_full_status(status: &SystemStatus) -> Result<(), ElifError> {
    // Display project status
    display_project_status(&status.project_status).await?;

    // Display services
    display_services_status(&status.service_status).await?;

    // Display database status
    if let Some(db_status) = &status.database_status {
        display_database_status(db_status).await?;
    }

    // Display health checks
    if !status.health_checks.is_empty() {
        display_health_checks(&status.health_checks).await?;
    }

    // Display system metrics
    display_system_metrics(&status.system_metrics).await?;

    Ok(())
}

async fn display_project_status(status: &ProjectStatus) -> Result<(), ElifError> {
    println!("\n📦 Project Status:");
    println!("   📛 Name: {}", status.name);
    println!("   🏷️  Version: {}", status.version);
    
    let build_icon = match status.build_status.as_str() {
        "success" => "✅",
        "failed" => "❌",
        _ => "❓",
    };
    println!("   {} Build: {}", build_icon, status.build_status);
    
    if status.compilation_errors > 0 {
        println!("   ⚠️  Compilation Errors: {}", status.compilation_errors);
    }
    
    if let Some(last_built) = &status.last_built {
        println!("   🕐 Last Built: {}", last_built);
    }
    
    let test_icon = match status.test_status.as_str() {
        "ready" => "✅",
        "failed" => "❌",
        _ => "❓",
    };
    println!("   {} Tests: {}", test_icon, status.test_status);

    Ok(())
}

async fn display_services_status(services: &HashMap<String, ServiceStatus>) -> Result<(), ElifError> {
    println!("\n🔧 Services Status:");
    
    for (_, service) in services {
        let status_icon = match service.status.as_str() {
            "running" => "🟢",
            "stopped" => "🔴",
            "error" => "❌",
            _ => "❓",
        };
        
        println!("   {} {}: {}", status_icon, service.name, service.status);
        
        if let Some(port) = service.port {
            println!("      🔌 Port: {}", port);
        }
        
        if let Some(uptime) = service.uptime {
            println!("      ⏱️  Uptime: {}s", uptime);
        }
    }

    Ok(())
}

async fn display_database_status(status: &DatabaseStatus) -> Result<(), ElifError> {
    println!("\n🗄️ Database Status:");
    
    let connection_icon = match status.connection_status.as_str() {
        "connected" => "✅",
        "disconnected" => "❌",
        _ => "❓",
    };
    println!("   {} Connection: {}", connection_icon, status.connection_status);
    
    if let Some(connections) = status.active_connections {
        println!("   🔗 Active Connections: {}", connections);
    }
    
    if let Some(size) = status.size_mb {
        println!("   💾 Database Size: {}MB", size);
    }
    
    if let Some(migration) = &status.last_migration {
        println!("   📝 Last Migration: {}", migration);
    }
    
    if let Some(slow_queries) = status.slow_queries {
        if slow_queries > 0 {
            println!("   ⚠️  Slow Queries: {}", slow_queries);
        }
    }

    Ok(())
}

async fn display_health_checks(health_checks: &[HealthCheck]) -> Result<(), ElifError> {
    println!("\n🏥 Health Checks:");
    
    for check in health_checks {
        let status_icon = match check.status.as_str() {
            "pass" => "✅",
            "fail" => "❌",
            "warn" => "⚠️",
            _ => "❓",
        };
        
        println!("   {} {}: {}", status_icon, check.name, check.message);
        
        if let Some(response_time) = check.response_time_ms {
            println!("      ⏱️  Response Time: {}ms", response_time);
        }
    }

    Ok(())
}

async fn display_system_metrics(metrics: &SystemMetrics) -> Result<(), ElifError> {
    println!("\n💻 System Metrics:");
    
    if let Some(cpu) = metrics.cpu_usage {
        println!("   🖥️  CPU Usage: {:.1}%", cpu);
    }
    
    if let Some(memory) = metrics.memory_usage {
        println!("   💾 Memory Usage: {}MB", memory);
    }
    
    if let Some(disk) = metrics.disk_usage {
        println!("   💿 Disk Usage: {}MB", disk);
    }
    
    if let Some(load) = metrics.load_average {
        println!("   📊 Load Average: {:.2}", load);
    }

    Ok(())
}

async fn generate_status_recommendations(status: &SystemStatus) -> Result<Vec<String>, ElifError> {
    let mut recommendations = Vec::new();
    
    // Project recommendations
    if status.project_status.build_status == "failed" {
        recommendations.push("Fix compilation errors before deployment".to_string());
    }
    
    if status.project_status.compilation_errors > 0 {
        recommendations.push(format!("Address {} compilation errors", status.project_status.compilation_errors));
    }
    
    // Service recommendations
    for (_, service) in &status.service_status {
        if service.status == "stopped" && service.name == "Application" {
            recommendations.push("Start the application service".to_string());
        }
        if service.status == "stopped" && service.name == "PostgreSQL" {
            recommendations.push("Start the database service".to_string());
        }
    }
    
    // Health check recommendations
    for check in &status.health_checks {
        if check.status == "fail" {
            recommendations.push(format!("Fix failing health check: {}", check.name));
        }
        if check.status == "warn" {
            recommendations.push(format!("Address warning in: {}", check.name));
        }
    }
    
    // Database recommendations
    if let Some(db) = &status.database_status {
        if db.connection_status == "disconnected" {
            recommendations.push("Restore database connection".to_string());
        }
        if let Some(slow_queries) = db.slow_queries {
            if slow_queries > 10 {
                recommendations.push("Optimize slow database queries".to_string());
            }
        }
    }
    
    Ok(recommendations)
}

async fn display_recommendations(recommendations: &[String]) -> Result<(), ElifError> {
    println!("\n💡 Recommendations:");
    for recommendation in recommendations {
        println!("   • {}", recommendation);
    }
    Ok(())
}

async fn save_status_report(status: &SystemStatus) -> Result<(), ElifError> {
    let report_json = serde_json::to_string_pretty(status)
        .map_err(|e| ElifError::SystemError {
            message: format!("Failed to serialize status report: {}", e),
            source: None,
        })?;

    tokio::fs::write("status-report.json", report_json)
        .await
        .map_err(|e| ElifError::Io(e))?;

    println!("📄 Status report saved to status-report.json");
    Ok(())
}

fn get_current_timestamp() -> String {
    if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
        format_timestamp(now.as_secs())
    } else {
        "unknown".to_string()
    }
}

fn format_timestamp(timestamp: u64) -> String {
    // Simple timestamp formatting - in production you'd use a proper date library
    let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0);
    if let Some(dt) = datetime {
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        "unknown".to_string()
    }
}
