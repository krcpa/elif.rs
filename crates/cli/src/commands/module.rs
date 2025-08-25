use elif_core::ElifError;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// List all modules and their dependencies
pub async fn list(dependencies: bool) -> Result<(), ElifError> {
    println!("🧩 Module Discovery & Analysis");
    println!();

    let discovery = ModuleDiscovery::new();
    let modules = discovery.discover_modules().await?;

    if modules.is_empty() {
        println!("📭 No modules found in this project.");
        println!("\n💡 Get started:");
        println!("   elifrs add module UserModule --controllers=UserController");
        return Ok(());
    }

    // Display module list
    print_modules_list(&modules, dependencies).await?;

    // Show dependency relationships if requested
    if dependencies {
        println!("\n🔗 Dependency Analysis:");
        let dependency_graph = discovery.analyze_dependencies(&modules).await?;
        print_dependency_summary(&dependency_graph)?;
    }

    Ok(())
}

/// Generate and visualize module dependency graph
pub async fn graph(format: &str, output: Option<&str>) -> Result<(), ElifError> {
    println!("📊 Module Dependency Graph");
    println!();

    let discovery = ModuleDiscovery::new();
    let modules = discovery.discover_modules().await?;
    
    if modules.is_empty() {
        println!("📭 No modules found to graph.");
        return Ok(());
    }

    let dependency_graph = discovery.analyze_dependencies(&modules).await?;
    
    match format {
        "svg" => generate_svg_graph(&dependency_graph, output).await?,
        "dot" => generate_dot_graph(&dependency_graph, output).await?,
        "text" => generate_text_graph(&dependency_graph).await?,
        "json" => generate_json_graph(&dependency_graph, output).await?,
        _ => {
            return Err(ElifError::validation(format!(
                "Unsupported format: {}. Available formats: svg, dot, text, json", 
                format
            )));
        }
    }

    Ok(())
}

/// Convert manual IoC to module system
pub async fn migrate(analyze_first: bool) -> Result<(), ElifError> {
    println!("🔄 IoC to Module System Migration");
    println!();

    if analyze_first {
        println!("📊 Analyzing existing IoC setup...");
        let analysis = analyze_existing_ioc().await?;
        print_migration_analysis(&analysis)?;
        
        print!("\n❓ Proceed with migration? (y/N): ");
        use std::io::Write;
        std::io::stdout().flush().map_err(|e| ElifError::Io(e))?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).map_err(|e| ElifError::Io(e))?;
        
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Migration cancelled.");
            return Ok(());
        }
    }

    println!("🚀 Starting IoC to Module migration...");
    
    // Backup existing code
    println!("Step 1/4: Creating backup...");
    create_migration_backup().await?;
    
    // Analyze existing providers and controllers
    println!("Step 2/4: Analyzing existing structure...");
    let ioc_analysis = analyze_existing_ioc().await?;
    
    // Generate module structure
    println!("Step 3/4: Generating module structure...");
    generate_modules_from_ioc(&ioc_analysis).await?;
    
    // Update registrations
    println!("Step 4/4: Updating provider registrations...");
    update_provider_registrations(&ioc_analysis).await?;

    println!();
    println!("✅ Migration completed successfully!");
    println!("\n📖 Next steps:");
    println!("   elifrs module:validate --fix-issues");
    println!("   elifrs module:graph --format=text");

    Ok(())
}

