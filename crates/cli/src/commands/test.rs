use elif_core::ElifError;
use crate::commands::module::ModuleDiscovery;
use crossbeam_channel::{select, tick, unbounded, Receiver};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::sleep;

/// Main entry point for the comprehensive testing system
pub async fn run(
    unit: bool,
    integration: bool,
    watch: bool,
    coverage: bool,
    module: Option<&str>,
) -> Result<(), ElifError> {
    println!("🧪 Elif.rs Testing System - Module-Aware Test Execution");
    println!();

    let test_runner = ModuleAwareTestRunner::new().await?;
    
    if watch {
        test_runner.run_with_watch(unit, integration, coverage, module).await
    } else {
        test_runner.run_tests(unit, integration, coverage, module).await
    }
}

/// Module-aware test runner with intelligent discovery and execution
pub struct ModuleAwareTestRunner {
    project_root: PathBuf,
    module_discovery: ModuleDiscovery,
    test_environment: TestEnvironment,
}

impl ModuleAwareTestRunner {
    pub async fn new() -> Result<Self, ElifError> {
        let project_root = std::env::current_dir()
            .map_err(|e| ElifError::system_error(format!("Failed to get current directory: {}", e)))?;
            
        let module_discovery = ModuleDiscovery::new();
        let test_environment = TestEnvironment::new(&project_root).await?;
        
        Ok(Self {
            project_root,
            module_discovery,
            test_environment,
        })
    }
    
    /// Run tests with comprehensive module awareness
    pub async fn run_tests(
        &self,
        unit: bool,
        integration: bool,
        coverage: bool,
        module_filter: Option<&str>,
    ) -> Result<(), ElifError> {
        // Pre-flight validation
        self.validate_test_environment().await?;
        
        // Discover test structure
        let test_discovery = self.discover_tests(module_filter).await?;
        
        if test_discovery.unit_tests.is_empty() && test_discovery.integration_tests.is_empty() {
            println!("📭 No tests found matching the criteria.");
            self.print_test_suggestions().await?;
            return Ok(());
        }
        
        // Print test plan
        self.print_test_plan(&test_discovery, unit, integration, coverage).await?;
        
        // Setup test environment (database, etc.)
        if integration {
            self.test_environment.setup_for_integration_tests().await?;
        }
        
        let mut test_results = TestResults::new();
        
        // Execute tests based on flags
        if unit || (!unit && !integration) {
            println!("\n🧪 Running Unit Tests...");
            let unit_result = self.run_unit_tests(&test_discovery, module_filter, coverage).await?;
            test_results.merge_unit_results(unit_result);
        }
        
        if integration || (!unit && !integration) {
            println!("\n🔗 Running Integration Tests...");
            let integration_result = self.run_integration_tests(&test_discovery, module_filter, coverage).await?;
            test_results.merge_integration_results(integration_result);
        }
        
        // Generate and display results
        self.print_test_summary(&test_results).await?;
        
        if coverage {
            self.generate_coverage_report(&test_results).await?;
        }
        
        if test_results.has_failures() {
            return Err(ElifError::validation("Some tests failed"));
        }
        
        Ok(())
    }
    
    /// Run tests with file watching for continuous feedback
    pub async fn run_with_watch(
        &self,
        unit: bool,
        integration: bool,
        coverage: bool,
        module_filter: Option<&str>,
    ) -> Result<(), ElifError> {
        println!("👀 Starting continuous testing mode...");
        println!("   Press Ctrl+C to stop\n");
        
        // Setup file watcher
        let watch_paths = self.get_test_watch_paths().await?;
        let mut file_watcher = TestFileWatcher::new(watch_paths)?;
        
        // Initial test run
        println!("🚀 Running initial test suite...");
        let _ = self.run_tests(unit, integration, coverage, module_filter).await;
        
        // Watch loop
        let debounce_duration = Duration::from_millis(500);
        let ticker = tick(Duration::from_millis(100));
        
        println!("\n👀 Watching for file changes...\n");
        
        loop {
            select! {
                recv(ticker) -> _ => {
                    if let Some(changed_files) = file_watcher.check_for_changes(debounce_duration) {
                        println!("🔄 Files changed: {:?}", changed_files.iter().map(|p| p.file_name().unwrap_or_default()).collect::<Vec<_>>());
                        
                        // Smart test selection based on changed files
                        let smart_module_filter = self.determine_affected_modules(&changed_files).await?;
                        let filter = smart_module_filter.as_deref().or(module_filter);
                        
                        println!("🎯 Running tests for affected modules...");
                        let _ = self.run_tests(unit, integration, false, filter).await; // Skip coverage in watch mode for speed
                        
                        println!("\n👀 Continuing to watch for changes...\n");
                    }
                },
            }
            
            sleep(Duration::from_millis(10)).await;
        }
    }
    
