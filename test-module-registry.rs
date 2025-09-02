// Simple test to verify the module registry system works
use elif_http_derive::module;
use elif_core::modules::{get_global_module_registry, CompileTimeModuleRegistry};

#[derive(Default)]
pub struct UserService;

#[derive(Default)]
pub struct UserController;

#[derive(Default)]
pub struct AuthService;

#[derive(Default)]
pub struct AuthController;

#[module(
    providers: [UserService],
    controllers: [UserController]
)]
pub struct UserModule;

#[module(
    providers: [AuthService],
    controllers: [AuthController],
    exports: [AuthService]
)]
pub struct AuthModule;

#[module(
    imports: [UserModule, AuthModule]
)]
pub struct AppModule;

fn main() {
    println!("Testing module registry system...");

    // Get the global registry
    let registry = get_global_module_registry();
    
    println!("Found {} modules:", registry.module_count());
    
    for module in registry.all_modules() {
        println!("Module: {}", module.name);
        println!("  Controllers: {:?}", module.controllers);
        println!("  Providers: {:?}", module.providers);
        println!("  Imports: {:?}", module.imports);
        println!("  Exports: {:?}", module.exports);
        println!();
    }
    
    // Test dependency resolution
    match registry.resolve_dependency_order() {
        Ok(ordered_modules) => {
            println!("Dependency resolution successful!");
            println!("Load order:");
            for (i, module) in ordered_modules.iter().enumerate() {
                println!("  {}. {}", i + 1, module.name);
            }
        }
        Err(e) => {
            println!("Dependency resolution failed: {}", e);
        }
    }
}