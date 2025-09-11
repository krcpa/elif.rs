use elif_core::ElifError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use crate::utils::{is_port_in_use, parse_redis_url, is_redis_accessible};

pub async fn run(fix_issues: bool, verbose: bool) -> Result<(), ElifError> {
    println!("🩺 Running elif.rs project diagnostics...");

    if verbose {
        println!("   Verbose mode: enabled");
    }

    if fix_issues {
        println!("   Auto-fix mode: enabled");
    }

    let mut doctor = Doctor::new(fix_issues, verbose);
    doctor.run_full_diagnosis().await
}

struct Doctor {
    fix_issues: bool,
    verbose: bool,
    issues: Vec<Issue>,
    fixes_applied: Vec<String>,
}

#[derive(Debug)]
struct Issue {
    category: IssueCategory,
    severity: IssueSeverity,
    description: String,
    auto_fixable: bool,
    fix_action: Option<FixAction>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum IssueCategory {
    ProjectStructure,
    CodeQuality,
    Configuration,
    Dependencies,
    Performance,
    FrameworkHealth,
}

#[derive(Debug, PartialEq)]
enum IssueSeverity {
    Critical,   // Blocks functionality
    Warning,    // Suboptimal but works
    Suggestion, // Best practice improvements
}

#[derive(Debug)]
#[allow(dead_code)]
enum FixAction {
    CreateFile {
        path: String,
        content: String,
    },
    RunCommand {
        command: String,
        args: Vec<String>,
    },
    UpdateFile {
        path: String,
        old: String,
        new: String,
    },
    CreateDirectory {
        path: String,
    },
}

impl Doctor {
    fn new(fix_issues: bool, verbose: bool) -> Self {
        Self {
            fix_issues,
            verbose,
            issues: Vec::new(),
            fixes_applied: Vec::new(),
        }
    }

    async fn run_full_diagnosis(&mut self) -> Result<(), ElifError> {
        println!("\n🔍 Diagnosing project health...");

        // Collect all issues
        self.diagnose_project_structure()?;
        self.diagnose_code_quality()?;
        self.diagnose_configuration()?;
        self.diagnose_dependencies()?;
        self.diagnose_performance()?;
        self.diagnose_framework_health().await?;

        // Report findings
        self.report_findings();

        // Apply fixes if requested
        if self.fix_issues && !self.issues.is_empty() {
            self.apply_fixes().await?;
        } else if !self.issues.is_empty() {
            self.suggest_fixes();
        }

        // Final summary
        self.print_final_summary();

        Ok(())
    }

    fn diagnose_project_structure(&mut self) -> Result<(), ElifError> {
        if self.verbose {
            println!("   📁 Diagnosing project structure...");
        }

        // Check essential files
        if !Path::new("README.md").exists() {
            self.issues.push(Issue {
                category: IssueCategory::ProjectStructure,
                severity: IssueSeverity::Warning,
                description: "Missing README.md file".to_string(),
                auto_fixable: true,
                fix_action: Some(FixAction::CreateFile {
                    path: "README.md".to_string(),
                    content: self.generate_readme_template(),
                }),
            });
        }

        if !Path::new(".gitignore").exists() {
            self.issues.push(Issue {
                category: IssueCategory::ProjectStructure,
                severity: IssueSeverity::Warning,
                description: "Missing .gitignore file".to_string(),
                auto_fixable: true,
                fix_action: Some(FixAction::CreateFile {
                    path: ".gitignore".to_string(),
                    content: self.generate_gitignore_template(),
                }),
            });
        }

        if !Path::new(".env.example").exists() && Path::new("src").join("main.rs").exists() {
            self.issues.push(Issue {
                category: IssueCategory::Configuration,
                severity: IssueSeverity::Suggestion,
                description: "Missing .env.example file for environment documentation".to_string(),
                auto_fixable: true,
                fix_action: Some(FixAction::CreateFile {
                    path: ".env.example".to_string(),
                    content: self.generate_env_example_template(),
                }),
            });
        }

        // Check directory structure
        let recommended_dirs = ["tests", "docs"];
        for dir in &recommended_dirs {
            if !Path::new(dir).exists() {
                self.issues.push(Issue {
                    category: IssueCategory::ProjectStructure,
                    severity: IssueSeverity::Suggestion,
                    description: format!("Missing {} directory", dir),
                    auto_fixable: true,
                    fix_action: Some(FixAction::CreateDirectory {
                        path: dir.to_string(),
                    }),
                });
            }
        }

        Ok(())
    }