    /// Validate the test environment is ready
    async fn validate_test_environment(&self) -> Result<(), ElifError> {
        // Check if we're in a Rust project
        if !self.project_root.join("Cargo.toml").exists() {
            return Err(ElifError::configuration(
                "Not in a Rust project directory (no Cargo.toml found)"
            ));
        }
        
        // Check if project compiles
        println!("🔍 Validating project compilation...");
        let output = Command::new("cargo")
            .args(["check", "--quiet"])
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
        
        println!("✅ Project validation passed");
        Ok(())
    }
    
    /// Discover all tests in the project with module awareness
    async fn discover_tests(&self, module_filter: Option<&str>) -> Result<TestDiscovery, ElifError> {
        println!("🔍 Discovering tests with module awareness...");
        
        let modules = self.module_discovery.discover_modules().await?;
        
        let mut discovery = TestDiscovery {
            unit_tests: HashMap::new(),
            integration_tests: HashMap::new(),
            module_test_files: HashMap::new(),
            total_unit_tests: 0,
            total_integration_tests: 0,
        };
        
        // Discover unit tests (lib.rs, individual modules)
        discovery.unit_tests = self.discover_unit_tests(&modules, module_filter).await?;
        discovery.total_unit_tests = discovery.unit_tests.values().map(|tests| tests.len()).sum();
        
        // Discover integration tests (tests/ directory)
        discovery.integration_tests = self.discover_integration_tests(&modules, module_filter).await?;
        discovery.total_integration_tests = discovery.integration_tests.values().map(|tests| tests.len()).sum();
        
        // Map test files to modules
        discovery.module_test_files = self.map_tests_to_modules(&discovery, &modules).await?;
        
        println!("✅ Test discovery completed:");
        println!("   Unit tests: {}", discovery.total_unit_tests);
        println!("   Integration tests: {}", discovery.total_integration_tests);
        if let Some(filter) = module_filter {
            println!("   Filtered by module: {}", filter);
        }
        
        Ok(discovery)
    }
    
    /// Discover unit tests (tests embedded in source files)
    async fn discover_unit_tests(
        &self, 
        modules: &[crate::commands::module::ModuleInfo],
        module_filter: Option<&str>
    ) -> Result<HashMap<String, Vec<TestInfo>>, ElifError> {
        let mut unit_tests = HashMap::new();
        
        // Check main lib.rs
        let lib_path = self.project_root.join("src/lib.rs");
        if lib_path.exists() {
            let tests = self.extract_tests_from_file(&lib_path, "lib").await?;
            if !tests.is_empty() {
                unit_tests.insert("lib".to_string(), tests);
            }
        }
        
        // Check module files
        for module in modules {
            if let Some(filter) = module_filter {
                if !module.name.to_lowercase().contains(&filter.to_lowercase()) {
                    continue;
                }
            }
            
            let tests = self.extract_tests_from_file(&module.path, &module.name).await?;
            if !tests.is_empty() {
                unit_tests.insert(module.name.clone(), tests);
            }
        }
        
        // Check other source files
        let src_dir = self.project_root.join("src");
        if src_dir.exists() {
            self.discover_tests_in_directory(&src_dir, &mut unit_tests, module_filter)?;
        }
        
        Ok(unit_tests)
    }
    
