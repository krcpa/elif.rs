//! UI test for Epic 3 module composition - should compile successfully

use elif_http_derive::{module, module_composition};

/// Test service
#[derive(Default)]
pub struct TestService;

/// Module A
#[module(
    providers: [TestService],
    controllers: [],
    imports: [],
    exports: [TestService]
)]
pub struct ModuleA;

/// Module B  
#[module(
    providers: [TestService @ "module_b"],
    controllers: [],
    imports: [],
    exports: [TestService]
)]
pub struct ModuleB;

fn main() {
    // Test module composition
    let composition_result = module_composition! {
        modules: [ModuleA, ModuleB],
        overrides: [
            TestService @ "override"
        ]
    };
    
    assert_eq!(composition_result.name, "ComposedApplication");
    println!("Epic 3 module composition works correctly!");
}