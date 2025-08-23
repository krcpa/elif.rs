//! UI test for Epic 3 module descriptor generation - should compile successfully

use elif_http_derive::module;

/// Test service
#[derive(Default)]
pub struct UserService {
    pub id: u32,
}

/// Test trait
pub trait EmailService: Send + Sync {
    fn send_email(&self, to: &str) -> Result<(), String>;
}

/// Test implementation
#[derive(Default)]
pub struct SmtpEmailService {
    pub host: String,
}

impl EmailService for SmtpEmailService {
    fn send_email(&self, _to: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Test controller
#[derive(Default)]
pub struct UserController {
    pub path: String,
}

/// Module with comprehensive Epic 3 features
#[module(
    providers: [
        UserService,
        EmailService => SmtpEmailService,
        UserService @ "named_user",
        EmailService => SmtpEmailService @ "smtp"
    ],
    controllers: [UserController],
    imports: [],
    exports: [UserService, EmailService]
)]
pub struct ComprehensiveModule;

/// Database module for import testing  
#[module(
    providers: [UserService @ "db_user"],
    controllers: [],
    imports: [],
    exports: [UserService]
)]
pub struct DatabaseModule;

/// Business module with imports
#[module(
    providers: [SmtpEmailService @ "business_email"],
    controllers: [UserController],
    imports: [DatabaseModule],
    exports: [SmtpEmailService]
)]
pub struct BusinessModule;

fn main() {
    // Test that generated descriptor method works
    let descriptor = ComprehensiveModule::module_descriptor();
    assert_eq!(descriptor.name, "ComprehensiveModule");
    
    // Test ModuleAutoConfiguration trait implementation
    use elif_core::modules::ModuleAutoConfiguration;
    let descriptor2 = <ComprehensiveModule as ModuleAutoConfiguration>::module_descriptor();
    assert_eq!(descriptor.name, descriptor2.name);
    
    // Test that provider information is captured
    assert_eq!(descriptor.service_count(), 4); // UserService + 3 EmailService variants
    assert_eq!(descriptor.controller_count(), 1);
    assert_eq!(descriptor.exports.len(), 2);
    
    println!("Epic 3 module descriptor generation works correctly!");
}