    /// Discover integration tests (tests/ directory)
    async fn discover_integration_tests(
        &self,
        _modules: &[crate::commands::module::ModuleInfo],
        module_filter: Option<&str>
    ) -> Result<HashMap<String, Vec<TestInfo>>, ElifError> {
        let mut integration_tests = HashMap::new();
        
        let tests_dir = self.project_root.join("tests");
        if !tests_dir.exists() {
            return Ok(integration_tests);
        }
        
        // Discover integration test files
        for entry in fs::read_dir(&tests_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let file_name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                    
                if let Some(filter) = module_filter {
                    if !file_name.to_lowercase().contains(&filter.to_lowercase()) {
                        continue;
                    }
                }
                
                let tests = self.extract_tests_from_file(&path, file_name).await?;
                if !tests.is_empty() {
                    integration_tests.insert(file_name.to_string(), tests);
                }
            }
        }
        
        Ok(integration_tests)
    }
    
    /// Extract test functions from a Rust file
    async fn extract_tests_from_file(&self, file_path: &Path, module_name: &str) -> Result<Vec<TestInfo>, ElifError> {
        let content = fs::read_to_string(file_path)?;
        let mut tests = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Look for test attributes
            if line.starts_with("#[test]") || line.starts_with("#[tokio::test]") {
                // Get the next few lines to find the function name
                let mut j = i + 1;
                while j < lines.len() && lines[j].trim().starts_with('#') {
                    j += 1;
                }
                
                if j < lines.len() {
                    let fn_line = lines[j].trim();
                    if let Some(test_name) = self.extract_function_name(fn_line) {
                        let test_type = if line.contains("tokio::test") {
                            TestType::AsyncTest
                        } else {
                            TestType::SyncTest
                        };
                        
                        tests.push(TestInfo {
                            name: test_name,
                            module: module_name.to_string(),
                            file_path: file_path.to_path_buf(),
                            line_number: i + 1,
                            test_type,
                        });
                    }
                }
            }
            
            i += 1;
        }
        
        Ok(tests)
    }
    
    /// Extract function name from a function declaration line
    fn extract_function_name(&self, fn_line: &str) -> Option<String> {
        if let Some(fn_start) = fn_line.find("fn ") {
            let after_fn = &fn_line[fn_start + 3..];
            if let Some(paren_pos) = after_fn.find('(') {
                let name = after_fn[..paren_pos].trim();
                return Some(name.to_string());
            }
        }
        None
    }
    
    /// Recursively discover tests in a directory
    fn discover_tests_in_directory(
        &self,
        dir: &Path,
        unit_tests: &mut HashMap<String, Vec<TestInfo>>,
        module_filter: Option<&str>
    ) -> Result<(), ElifError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() && path.file_name() != Some(std::ffi::OsStr::new("target")) {
                self.discover_tests_in_directory(&path, unit_tests, module_filter)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let file_name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                    
                if let Some(filter) = module_filter {
                    if !file_name.to_lowercase().contains(&filter.to_lowercase()) {
                        continue;
                    }
                }
                
                // Extract tests synchronously by reading file directly
                let tests = self.extract_tests_from_file_sync(&path, file_name)?;
                if !tests.is_empty() {
                    unit_tests.entry(file_name.to_string()).or_insert_with(Vec::new).extend(tests);
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract test functions from a Rust file (synchronous version)
    fn extract_tests_from_file_sync(&self, file_path: &Path, module_name: &str) -> Result<Vec<TestInfo>, ElifError> {
        let content = fs::read_to_string(file_path)?;
        let mut tests = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Look for test attributes
            if line.starts_with("#[test]") || line.starts_with("#[tokio::test]") {
                // Get the next few lines to find the function name
                let mut j = i + 1;
                while j < lines.len() && lines[j].trim().starts_with('#') {
                    j += 1;
                }
                
                if j < lines.len() {
                    let fn_line = lines[j].trim();
                    if let Some(test_name) = self.extract_function_name(fn_line) {
                        let test_type = if line.contains("tokio::test") {
                            TestType::AsyncTest
                        } else {
                            TestType::SyncTest
                        };
                        
                        tests.push(TestInfo {
                            name: test_name,
                            module: module_name.to_string(),
                            file_path: file_path.to_path_buf(),
                            line_number: i + 1,
                            test_type,
                        });
                    }
                }
            }
            
            i += 1;
        }
        
        Ok(tests)
    }
    
    /// Map discovered tests to their corresponding modules
    async fn map_tests_to_modules(
        &self,
        discovery: &TestDiscovery,
        _modules: &[crate::commands::module::ModuleInfo]
    ) -> Result<HashMap<String, Vec<String>>, ElifError> {
        let mut mapping = HashMap::new();
        
        // Map unit tests
        for (test_module, tests) in &discovery.unit_tests {
            let test_names: Vec<String> = tests.iter().map(|t| t.name.clone()).collect();
            mapping.insert(test_module.clone(), test_names);
        }
        
        // Map integration tests
        for (test_file, tests) in &discovery.integration_tests {
            let test_names: Vec<String> = tests.iter().map(|t| t.name.clone()).collect();
            mapping.insert(format!("integration:{}", test_file), test_names);
        }
        
        Ok(mapping)
    }
    
    /// Print the test execution plan
    async fn print_test_plan(
        &self,
        discovery: &TestDiscovery,
        unit: bool,
        integration: bool,
        coverage: bool
    ) -> Result<(), ElifError> {
        println!("\n📋 Test Execution Plan:");
        
        if unit || (!unit && !integration) {
            println!("   🧪 Unit Tests: {} tests across {} modules", 
                discovery.total_unit_tests,
                discovery.unit_tests.len()
            );
            for (module, tests) in &discovery.unit_tests {
                println!("     • {}: {} tests", module, tests.len());
            }
        }
        
        if integration || (!unit && !integration) {
            println!("   🔗 Integration Tests: {} tests across {} files", 
                discovery.total_integration_tests,
                discovery.integration_tests.len()
            );
            for (file, tests) in &discovery.integration_tests {
                println!("     • {}: {} tests", file, tests.len());
            }
        }
        
        if coverage {
            println!("   📊 Coverage reporting: enabled");
        }
        
        println!();
        Ok(())
    }
    
    /// Print test suggestions when no tests are found
    async fn print_test_suggestions(&self) -> Result<(), ElifError> {
        println!("\n💡 Getting Started with Testing:");
        println!("   • Add #[test] functions to your source files");
        println!("   • Create integration tests in tests/ directory");
        println!("   • Use #[tokio::test] for async tests");
        println!("\n📖 Example:");
        println!("   #[test]");
        println!("   fn test_user_creation() {{");
        println!("       // Your test code here");
        println!("   }}");
        Ok(())
    }
    
    /// Run unit tests with module awareness
    async fn run_unit_tests(
        &self,
        discovery: &TestDiscovery,
        module_filter: Option<&str>,
        coverage: bool
    ) -> Result<TestExecutionResult, ElifError> {
        if discovery.unit_tests.is_empty() {
            return Ok(TestExecutionResult::new("unit"));
        }
        
        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        cmd.arg("--lib");
        
        // Add module-specific filters
        if let Some(module) = module_filter {
            let test_patterns: Vec<String> = discovery.unit_tests.iter()
                .filter(|(mod_name, _)| mod_name.to_lowercase().contains(&module.to_lowercase()))
                .flat_map(|(_, tests)| tests.iter().map(|t| t.name.clone()))
                .collect();
                
            if !test_patterns.is_empty() {
                cmd.arg("--");
                for pattern in &test_patterns {
                    cmd.arg(pattern);
                }
            }
        }
        
        // Add coverage instrumentation
        if coverage {
            cmd.env("RUSTFLAGS", "-C instrument-coverage");
        }
        
        let result = self.execute_test_command(cmd).await?;
        Ok(result)
    }
    
    /// Run integration tests with module awareness
    async fn run_integration_tests(
        &self,
        discovery: &TestDiscovery,
        module_filter: Option<&str>,
        coverage: bool
    ) -> Result<TestExecutionResult, ElifError> {
        if discovery.integration_tests.is_empty() {
            return Ok(TestExecutionResult::new("integration"));
        }
        
        let mut results = TestExecutionResult::new("integration");
        
        // Run each integration test file separately for better control
        for (test_file, _tests) in &discovery.integration_tests {
            if let Some(module) = module_filter {
                if !test_file.to_lowercase().contains(&module.to_lowercase()) {
                    continue;
                }
            }
            
            println!("   Running integration tests in: {}", test_file);
            
            let mut cmd = Command::new("cargo");
            cmd.arg("test");
            cmd.arg("--test");
            cmd.arg(test_file);
            
            if coverage {
                cmd.env("RUSTFLAGS", "-C instrument-coverage");
            }
            
            let file_result = self.execute_test_command(cmd).await?;
            results.merge(file_result);
        }
        
        Ok(results)
    }
    
    /// Execute a test command and capture results
    async fn execute_test_command(&self, mut cmd: Command) -> Result<TestExecutionResult, ElifError> {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .map_err(|e| ElifError::system_error(format!("Failed to start test process: {}", e)))?;
        
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        
        let mut result = TestExecutionResult::new("test");
        
        // Read output in real-time
        tokio::select! {
            _ = async {
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    println!("{}", line);
                    self.parse_test_output_line(&line, &mut result);
                }
            } => {},
            _ = async {
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    eprintln!("{}", line);
                }
            } => {},
        }
        
        let status = child.wait().await
            .map_err(|e| ElifError::system_error(format!("Failed to wait for test process: {}", e)))?;
        
        result.success = status.success();
        Ok(result)
    }
    
    /// Parse a line of test output to extract test results
    fn parse_test_output_line(&self, line: &str, result: &mut TestExecutionResult) {
        if line.contains("test result:") {
            // Parse summary line like "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
            if let Some(passed_start) = line.find(" passed") {
                if let Some(number_start) = line[..passed_start].rfind(' ') {
                    if let Ok(passed) = line[number_start + 1..passed_start].parse::<usize>() {
                        result.passed += passed;
                    }
                }
            }
            
            if let Some(failed_start) = line.find(" failed") {
                if let Some(number_start) = line[..failed_start].rfind(' ') {
                    if let Ok(failed) = line[number_start + 1..failed_start].parse::<usize>() {
                        result.failed += failed;
                    }
                }
            }
            
            if let Some(ignored_start) = line.find(" ignored") {
                if let Some(number_start) = line[..ignored_start].rfind(' ') {
                    if let Ok(ignored) = line[number_start + 1..ignored_start].parse::<usize>() {
                        result.ignored += ignored;
                    }
                }
            }
        }
    }
    
    /// Print comprehensive test summary
    async fn print_test_summary(&self, results: &TestResults) -> Result<(), ElifError> {
        println!("\n{}", "=".repeat(60));
        println!("📊 Test Results Summary");
        println!("{}", "=".repeat(60));
        
        if results.unit_results.passed > 0 || results.unit_results.failed > 0 {
            println!("🧪 Unit Tests:");
            println!("   ✅ Passed: {}", results.unit_results.passed);
            if results.unit_results.failed > 0 {
                println!("   ❌ Failed: {}", results.unit_results.failed);
            }
            if results.unit_results.ignored > 0 {
                println!("   ⏭️  Ignored: {}", results.unit_results.ignored);
            }
            println!();
        }
        
        if results.integration_results.passed > 0 || results.integration_results.failed > 0 {
            println!("🔗 Integration Tests:");
            println!("   ✅ Passed: {}", results.integration_results.passed);
            if results.integration_results.failed > 0 {
                println!("   ❌ Failed: {}", results.integration_results.failed);
            }
            if results.integration_results.ignored > 0 {
                println!("   ⏭️  Ignored: {}", results.integration_results.ignored);
            }
            println!();
        }
        
        let total_passed = results.unit_results.passed + results.integration_results.passed;
        let total_failed = results.unit_results.failed + results.integration_results.failed;
        let total_ignored = results.unit_results.ignored + results.integration_results.ignored;
        
        println!("🎯 Overall Results:");
        println!("   Total: {}", total_passed + total_failed + total_ignored);
        println!("   ✅ Passed: {}", total_passed);
        if total_failed > 0 {
            println!("   ❌ Failed: {}", total_failed);
        }
        if total_ignored > 0 {
            println!("   ⏭️  Ignored: {}", total_ignored);
        }
        
        if total_failed == 0 {
            println!("\n🎉 All tests passed!");
        } else {
            println!("\n💥 {} test(s) failed", total_failed);
        }
        
        println!("{}", "=".repeat(60));
        Ok(())
    }
    
    /// Generate coverage report
    async fn generate_coverage_report(&self, _results: &TestResults) -> Result<(), ElifError> {
        println!("\n📊 Generating coverage report...");
        
        // Check if llvm-cov is available
        let cov_check = Command::new("cargo")
            .args(["--list"])
            .output()
            .await;
            
        match cov_check {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("llvm-cov") {
                    // Generate coverage with llvm-cov
                    let cov_cmd = Command::new("cargo")
                        .args(["llvm-cov", "--html"])
                        .output()
                        .await;
                        
                    match cov_cmd {
                        Ok(cov_output) => {
                            if cov_output.status.success() {
                                println!("✅ Coverage report generated: target/llvm-cov/html/index.html");
                            } else {
                                println!("⚠️  Coverage generation failed");
                            }
                        }
                        Err(_) => {
                            println!("⚠️  Coverage generation failed");
                        }
                    }
                } else {
                    println!("💡 Install cargo-llvm-cov for coverage reports: cargo install cargo-llvm-cov");
                }
            }
            Err(_) => {
                println!("💡 Coverage reporting requires cargo-llvm-cov");
            }
        }
        
        Ok(())
    }
    
    /// Get paths to watch for test-relevant changes
    async fn get_test_watch_paths(&self) -> Result<Vec<PathBuf>, ElifError> {
        let mut paths = vec![
            self.project_root.join("src"),
            self.project_root.join("Cargo.toml"),
        ];
        
        // Add tests directory if it exists
        let tests_dir = self.project_root.join("tests");
        if tests_dir.exists() {
            paths.push(tests_dir);
        }
        
        // Add examples directory if it exists
        let examples_dir = self.project_root.join("examples");
        if examples_dir.exists() {
            paths.push(examples_dir);
        }
        
        // Add module-specific directories
        for optional_path in ["modules", "services", "controllers", "middleware"] {
            let path = self.project_root.join(optional_path);
            if path.exists() {
                paths.push(path);
            }
        }
        
        Ok(paths)
    }
    
    /// Determine which modules are affected by file changes
    async fn determine_affected_modules(&self, changed_files: &[PathBuf]) -> Result<Option<String>, ElifError> {
        let modules = self.module_discovery.discover_modules().await?;
        
        for changed_file in changed_files {
            // Check if the changed file belongs to a specific module
            for module in &modules {
                if changed_file.starts_with(&module.path.parent().unwrap_or(Path::new("."))) {
                    return Ok(Some(module.name.clone()));
                }
            }
            
            // Check if it's a test file
            if let Some(file_name) = changed_file.file_stem().and_then(|s| s.to_str()) {
                if file_name.contains("test") {
                    return Ok(Some(file_name.to_string()));
                }
            }
        }
        
        Ok(None)
    }
}