    fn diagnose_code_quality(&mut self) -> Result<(), ElifError> {
        if self.verbose {
            println!("   ✨ Diagnosing code quality...");
        }

        // Check formatting
        let fmt_output = Command::new("cargo").args(["fmt", "--check"]).output();

        match fmt_output {
            Ok(output) if !output.status.success() => {
                self.issues.push(Issue {
                    category: IssueCategory::CodeQuality,
                    severity: IssueSeverity::Warning,
                    description: "Code formatting issues detected".to_string(),
                    auto_fixable: true,
                    fix_action: Some(FixAction::RunCommand {
                        command: "cargo".to_string(),
                        args: vec!["fmt".to_string()],
                    }),
                });
            }
            Err(_) => {
                if self.verbose {
                    println!("     ⚠️ rustfmt not available, skipping format check");
                }
            }
            _ => {}
        }

        // Check clippy warnings
        let clippy_output = Command::new("cargo").args(["clippy", "--quiet"]).output();

        match clippy_output {
            Ok(output) if !output.status.success() => {
                let fixable_clippy = Command::new("cargo")
                    .args(["clippy", "--fix", "--allow-dirty", "--allow-staged"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);

                self.issues.push(Issue {
                    category: IssueCategory::CodeQuality,
                    severity: IssueSeverity::Warning,
                    description: "Clippy warnings detected".to_string(),
                    auto_fixable: fixable_clippy,
                    fix_action: if fixable_clippy {
                        Some(FixAction::RunCommand {
                            command: "cargo".to_string(),
                            args: vec![
                                "clippy".to_string(),
                                "--fix".to_string(),
                                "--allow-dirty".to_string(),
                            ],
                        })
                    } else {
                        None
                    },
                });
            }
            Err(_) => {
                if self.verbose {
                    println!("     ⚠️ clippy not available, skipping lint check");
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn diagnose_configuration(&mut self) -> Result<(), ElifError> {
        if self.verbose {
            println!("   ⚙️ Diagnosing configuration...");
        }

        // Check Cargo.toml issues
        if let Ok(content) = fs::read_to_string("Cargo.toml") {
            if content.contains("unused manifest key: workspace.dev-dependencies")
                || content.contains("workspace.dev-dependencies")
            {
                self.issues.push(Issue {
                    category: IssueCategory::Configuration,
                    severity: IssueSeverity::Warning,
                    description: "Unused manifest key 'workspace.dev-dependencies' in Cargo.toml"
                        .to_string(),
                    auto_fixable: false, // Requires manual review
                    fix_action: None,
                });
            }
        }

        Ok(())
    }

    fn diagnose_dependencies(&mut self) -> Result<(), ElifError> {
        if self.verbose {
            println!("   📚 Diagnosing dependencies...");
        }

        // Check for future-incompatible dependencies
        let output = Command::new("cargo").args(["check"]).output();

        if let Ok(output) = output {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("future version of Rust") {
                self.issues.push(Issue {
                    category: IssueCategory::Dependencies,
                    severity: IssueSeverity::Critical,
                    description:
                        "Future-incompatible dependencies detected (e.g., sqlx-postgres v0.7.4)"
                            .to_string(),
                    auto_fixable: false, // Requires careful dependency updates
                    fix_action: None,
                });
            }
        }

        // Check if dependencies are up to date
        let outdated_output = Command::new("cargo").args(["outdated"]).output();

        match outdated_output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.trim().is_empty() && stdout.contains("->") {
                    self.issues.push(Issue {
                        category: IssueCategory::Dependencies,
                        severity: IssueSeverity::Suggestion,
                        description: "Outdated dependencies available for update".to_string(),
                        auto_fixable: false, // Requires review
                        fix_action: None,
                    });
                }
            }
            Err(_) => {
                if self.verbose {
                    println!("     ⚠️ cargo-outdated not available (install with: cargo install cargo-outdated)");
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn diagnose_performance(&mut self) -> Result<(), ElifError> {
        if self.verbose {
            println!("   🚀 Diagnosing performance optimizations...");
        }

        // Check if release profile optimizations are configured
        if let Ok(content) = fs::read_to_string("Cargo.toml") {
            if !content.contains("[profile.release]") {
                self.issues.push(Issue {
                    category: IssueCategory::Performance,
                    severity: IssueSeverity::Suggestion,
                    description: "No release profile optimizations configured".to_string(),
                    auto_fixable: true,
                    fix_action: Some(FixAction::UpdateFile {
                        path: "Cargo.toml".to_string(),
                        old: content.clone(),
                        new: format!("{}\n\n[profile.release]\nlto = true\ncodegen-units = 1\npanic = \"abort\"\n", content.trim()),
                    }),
                });
            }
        }

        Ok(())
    }

    async fn diagnose_framework_health(&mut self) -> Result<(), ElifError> {
        if self.verbose {
            println!("   🏥 Diagnosing framework health...");
        }

        // Check if essential elif.rs dependencies are present and their configuration
        if let Ok(content) = fs::read_to_string("Cargo.toml") {
            if let Ok(parsed_cargo) = toml::from_str::<toml::Value>(&content) {
                let has_elif_dependencies = self.check_elif_dependencies(&parsed_cargo);
                
                if !has_elif_dependencies {
                    self.issues.push(Issue {
                        category: IssueCategory::FrameworkHealth,
                        severity: IssueSeverity::Warning,
                        description: "No elif.rs framework dependencies detected".to_string(),
                        auto_fixable: false,
                        fix_action: None,
                    });
                }

                // Check for derive feature in elif-http
                if !self.is_elif_http_derive_enabled(&parsed_cargo) {
                    // Only suggest if elif-http is present
                    if self.has_elif_http_dependency(&parsed_cargo) {
                        self.issues.push(Issue {
                            category: IssueCategory::FrameworkHealth,
                            severity: IssueSeverity::Suggestion,
                            description: "Consider enabling 'derive' feature for elif-http to use declarative routing".to_string(),
                            auto_fixable: false,
                            fix_action: None,
                        });
                    }
                }
            } else {
                self.issues.push(Issue {
                    category: IssueCategory::FrameworkHealth,
                    severity: IssueSeverity::Warning,
                    description: "Failed to parse Cargo.toml - invalid TOML format".to_string(),
                    auto_fixable: false,
                    fix_action: None,
                });
            }
        }

        // Check for database configuration if using elif-orm
        if let Ok(content) = fs::read_to_string("Cargo.toml") {
            if let Ok(parsed_cargo) = toml::from_str::<toml::Value>(&content) {
                if self.has_elif_orm_dependency(&parsed_cargo) && std::env::var("DATABASE_URL").is_err() {
                    self.issues.push(Issue {
                        category: IssueCategory::FrameworkHealth,
                        severity: IssueSeverity::Warning,
                        description: "Using elif-orm but DATABASE_URL not configured".to_string(),
                        auto_fixable: false,
                        fix_action: None,
                    });
                }
            }
        }

        // Check for migrations directory if using database
        if std::env::var("DATABASE_URL").is_ok() && !Path::new("migrations").exists() {
            self.issues.push(Issue {
                category: IssueCategory::FrameworkHealth,
                severity: IssueSeverity::Suggestion,
                description: "Database configured but no migrations directory found".to_string(),
                auto_fixable: true,
                fix_action: Some(FixAction::CreateDirectory {
                    path: "migrations".to_string(),
                }),
            });
        }

        // Check for essential services if they should be running
        if std::env::var("DATABASE_URL").is_ok() && !is_port_in_use(5432) {
            self.issues.push(Issue {
                category: IssueCategory::FrameworkHealth,
                severity: IssueSeverity::Critical,
                description: "Database service is not running (PostgreSQL on port 5432)".to_string(),
                auto_fixable: false,
                fix_action: None,
            });
        }

        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            let (host, port) = parse_redis_url(&redis_url).unwrap_or(("127.0.0.1".to_string(), 6379));
            
            if !is_redis_accessible(&host, port).await {
                self.issues.push(Issue {
                    category: IssueCategory::FrameworkHealth,
                    severity: IssueSeverity::Warning,
                    description: format!("Redis service is not running on {}:{}", host, port),
                    auto_fixable: false,
                    fix_action: None,
                });
            }
        }

        // Check for application health
        let common_ports = [3000, 8000, 8080];
        let app_running = common_ports.iter().any(|port| is_port_in_use(*port));
        
        if !app_running {
            self.issues.push(Issue {
                category: IssueCategory::FrameworkHealth,
                severity: IssueSeverity::Suggestion,
                description: "Application is not currently running".to_string(),
                auto_fixable: false,
                fix_action: None,
            });
        }

        Ok(())
    }


    fn check_elif_dependencies(&self, parsed_cargo: &toml::Value) -> bool {
        if let Some(deps) = parsed_cargo.get("dependencies").and_then(|d| d.as_table()) {
            // Check if any elif.rs dependency is present
            deps.keys().any(|key| key.starts_with("elif-"))
        } else {
            false
        }
    }

    fn has_elif_http_dependency(&self, parsed_cargo: &toml::Value) -> bool {
        if let Some(deps) = parsed_cargo.get("dependencies").and_then(|d| d.as_table()) {
            deps.contains_key("elif-http")
        } else {
            false
        }
    }

    fn has_elif_orm_dependency(&self, parsed_cargo: &toml::Value) -> bool {
        if let Some(deps) = parsed_cargo.get("dependencies").and_then(|d| d.as_table()) {
            deps.contains_key("elif-orm")
        } else {
            false
        }
    }

    fn is_elif_http_derive_enabled(&self, parsed_cargo: &toml::Value) -> bool {
        if let Some(deps) = parsed_cargo.get("dependencies").and_then(|d| d.as_table()) {
            if let Some(elif_http_dep) = deps.get("elif-http") {
                match elif_http_dep {
                    // Handle table format: elif-http = { version = "...", features = ["derive"] }
                    toml::Value::Table(dep_table) => {
                        if let Some(features) = dep_table.get("features").and_then(|f| f.as_array()) {
                            features.iter().any(|feature| {
                                feature.as_str() == Some("derive")
                            })
                        } else {
                            false
                        }
                    },
                    // Handle string format with features: elif-http = { version = "...", features = ["derive"] }
                    // Note: string format like "0.8.0" doesn't support features
                    toml::Value::String(_) => {
                        // String format cannot have features, so derive is not enabled
                        false
                    },
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn report_findings(&self) {
        println!("\n📋 Diagnosis Results:");

        let mut by_category: HashMap<IssueCategory, Vec<&Issue>> = HashMap::new();
        for issue in &self.issues {
            by_category.entry(issue.category).or_default().push(issue);
        }

        let categories = [
            (IssueCategory::ProjectStructure, "📁 Project Structure"),
            (IssueCategory::CodeQuality, "✨ Code Quality"),
            (IssueCategory::Configuration, "⚙️ Configuration"),
            (IssueCategory::Dependencies, "📚 Dependencies"),
            (IssueCategory::Performance, "🚀 Performance"),
            (IssueCategory::FrameworkHealth, "🏥 Framework Health"),
        ];

        for (category, title) in categories {
            if let Some(issues) = by_category.get(&category) {
                if !issues.is_empty() {
                    println!("\n{}", title);
                    for issue in issues {
                        let severity_icon = match issue.severity {
                            IssueSeverity::Critical => "❌",
                            IssueSeverity::Warning => "⚠️",
                            IssueSeverity::Suggestion => "💡",
                        };
                        let fix_indicator = if issue.auto_fixable {
                            " [Auto-fixable]"
                        } else {
                            ""
                        };
                        println!("  {} {}{}", severity_icon, issue.description, fix_indicator);
                    }
                }
            }
        }

        if self.issues.is_empty() {
            println!("  🎉 No issues found! Your project is in excellent health.");
        } else {
            let fixable_count = self.issues.iter().filter(|i| i.auto_fixable).count();
            println!(
                "\n📊 Summary: {} issues found, {} auto-fixable",
                self.issues.len(),
                fixable_count
            );
        }
    }

    async fn apply_fixes(&mut self) -> Result<(), ElifError> {
        println!("\n🔧 Applying automatic fixes...");

        let fixable_issues: Vec<_> = self.issues.iter().filter(|i| i.auto_fixable).collect();

        if fixable_issues.is_empty() {
            println!("   No auto-fixable issues found.");
            return Ok(());
        }

        for issue in fixable_issues {
            if let Some(fix_action) = &issue.fix_action {
                if self.verbose {
                    println!("   Fixing: {}", issue.description);
                }

                match self.apply_fix_action(fix_action) {
                    Ok(fix_description) => {
                        self.fixes_applied.push(fix_description);
                        println!("   ✅ Fixed: {}", issue.description);
                    }
                    Err(e) => {
                        println!("   ❌ Failed to fix '{}': {}", issue.description, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn apply_fix_action(&self, action: &FixAction) -> Result<String, ElifError> {
        match action {
            FixAction::CreateFile { path, content } => {
                fs::write(path, content).map_err(|e| ElifError::Validation {
                    message: format!("Failed to create {}: {}", path, e),
                })?;
                Ok(format!("Created {}", path))
            }

            FixAction::RunCommand { command, args } => {
                let output = Command::new(command).args(args).output().map_err(|e| {
                    ElifError::Validation {
                        message: format!("Failed to run {}: {}", command, e),
                    }
                })?;

                if !output.status.success() {
                    return Err(ElifError::Validation {
                        message: format!("Command failed: {} {}", command, args.join(" ")),
                    });
                }

                Ok(format!("Ran {} {}", command, args.join(" ")))
            }

            FixAction::UpdateFile { path, old: _, new } => {
                // We could use `old` for validation, but for now just update the file
                fs::write(path, new).map_err(|e| ElifError::Validation {
                    message: format!("Failed to update {}: {}", path, e),
                })?;
                Ok(format!("Updated {}", path))
            }

            FixAction::CreateDirectory { path } => {
                fs::create_dir_all(path).map_err(|e| ElifError::Validation {
                    message: format!("Failed to create directory {}: {}", path, e),
                })?;
                Ok(format!("Created directory {}", path))
            }
        }
    }

    fn suggest_fixes(&self) {
        println!("\n💡 Suggested fixes (run with --fix-issues to apply automatically):");

        for issue in self.issues.iter().filter(|i| i.auto_fixable) {
            println!("  • {}", issue.description);
        }

        let manual_issues: Vec<_> = self.issues.iter().filter(|i| !i.auto_fixable).collect();
        if !manual_issues.is_empty() {
            println!("\n🔧 Manual fixes required:");
            for issue in manual_issues {
                println!("  • {}", issue.description);
                match issue.category {
                    IssueCategory::Dependencies => {
                        if issue.description.contains("future-incompatible") {
                            println!("    → Run: cargo update");
                            println!("    → Or update specific dependencies in Cargo.toml");
                        } else if issue.description.contains("outdated") {
                            println!("    → Run: cargo outdated");
                            println!("    → Update dependencies in Cargo.toml as needed");
                        }
                    }
                    IssueCategory::Configuration => {
                        if issue.description.contains("workspace.dev-dependencies") {
                            println!(
                                "    → Remove unused 'workspace.dev-dependencies' from Cargo.toml"
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn print_final_summary(&self) {
        println!("\n🏁 Doctor Summary:");

        if !self.fixes_applied.is_empty() {
            println!("   ✅ Applied {} fixes:", self.fixes_applied.len());
            for fix in &self.fixes_applied {
                println!("     • {}", fix);
            }
        }

        let remaining_issues = self.issues.len() - self.fixes_applied.len();
        if remaining_issues > 0 {
            println!(
                "   📋 {} issues remain (some require manual attention)",
                remaining_issues
            );
            println!("   💡 Run 'elifrs doctor --verbose' for detailed information");
        } else if self.issues.is_empty() {
            println!("   🎉 Your elif.rs project is in perfect health!");
        } else {
            println!("   ✨ All detected issues have been resolved!");
        }
    }

    // Template generators
    fn generate_readme_template(&self) -> String {
        let project_name = std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my-elif-app".to_string());

        format!(
            r#"# {}

A web application built with [elif.rs](https://github.com/krcpa/elif.rs) - Rust Made Simple.

## Quick Start

```bash
# Install dependencies
cargo build

# Run the application
cargo run

# Run tests
cargo test
```

## Features

- 🚀 Built with elif.rs framework
- ⚡ Fast and reliable Rust backend
- 🎯 Type-safe development experience

## Development

```bash
# Format code
cargo fmt

# Run lints
cargo clippy

# Check project health
elifrs check
```

## License

This project is licensed under the MIT License.
"#,
            project_name
        )
    }

    fn generate_gitignore_template(&self) -> String {
        r#"# Rust
/target/
Cargo.lock

# IDE
.vscode/
.idea/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Logs
*.log

# Environment
.env
.env.local

# Database
*.db
*.sqlite
*.sqlite3
"#
        .to_string()
    }

    fn generate_env_example_template(&self) -> String {
        r#"# Environment Configuration Template
# Copy this file to .env and configure your values

# Database
DATABASE_URL=postgresql://username:password@localhost/database_name

# Server
PORT=3000
HOST=127.0.0.1

# Application
APP_ENV=development
APP_SECRET=your-secret-key-here

# Logging
RUST_LOG=debug
"#
        .to_string()
    }
}