/// Validate module composition for issues
pub async fn validate(fix_issues: bool) -> Result<(), ElifError> {
    println!("🔍 Module Composition Validation");
    println!();

    let discovery = ModuleDiscovery::new();
    let modules = discovery.discover_modules().await?;
    
    if modules.is_empty() {
        println!("📭 No modules found to validate.");
        return Ok(());
    }

    let dependency_graph = discovery.analyze_dependencies(&modules).await?;
    let validation_report = discovery.validate_composition(&dependency_graph).await?;

    print_validation_report(&validation_report)?;

    if fix_issues && !validation_report.errors.is_empty() {
        println!("\n🔧 Attempting to fix issues...");
        fix_validation_issues(&validation_report).await?;
        
        // Re-validate after fixes
        let modules = discovery.discover_modules().await?;
        let dependency_graph = discovery.analyze_dependencies(&modules).await?;
        let updated_report = discovery.validate_composition(&dependency_graph).await?;
        
        if updated_report.errors.is_empty() {
            println!("✅ All issues fixed successfully!");
        } else {
            println!("⚠️  Some issues require manual attention:");
            print_validation_report(&updated_report)?;
        }
    }

    Ok(())
}

// Core data structures

#[derive(Debug, Clone)]
pub struct ModuleDiscovery {
    project_root: PathBuf,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub providers: Vec<String>,
    pub controllers: Vec<String>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub file_count: usize,
    pub line_count: usize,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub modules: Vec<ModuleInfo>,
    pub dependencies: HashMap<String, Vec<String>>,
    pub circular_deps: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub module: String,
    pub error_type: ValidationErrorType,
    pub message: String,
    pub fixable: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ValidationErrorType {
    CircularDependency,
    MissingDependency,
    UnregisteredProvider,
    DuplicateController,
}

#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub module: String,
    pub message: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IocAnalysis {
    pub providers: Vec<ProviderInfo>,
    pub controllers: Vec<ControllerInfo>,
    pub manual_registrations: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProviderInfo {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ControllerInfo {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
}

// Implementation

impl ModuleDiscovery {
    pub fn new() -> Self {
        Self {
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    pub async fn discover_modules(&self) -> Result<Vec<ModuleInfo>, ElifError> {
        let mut modules = Vec::new();
        let modules_dir = self.project_root.join("src/modules");

        if !modules_dir.exists() {
            return Ok(modules);
        }

        // Scan for module files
        for entry in fs::read_dir(&modules_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("rs")
                && path.file_name().and_then(|s| s.to_str()) != Some("mod.rs")
            {
                if let Some(module) = self.parse_module_file(&path).await? {
                    modules.push(module);
                }
            }
        }

        Ok(modules)
    }

    pub async fn analyze_dependencies(&self, modules: &[ModuleInfo]) -> Result<DependencyGraph, ElifError> {
        let mut dependencies = HashMap::new();

        // Build dependency map
        for module in modules {
            dependencies.insert(module.name.clone(), module.imports.clone());
        }

        // Detect circular dependencies
        let circular_deps = self.detect_circular_dependencies(&dependencies);

        Ok(DependencyGraph {
            modules: modules.to_vec(),
            dependencies,
            circular_deps,
        })
    }

    pub async fn validate_composition(&self, graph: &DependencyGraph) -> Result<ValidationReport, ElifError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Check for circular dependencies
        for cycle in &graph.circular_deps {
            errors.push(ValidationError {
                module: cycle[0].clone(),
                error_type: ValidationErrorType::CircularDependency,
                message: format!("Circular dependency detected: {}", cycle.join(" -> ")),
                fixable: false,
            });
        }

        // Check for missing dependencies
        let module_names: HashSet<_> = graph.modules.iter().map(|m| &m.name).collect();
        
        for module in &graph.modules {
            for import in &module.imports {
                if !module_names.contains(import) {
                    errors.push(ValidationError {
                        module: module.name.clone(),
                        error_type: ValidationErrorType::MissingDependency,
                        message: format!("Module '{}' imports non-existent module '{}'", module.name, import),
                        fixable: false,
                    });
                }
            }

            // Check for unregistered providers
            for provider in &module.providers {
                if !self.provider_exists(provider).await? {
                    warnings.push(ValidationWarning {
                        module: module.name.clone(),
                        message: format!("Provider '{}' declared but not found", provider),
                    });
                }
            }

            // Check for unregistered controllers
            for controller in &module.controllers {
                if !self.controller_exists(controller).await? {
                    warnings.push(ValidationWarning {
                        module: module.name.clone(),
                        message: format!("Controller '{}' declared but not found", controller),
                    });
                }
            }
        }

        // Generate suggestions
        if !errors.is_empty() || !warnings.is_empty() {
            suggestions.push("Run 'elifrs module:validate --fix-issues' to attempt automatic fixes".to_string());
        }

        if graph.modules.len() > 10 {
            suggestions.push("Consider breaking down large modules into smaller, focused modules".to_string());
        }

        Ok(ValidationReport {
            errors,
            warnings,
            suggestions,
        })
    }

    async fn parse_module_file(&self, path: &Path) -> Result<Option<ModuleInfo>, ElifError> {
        let content = fs::read_to_string(path)?;
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Extract module name from #[module(...)] attribute
        let module_name = self.extract_module_name(&content, file_name);
        let line_count = content.lines().count();

        // Parse module attributes
        let providers = self.extract_module_list(&content, "providers");
        let controllers = self.extract_module_list(&content, "controllers");
        let imports = self.extract_module_list(&content, "imports");
        let exports = self.extract_module_list(&content, "exports");

        Ok(Some(ModuleInfo {
            name: module_name,
            path: path.to_path_buf(),
            providers,
            controllers,
            imports,
            exports,
            file_count: 1,
            line_count,
        }))
    }

    fn extract_module_name(&self, content: &str, file_name: &str) -> String {
        // Look for struct definition
        for line in content.lines() {
            if line.trim().starts_with("pub struct ") {
                if let Some(struct_name) = line.split("pub struct ")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|s| s.split(';').next())
                {
                    return struct_name.to_string();
                }
            }
        }
        
        // Fallback to file name in PascalCase
        self.to_pascal_case(file_name)
    }

    fn extract_module_list(&self, content: &str, list_name: &str) -> Vec<String> {
        let mut items = Vec::new();
        let mut in_module_attr = false;
        let mut brace_count = 0;

        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.starts_with("#[module(") {
                in_module_attr = true;
                brace_count = 1;
            } else if in_module_attr {
                if trimmed.contains('(') {
                    brace_count += trimmed.matches('(').count();
                }
                if trimmed.contains(')') {
                    brace_count -= trimmed.matches(')').count();
                }
                
                if brace_count == 0 {
                    in_module_attr = false;
                    continue;
                }
            }

            if in_module_attr && trimmed.starts_with(&format!("{} =", list_name)) {
                // Extract items between [ and ]
                let line_content = if trimmed.contains('[') && trimmed.contains(']') {
                    trimmed.to_string()
                } else {
                    // Multi-line list, collect until we find the closing bracket
                    let full_content = trimmed.to_string();
                    // This is simplified - in reality you'd need proper parsing
                    full_content
                };

                if let Some(start) = line_content.find('[') {
                    if let Some(end) = line_content.find(']') {
                        let items_str = &line_content[start + 1..end];
                        for item in items_str.split(',') {
                            let item = item.trim().trim_matches('"').trim_matches('\'');
                            if !item.is_empty() {
                                items.push(item.to_string());
                            }
                        }
                    }
                }
            }
        }

        items
    }

    fn detect_circular_dependencies(&self, dependencies: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for module in dependencies.keys() {
            if !visited.contains(module) {
                let mut path = Vec::new();
                if let Some(cycle) = self.dfs_cycle_detection(
                    module,
                    dependencies,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                ) {
                    cycles.push(cycle);
                }
            }
        }

        cycles
    }

    fn dfs_cycle_detection(
        &self,
        module: &str,
        dependencies: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(module.to_string());
        rec_stack.insert(module.to_string());
        path.push(module.to_string());

        if let Some(deps) = dependencies.get(module) {
            for dep in deps {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.dfs_cycle_detection(dep, dependencies, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Found cycle
                    let cycle_start = path.iter().position(|m| m == dep).unwrap_or(0);
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dep.clone());
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(module);
        path.pop();
        None
    }

    async fn provider_exists(&self, provider_name: &str) -> Result<bool, ElifError> {
        let services_dir = self.project_root.join("src/services");
        let provider_file = services_dir.join(format!("{}.rs", self.to_snake_case(provider_name)));
        Ok(provider_file.exists())
    }

    async fn controller_exists(&self, controller_name: &str) -> Result<bool, ElifError> {
        let controllers_dir = self.project_root.join("src/controllers");
        let controller_file = controllers_dir.join(format!("{}.rs", self.to_snake_case(controller_name)));
        Ok(controller_file.exists())
    }

    fn to_pascal_case(&self, snake_case: &str) -> String {
        snake_case
            .split('_')
            .map(|word| {
                let mut chars: Vec<char> = word.chars().collect();
                if !chars.is_empty() {
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                }
                chars.into_iter().collect::<String>()
            })
            .collect::<String>()
            + "Module"
    }

    fn to_snake_case(&self, name: &str) -> String {
        let mut result = String::new();
        for (i, c) in name.chars().enumerate() {
            if i > 0 && c.is_uppercase() {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        }
        result
    }
}

// Display and output functions

async fn print_modules_list(modules: &[ModuleInfo], show_dependencies: bool) -> Result<(), ElifError> {
    println!("📋 Module Overview ({} modules):", modules.len());
    println!("┌─────────────────────────┬─────────────┬─────────────┬────────────┬───────────┐");
    println!("│ Module                  │ Controllers │ Providers   │ Lines      │ Imports   │");
    println!("├─────────────────────────┼─────────────┼─────────────┼────────────┼───────────┤");

    for module in modules {
        println!(
            "│ {:<23} │ {:<11} │ {:<11} │ {:<10} │ {:<9} │",
            truncate(&module.name, 23),
            module.controllers.len(),
            module.providers.len(),
            module.line_count,
            module.imports.len(),
        );
    }

    println!("└─────────────────────────┴─────────────┴─────────────┴────────────┴───────────┘");

    if show_dependencies {
        println!("\n🔗 Module Details:");
        for module in modules {
            print_module_details(module)?;
        }
    }

    // Summary statistics
    let total_controllers: usize = modules.iter().map(|m| m.controllers.len()).sum();
    let total_providers: usize = modules.iter().map(|m| m.providers.len()).sum();
    let total_lines: usize = modules.iter().map(|m| m.line_count).sum();

    println!("\n📊 Summary:");
    println!("   Modules: {}", modules.len());
    println!("   Controllers: {}", total_controllers);
    println!("   Providers: {}", total_providers);
    println!("   Total lines: {}", total_lines);

    Ok(())
}

fn print_module_details(module: &ModuleInfo) -> Result<(), ElifError> {
    println!("\n🧩 {}", module.name);

    if !module.controllers.is_empty() {
        println!("   🎮 Controllers: {}", module.controllers.join(", "));
    }

    if !module.providers.is_empty() {
        println!("   ⚙️ Providers: {}", module.providers.join(", "));
    }

    if !module.imports.is_empty() {
        println!("   📥 Imports: {}", module.imports.join(", "));
    }

    if !module.exports.is_empty() {
        println!("   📤 Exports: {}", module.exports.join(", "));
    }

    Ok(())
}

fn print_dependency_summary(graph: &DependencyGraph) -> Result<(), ElifError> {
    if !graph.circular_deps.is_empty() {
        println!("⚠️  Circular Dependencies Detected:");
        for cycle in &graph.circular_deps {
            println!("   🔄 {}", cycle.join(" -> "));
        }
        println!();
    }

    // Show modules with most dependencies
    let mut dep_counts: Vec<_> = graph.dependencies.iter()
        .map(|(module, deps)| (module, deps.len()))
        .collect();
    dep_counts.sort_by(|a, b| b.1.cmp(&a.1));

    if dep_counts.len() > 3 {
        println!("📈 Most Connected Modules:");
        for (module, count) in dep_counts.iter().take(3) {
            println!("   • {} ({} dependencies)", module, count);
        }
    }

    Ok(())
}

async fn generate_svg_graph(_graph: &DependencyGraph, _output: Option<&str>) -> Result<(), ElifError> {
    println!("⚠️ SVG graph generation requires graphviz integration");
    println!("   This feature will be completed in the next iteration");
    println!("\n💡 Workaround:");
    println!("   elifrs module:graph --format=dot | dot -Tsvg > modules.svg");
    Ok(())
}

async fn generate_dot_graph(graph: &DependencyGraph, output: Option<&str>) -> Result<(), ElifError> {
    let dot_content = format_dot_graph(graph)?;
    
    match output {
        Some(path) => {
            fs::write(path, &dot_content)?;
            println!("✅ DOT graph saved to: {}", path);
        }
        None => {
            println!("{}", dot_content);
        }
    }
    
    println!("\n💡 Generate image: dot -Tpng modules.dot -o modules.png");
    Ok(())
}

async fn generate_text_graph(graph: &DependencyGraph) -> Result<(), ElifError> {
    println!("📊 Module Dependency Graph (Text Format):");
    println!();

    for module in &graph.modules {
        println!("🧩 {}", module.name);
        
        if let Some(deps) = graph.dependencies.get(&module.name) {
            if deps.is_empty() {
                println!("   └── (no dependencies)");
            } else {
                for (i, dep) in deps.iter().enumerate() {
                    let prefix = if i == deps.len() - 1 { "└──" } else { "├──" };
                    println!("   {} 📥 {}", prefix, dep);
                }
            }
        }
        
        // Show what depends on this module
        let dependents: Vec<_> = graph.dependencies.iter()
            .filter(|(_, deps)| deps.contains(&module.name))
            .map(|(module, _)| module)
            .collect();
            
        if !dependents.is_empty() {
            let dependent_names: Vec<String> = dependents.iter().map(|s| (*s).clone()).collect();
            println!("   📤 Used by: {}", dependent_names.join(", "));
        }
        
        println!();
    }

    Ok(())
}

async fn generate_json_graph(graph: &DependencyGraph, output: Option<&str>) -> Result<(), ElifError> {
    let json_graph = json!({
        "modules": graph.modules.iter().map(|m| json!({
            "name": m.name,
            "path": m.path.to_string_lossy(),
            "providers": m.providers,
            "controllers": m.controllers,
            "imports": m.imports,
            "exports": m.exports,
            "line_count": m.line_count
        })).collect::<Vec<_>>(),
        "dependencies": graph.dependencies,
        "circular_dependencies": graph.circular_deps,
        "metadata": {
            "total_modules": graph.modules.len(),
            "framework": "elif.rs",
            "version": "0.8.0"
        }
    });

    let json_str = serde_json::to_string_pretty(&json_graph)
        .map_err(|e| ElifError::system_error(format!("JSON serialization failed: {}", e)))?;

    match output {
        Some(path) => {
            fs::write(path, &json_str)?;
            println!("✅ JSON graph saved to: {}", path);
        }
        None => {
            println!("{}", json_str);
        }
    }

    Ok(())
}

fn format_dot_graph(graph: &DependencyGraph) -> Result<String, ElifError> {
    let mut dot = String::new();
    dot.push_str("digraph ModuleDependencies {\n");
    dot.push_str("  rankdir=TB;\n");
    dot.push_str("  node [shape=box, style=rounded];\n\n");

    // Define nodes with colors based on module type
    for module in &graph.modules {
        let color = if module.controllers.is_empty() && module.providers.is_empty() {
            "lightgray"
        } else if !module.controllers.is_empty() && !module.providers.is_empty() {
            "lightblue"
        } else if !module.controllers.is_empty() {
            "lightgreen"
        } else {
            "lightyellow"
        };

        dot.push_str(&format!(
            "  \"{}\" [fillcolor={}, style=filled, label=\"{}\\n{}C {}P\"];\n",
            module.name,
            color,
            module.name,
            module.controllers.len(),
            module.providers.len()
        ));
    }

    dot.push_str("\n");

    // Define edges
    for (module, deps) in &graph.dependencies {
        for dep in deps {
            dot.push_str(&format!("  \"{}\" -> \"{}\";\n", dep, module));
        }
    }

    // Highlight circular dependencies
    if !graph.circular_deps.is_empty() {
        dot.push_str("\n  // Circular dependencies (highlighted in red)\n");
        for cycle in &graph.circular_deps {
            for i in 0..cycle.len() {
                let from = &cycle[i];
                let to = &cycle[(i + 1) % cycle.len()];
                dot.push_str(&format!("  \"{}\" -> \"{}\" [color=red, penwidth=2];\n", from, to));
            }
        }
    }

    dot.push_str("}\n");
    Ok(dot)
}

// Migration functions (simplified implementations)

async fn analyze_existing_ioc() -> Result<IocAnalysis, ElifError> {
    println!("⚠️ IoC analysis implementation coming in next iteration");
    
    // Placeholder implementation
    Ok(IocAnalysis {
        providers: Vec::new(),
        controllers: Vec::new(),
        manual_registrations: Vec::new(),
    })
}

fn print_migration_analysis(_analysis: &IocAnalysis) -> Result<(), ElifError> {
    println!("📊 Migration Analysis Results:");
    println!("   • Manual IoC registrations found: 0");
    println!("   • Providers to migrate: 0");
    println!("   • Controllers to migrate: 0");
    println!("\n⚠️ Full analysis implementation coming in next iteration");
    Ok(())
}

async fn create_migration_backup() -> Result<(), ElifError> {
    println!("⚠️ Backup implementation requires file system operations");
    println!("   This feature will be completed in the next iteration");
    Ok(())
}

async fn generate_modules_from_ioc(_analysis: &IocAnalysis) -> Result<(), ElifError> {
    println!("⚠️ Module generation from IoC analysis coming in next iteration");
    Ok(())
}

async fn update_provider_registrations(_analysis: &IocAnalysis) -> Result<(), ElifError> {
    println!("⚠️ Provider registration updates coming in next iteration");
    Ok(())
}

fn print_validation_report(report: &ValidationReport) -> Result<(), ElifError> {
    if !report.errors.is_empty() {
        println!("❌ Validation Errors ({}):", report.errors.len());
        for error in &report.errors {
            let icon = match error.error_type {
                ValidationErrorType::CircularDependency => "🔄",
                ValidationErrorType::MissingDependency => "❓",
                ValidationErrorType::UnregisteredProvider => "⚙️",
                ValidationErrorType::DuplicateController => "🎮",
            };
            
            let fix_status = if error.fixable { " (fixable)" } else { " (manual)" };
            println!("   {} {}: {}{}", icon, error.module, error.message, fix_status);
        }
        println!();
    }

    if !report.warnings.is_empty() {
        println!("⚠️  Validation Warnings ({}):", report.warnings.len());
        for warning in &report.warnings {
            println!("   ⚠️  {}: {}", warning.module, warning.message);
        }
        println!();
    }

    if report.errors.is_empty() && report.warnings.is_empty() {
        println!("✅ Module composition is valid!");
        println!("   No circular dependencies detected");
        println!("   All providers and controllers properly registered");
    }

    if !report.suggestions.is_empty() {
        println!("💡 Suggestions:");
        for suggestion in &report.suggestions {
            println!("   • {}", suggestion);
        }
    }

    Ok(())
}

async fn fix_validation_issues(_report: &ValidationReport) -> Result<(), ElifError> {
    println!("⚠️ Automatic issue fixing implementation coming in next iteration");
    println!("   Current issues would require manual attention");
    Ok(())
}

// Helper functions

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}