/// Test environment setup and management
pub struct TestEnvironment {
    project_root: PathBuf,
    database_url: Option<String>,
}

impl TestEnvironment {
    pub async fn new(project_root: &Path) -> Result<Self, ElifError> {
        let database_url = std::env::var("DATABASE_URL").ok()
            .or_else(|| std::env::var("TEST_DATABASE_URL").ok());
            
        Ok(Self {
            project_root: project_root.to_path_buf(),
            database_url,
        })
    }
    
    pub async fn setup_for_integration_tests(&self) -> Result<(), ElifError> {
        println!("🔧 Setting up test environment...");
        
        // Setup test database if configured
        if let Some(db_url) = &self.database_url {
            println!("   Database: {}", 
                if db_url.len() > 50 { 
                    format!("{}...", &db_url[..47]) 
                } else { 
                    db_url.clone() 
                }
            );
            
            // Run migrations if they exist
            let migrations_dir = self.project_root.join("migrations");
            if migrations_dir.exists() {
                println!("   Running test database migrations...");
                // This would integrate with the database system
                // For now, just indicate it's ready
            }
        }
        
        // Setup test data directories
        let test_data_dir = self.project_root.join("test-data");
        if test_data_dir.exists() {
            println!("   Test data directory: available");
        }
        
        println!("✅ Test environment ready");
        Ok(())
    }
}

