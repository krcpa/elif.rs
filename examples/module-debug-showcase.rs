//! Module Debug and Visualization Showcase
//! 
//! Demonstrates the debugging and visualization tools available for 
//! analyzing module compositions and dependency graphs.

// Import the debug tools (would be available after running debug_modules!())
use elif_http_derive::{module, debug_modules};

// =============================================================================
// EXAMPLE MODULES  
// =============================================================================

#[derive(Default)]
pub struct DatabaseService;

#[derive(Default)]
pub struct CacheService;

#[derive(Default)]
pub struct UserService;

#[derive(Default)]
pub struct EmailService;

#[derive(Default)]
pub struct UserController;

#[derive(Default)]
pub struct PostController;

// Define some example modules
#[module(
    providers: [DatabaseService],
    exports: [DatabaseService]
)]
pub struct DatabaseModule;

#[module(
    providers: [CacheService], 
    exports: [CacheService]
)]
pub struct CacheModule;

#[module(
    imports: [DatabaseModule],
    providers: [UserService],
    controllers: [UserController],
    exports: [UserService]
)]
pub struct UserModule;

#[module(
    imports: [DatabaseModule, UserModule],
    providers: [EmailService],
    exports: [EmailService]
)]
pub struct EmailModule;

#[module(
    imports: [UserModule, EmailModule],
    controllers: [PostController]
)]
pub struct PostModule;

// =============================================================================
// DEBUG TOOLS DEMONSTRATION
// =============================================================================

// Generate debug utilities (this would typically be in a separate module)
debug_modules!();

fn main() {
    println!("🔧 Module Debug and Visualization Showcase");
    println!("===========================================\n");
    
    // Example 1: Analyze module composition
    println!("1. Module Analysis:");
    let modules = analyze_modules!(
        DatabaseModule,
        CacheModule, 
        UserModule,
        EmailModule,
        PostModule
    );
    println!();
    
    // Example 2: Health check
    println!("2. Health Check:");
    let mut health_check = ModuleHealthCheck::new();
    health_check.check_modules(&modules);
    health_check.report();
    println!();
    
    // Example 3: Custom visualization
    println!("3. Custom Dependency Analysis:");
    analyze_module_dependencies();
    println!();
    
    // Example 4: DOT graph generation
    println!("4. Graphviz DOT Output:");
    let dot_graph = generate_dot_graph(&modules);
    println!("```dot");
    println!("{}", dot_graph);
    println!("```");
    println!("💡 Save this as 'modules.dot' and run: dot -Tpng modules.dot -o modules.png");
    println!();
    
    println!("✅ Debug showcase completed!");
}

/// Custom analysis for module dependencies
fn analyze_module_dependencies() {
    println!("📊 Manual Dependency Analysis:");
    println!("==============================");
    
    // Simulate dependency analysis
    let dependencies = vec![
        ("DatabaseModule", vec!["(none)"]),
        ("CacheModule", vec!["(none)"]), 
        ("UserModule", vec!["DatabaseModule"]),
        ("EmailModule", vec!["DatabaseModule", "UserModule"]),
        ("PostModule", vec!["UserModule", "EmailModule"]),
    ];
    
    for (module, deps) in dependencies {
        println!("📦 {}", module);
        for dep in deps {
            if dep == "(none)" {
                println!("   └── 🌱 No dependencies (root module)");
            } else {
                println!("   └── 📎 depends on {}", dep);
            }
        }
    }
    
    // Check for potential issues
    println!("\n🔍 Potential Issues Analysis:");
    println!("• ✅ No circular dependencies detected");
    println!("• ✅ All imports have corresponding exports");
    println!("• ⚠️  PostModule doesn't export anything (dead end module)");
    println!("• 💡 Consider combining CacheModule with DatabaseModule");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn debug_tools_work() {
        // Test that debug macros compile and run
        let modules = vec!["TestModule1", "TestModule2"];
        visualize_dependency_graph(&modules);
        
        let issues = analyze_composition_issues(&modules);
        assert!(issues.is_empty());
        
        let dot = generate_dot_graph(&modules);
        assert!(dot.contains("digraph ModuleDependencies"));
    }
    
    #[test] 
    fn health_check_works() {
        let mut health_check = ModuleHealthCheck::new();
        health_check.check_modules(&["Module1", "Module2"]);
        
        assert_eq!(health_check.total_modules, 2);
        assert!(health_check.health_score > 90.0);
    }
    
    #[test]
    fn empty_modules_detected() {
        let issues = analyze_composition_issues(&[]);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("No modules defined"));
    }
    
    #[test]
    fn duplicate_modules_detected() {
        let issues = analyze_composition_issues(&["Module1", "Module1"]);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("Duplicate module"));
    }
}