//! Debug and visualization tools for the module system
//! 
//! Provides utilities for debugging module compositions, visualizing dependency graphs,
//! and analyzing potential issues in module configurations.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Type, Error};

/// Generate debug information for module composition
#[allow(dead_code)]
pub fn generate_debug_info(modules: &[&Type]) -> Result<TokenStream, Error> {
    let module_names: Vec<_> = modules.iter().map(|m| {
        quote! { stringify!(#m) }
    }).collect();
    
    let debug_info = quote! {
        {
            use std::collections::HashMap;
            
            // Debug information structure
            #[derive(Debug)]
            pub struct ModuleDebugInfo {
                pub modules: Vec<&'static str>,
                pub dependency_graph: HashMap<&'static str, Vec<&'static str>>,
                pub potential_issues: Vec<String>,
            }
            
            let debug_info = ModuleDebugInfo {
                modules: vec![#(#module_names),*],
                dependency_graph: HashMap::new(), // Will be populated by runtime analysis
                potential_issues: Vec::new(),
            };
            
            // Print debug information
            println!("🔍 Module Debug Information");
            println!("==========================");
            println!("Modules: {:?}", debug_info.modules);
            println!("Total modules: {}", debug_info.modules.len());
            
            if debug_info.modules.is_empty() {
                println!("⚠️  Warning: No modules found in composition");
            }
            
            debug_info
        }
    };
    
    Ok(debug_info)
}

/// Generate dependency visualization code
pub fn generate_dependency_graph_visualization() -> TokenStream {
    quote! {
        /// Visualize module dependency graph as ASCII art
        pub fn visualize_dependency_graph(modules: &[&str]) {
            println!("🌳 Module Dependency Graph");
            println!("==========================");
            
            for (i, module) in modules.iter().enumerate() {
                if i == modules.len() - 1 {
                    println!("└── {}", module);
                } else {
                    println!("├── {}", module);
                }
            }
            
            if modules.is_empty() {
                println!("(No modules found)");
            }
        }
        
        /// Generate DOT notation for dependency graph visualization
        pub fn generate_dot_graph(modules: &[&str]) -> String {
            let mut dot = String::from("digraph ModuleDependencies {\n");
            dot.push_str("    rankdir=LR;\n");
            dot.push_str("    node [shape=box, style=rounded];\n\n");
            
            for module in modules {
                dot.push_str(&format!("    \"{}\";\n", module));
            }
            
            // TODO: Add actual dependency edges when runtime analysis is available
            // For now, just show modules as isolated nodes
            
            dot.push_str("}\n");
            dot
        }
        
        /// Analyze potential module composition issues
        pub fn analyze_composition_issues(modules: &[&str]) -> Vec<String> {
            let mut issues = Vec::new();
            
            if modules.is_empty() {
                issues.push("No modules defined in composition".to_string());
            }
            
            if modules.len() > 50 {
                issues.push(format!("Large number of modules ({}): consider grouping related modules", modules.len()));
            }
            
            // Check for potential naming conflicts
            let mut seen = std::collections::HashSet::new();
            for module in modules {
                if !seen.insert(module) {
                    issues.push(format!("Duplicate module: {}", module));
                }
            }
            
            issues
        }
    }
}

/// Generate compile-time module analysis macros
pub fn generate_analysis_macros() -> TokenStream {
    quote! {
        /// Compile-time module analysis macro
        /// 
        /// Usage: analyze_modules!(UserModule, AuthModule, PostModule);
        #[macro_export]
        macro_rules! analyze_modules {
            ($($module:ty),* $(,)?) => {{
                let modules = vec![$(stringify!($module)),*];
                
                println!("📊 Module Analysis Report");
                println!("=========================");
                println!("Analyzing {} modules:", modules.len());
                
                for (i, module) in modules.iter().enumerate() {
                    println!("  {}. {}", i + 1, module);
                }
                
                // Analyze potential issues
                let issues = analyze_composition_issues(&modules);
                if !issues.is_empty() {
                    println!("\n⚠️  Potential Issues:");
                    for issue in issues {
                        println!("  • {}", issue);
                    }
                } else {
                    println!("\n✅ No issues detected");
                }
                
                // Generate visualization
                println!();
                visualize_dependency_graph(&modules);
                
                // Generate DOT graph for external tools
                println!("\n🎨 DOT Graph (for Graphviz):");
                println!("{}", generate_dot_graph(&modules));
                
                modules
            }};
        }
        
        /// Debug print macro for module composition
        #[macro_export]  
        macro_rules! debug_composition {
            ($composition:expr) => {{
                println!("🐛 Debug Module Composition");
                println!("===========================");
                println!("Composition: {}", stringify!($composition));
                $composition
            }};
        }
    }
}

/// Generate module health check utilities
pub fn generate_health_check_utils() -> TokenStream {
    quote! {
        /// Module composition health check
        pub struct ModuleHealthCheck {
            pub total_modules: usize,
            pub circular_dependencies: Vec<String>,
            pub missing_exports: Vec<String>,
            pub unused_imports: Vec<String>,
            pub health_score: f32,
        }
        
        impl ModuleHealthCheck {
            pub fn new() -> Self {
                Self {
                    total_modules: 0,
                    circular_dependencies: Vec::new(),
                    missing_exports: Vec::new(), 
                    unused_imports: Vec::new(),
                    health_score: 100.0,
                }
            }
            
            pub fn check_modules(&mut self, modules: &[&str]) {
                self.total_modules = modules.len();
                
                // Basic health scoring
                if self.total_modules == 0 {
                    self.health_score = 0.0;
                } else if self.total_modules > 100 {
                    self.health_score -= 10.0; // Penalty for too many modules
                }
                
                // TODO: Add actual dependency analysis when runtime support is available
            }
            
            pub fn report(&self) {
                println!("🏥 Module Health Report");
                println!("=======================");
                println!("Total modules: {}", self.total_modules);
                println!("Health score: {:.1}/100", self.health_score);
                
                if !self.circular_dependencies.is_empty() {
                    println!("🔄 Circular dependencies:");
                    for dep in &self.circular_dependencies {
                        println!("  • {}", dep);
                    }
                }
                
                if !self.missing_exports.is_empty() {
                    println!("❌ Missing exports:");
                    for export in &self.missing_exports {
                        println!("  • {}", export);
                    }
                }
                
                if !self.unused_imports.is_empty() {
                    println!("⚠️  Unused imports:");
                    for import in &self.unused_imports {
                        println!("  • {}", import);
                    }
                }
                
                if self.health_score >= 90.0 {
                    println!("✅ Module composition is healthy!");
                } else if self.health_score >= 70.0 {
                    println!("⚠️  Module composition has minor issues");
                } else {
                    println!("❌ Module composition needs attention");
                }
            }
        }
    }
}