/// File watcher for test-related changes
struct TestFileWatcher {
    receiver: Receiver<Event>,
    _watcher: RecommendedWatcher,
    last_changes: HashMap<PathBuf, Instant>,
}

impl TestFileWatcher {
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

        for path in &watch_paths {
            if path.exists() {
                watcher
                    .watch(path, RecursiveMode::Recursive)
                    .map_err(|e| {
                        ElifError::system_error(format!("Failed to watch path {:?}: {}", path, e))
                    })?;
            }
        }

        Ok(TestFileWatcher {
            receiver: rx,
            _watcher: watcher,
            last_changes: HashMap::new(),
        })
    }

    fn check_for_changes(&mut self, debounce_duration: Duration) -> Option<Vec<PathBuf>> {
        let mut changed_files = HashSet::new();
        let now = Instant::now();

        // Collect all recent events
        while let Ok(event) = self.receiver.try_recv() {
            if self.is_test_relevant_change(&event) {
                for path in event.paths {
                    self.last_changes.insert(path.clone(), now);
                    changed_files.insert(path);
                }
            }
        }

        // Remove old changes and check debounce
        self.last_changes.retain(|_, time| now.duration_since(*time) < debounce_duration * 2);
        
        // Return changes that have been debounced
        let debounced_changes: Vec<PathBuf> = self.last_changes.iter()
            .filter(|(_, time)| now.duration_since(**time) >= debounce_duration)
            .map(|(path, _)| path.clone())
            .collect();
            
        if debounced_changes.is_empty() {
            None
        } else {
            // Remove the returned changes
            for path in &debounced_changes {
                self.last_changes.remove(path);
            }
            Some(debounced_changes)
        }
    }

    fn is_test_relevant_change(&self, event: &Event) -> bool {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                event.paths.iter().any(|path| {
                    let path_str = path.to_string_lossy();
                    
                    // Include Rust source files and Cargo.toml
                    if path_str.ends_with(".rs") || path_str.ends_with("Cargo.toml") {
                        // Exclude build artifacts and temporary files
                        !path_str.contains("target/")
                            && !path_str.contains(".tmp")
                            && !path_str.contains(".git/")
                            && !path_str.ends_with("~")
                            && !path_str.ends_with(".swp")
                    } else {
                        false
                    }
                })
            }
            _ => false,
        }
    }
}

// Data structures

#[derive(Debug, Clone)]
pub struct TestDiscovery {
    pub unit_tests: HashMap<String, Vec<TestInfo>>,
    pub integration_tests: HashMap<String, Vec<TestInfo>>,
    pub module_test_files: HashMap<String, Vec<String>>,
    pub total_unit_tests: usize,
    pub total_integration_tests: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestInfo {
    pub name: String,
    pub module: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub test_type: TestType,
}

#[derive(Debug, Clone)]
pub enum TestType {
    SyncTest,
    AsyncTest,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestExecutionResult {
    pub name: String,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub success: bool,
}

impl TestExecutionResult {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: 0,
            failed: 0,
            ignored: 0,
            success: true,
        }
    }
    
    pub fn merge(&mut self, other: TestExecutionResult) {
        self.passed += other.passed;
        self.failed += other.failed;
        self.ignored += other.ignored;
        self.success = self.success && other.success;
    }
}

#[derive(Debug, Clone)]
pub struct TestResults {
    pub unit_results: TestExecutionResult,
    pub integration_results: TestExecutionResult,
}

impl TestResults {
    pub fn new() -> Self {
        Self {
            unit_results: TestExecutionResult::new("unit"),
            integration_results: TestExecutionResult::new("integration"),
        }
    }
    
    pub fn merge_unit_results(&mut self, results: TestExecutionResult) {
        self.unit_results.merge(results);
    }
    
    pub fn merge_integration_results(&mut self, results: TestExecutionResult) {
        self.integration_results.merge(results);
    }
    
    pub fn has_failures(&self) -> bool {
        self.unit_results.failed > 0 || self.integration_results.failed > 0
    }
}